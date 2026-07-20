use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/TunnyIcon.png");

    // Propagate LightGBM link settings to the final binary.
    // rust_core's build.rs emits these for the rlib, but the linker flags
    // must also be visible when linking the top-level binary.
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let libs_dir = manifest_dir.parent().unwrap().join("libs");
    if cfg!(target_os = "macos") {
        // On macOS, rustc-link-lib flags are placed before rustc-link-search by
        // the linker driver, so ld never sees the search path in time. Instead,
        // emit everything via link-arg to guarantee -L precedes -l.
        println!("cargo:rustc-link-arg=-Wl,-L{}", libs_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-l_lightgbm");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libs_dir.display());
    } else if cfg!(target_os = "windows") {
        // Windows (MSVC): links lib_lightgbm.lib (import library from the DLL).
        // The literal file name is lib_lightgbm.lib, so the link name is `lib_lightgbm`.
        println!("cargo:rustc-link-search=native={}", libs_dir.display());
        println!("cargo:rustc-link-lib=dylib=lib_lightgbm");
    } else {
        // Linux: the shared object is `lib_lightgbm.so`. `rustc-link-lib=dylib=NAME`
        // links `libNAME.so`, so NAME must be `_lightgbm` (→ lib_lightgbm.so).
        // -rpath lets the produced binary locate the .so at runtime.
        println!("cargo:rustc-link-search=native={}", libs_dir.display());
        println!("cargo:rustc-link-lib=dylib=_lightgbm");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libs_dir.display());
    }

    generate_license_data();

    #[cfg(windows)]
    {
        let icon_path = build_windows_icon().expect("failed to generate Windows icon");
        let mut resources = winres::WindowsResource::new();
        resources.set_icon(icon_path.to_string_lossy().as_ref());
        resources
            .compile()
            .expect("failed to compile Windows resources");
    }
}

/// Collects license information for dependency crates from `cargo metadata` and
/// generates it into `OUT_DIR/licenses.rs` as `pub static LICENSES: &[LicenseEntry]`.
///
/// The generated file is pulled in via `include!` from `src/licenses.rs`. Even if
/// collection fails, the build is not aborted; an empty array is written out so the
/// app still compiles.
fn generate_license_data() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Regenerate whenever dependencies change.
    println!("cargo:rerun-if-changed=Cargo.toml");
    let lock = manifest_dir
        .parent()
        .unwrap_or(&manifest_dir)
        .join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());

    let body = collect_license_entries(&manifest_dir).unwrap_or_else(|e| {
        println!("cargo:warning=license metadata collection failed: {e}");
        String::new()
    });

    let src = format!("pub static LICENSES: &[LicenseEntry] = &[\n{body}];\n");
    let out_path = out_dir.join("licenses.rs");
    fs::write(&out_path, src)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", out_path.display()));
}

/// Runs `cargo metadata` and returns the license entries for external crates included
/// in the shipped binary, as the body of a Rust array literal (each line
/// `LicenseEntry { ... },`).
fn collect_license_entries(manifest_dir: &Path) -> Result<String, String> {
    use cargo_metadata::{DependencyKind, MetadataCommand};
    use std::collections::{BTreeMap, HashSet, VecDeque};

    let metadata = MetadataCommand::new()
        .manifest_path(manifest_dir.join("Cargo.toml"))
        .exec()
        .map_err(|e| e.to_string())?;

    // Index from package id -> Package.
    let pkg_by_id: BTreeMap<_, _> = metadata
        .packages
        .iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or_else(|| "no resolve graph in metadata".to_string())?;
    let node_by_id: BTreeMap<_, _> = resolve.nodes.iter().map(|n| (n.id.clone(), n)).collect();

    // Identify the root (this crate) by name. In a workspace, resolve.root can be None.
    let root_id = metadata
        .packages
        .iter()
        .find(|p| p.name == "tunny-desktop")
        .map(|p| p.id.clone())
        .ok_or_else(|| "tunny-desktop package not found".to_string())?;

    // Walk only Normal / Build dependencies from the root to compute the closure
    // included in the distributed artifact (dev-dependencies are excluded since they
    // don't end up in the executable binary).
    let mut reachable: HashSet<_> = HashSet::new();
    let mut queue: VecDeque<_> = VecDeque::new();
    queue.push_back(root_id.clone());
    reachable.insert(root_id.clone());
    while let Some(id) = queue.pop_front() {
        let Some(node) = node_by_id.get(&id) else {
            continue;
        };
        for dep in &node.deps {
            let ships = dep
                .dep_kinds
                .iter()
                .any(|k| matches!(k.kind, DependencyKind::Normal | DependencyKind::Build));
            if ships && reachable.insert(dep.pkg.clone()) {
                queue.push_back(dep.pkg.clone());
            }
        }
    }

    // Local/workspace crates (source = None) are excluded.
    // Sort by name for stable output.
    let mut entries: Vec<&cargo_metadata::Package> = reachable
        .iter()
        .filter_map(|id| pkg_by_id.get(id).copied())
        .filter(|p| p.source.is_some())
        .collect();
    entries.sort_by(|a, b| (a.name.as_str(), &a.version).cmp(&(b.name.as_str(), &b.version)));

    let mut body = String::new();
    for pkg in entries {
        let license = pkg.license.clone().unwrap_or_default();
        let repository = pkg.repository.clone().unwrap_or_default();
        let crate_dir = pkg.manifest_path.parent();
        let text = crate_dir
            .map(|d| read_license_text(d.as_std_path()))
            .unwrap_or_default();
        body.push_str(&format!(
            "    LicenseEntry {{ name: {:?}, version: {:?}, license: {:?}, repository: {:?}, text: {:?} }},\n",
            pkg.name,
            pkg.version.to_string(),
            license,
            repository,
            text,
        ));
    }
    Ok(body)
}

/// Reads LICENSE / COPYING / NOTICE etc. directly under the crate directory and
/// returns their concatenated full text. Multiple files are separated with a
/// filename heading.
fn read_license_text(crate_dir: &Path) -> String {
    let mut files: Vec<PathBuf> = match fs::read_dir(crate_dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_license_file_name(p))
            .collect(),
        Err(_) => return String::new(),
    };
    files.sort();

    let mut out = String::new();
    for f in files {
        let Ok(text) = fs::read_to_string(&f) else {
            continue; // skip binary files, etc.
        };
        let name = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!("===== {name} =====\n"));
        out.push_str(text.trim_end());
    }
    out
}

/// Determines whether a file name is likely to contain full license text
/// (case-insensitive).
fn is_license_file_name(path: &Path) -> bool {
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_uppercase()) else {
        return false;
    };
    [
        "LICENSE",
        "LICENCE",
        "COPYING",
        "COPYRIGHT",
        "NOTICE",
        "UNLICENSE",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

#[cfg(windows)]
fn build_windows_icon() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let png_path = manifest_dir.join("assets").join("TunnyIcon.png");
    let icon_path = out_dir.join("TunnyIcon.ico");

    let image = image::open(&png_path)?.into_rgba8();
    let (width, height) = image.dimensions();

    let icon_image = ico::IconImage::from_rgba_data(width, height, image.into_raw());
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);

    let mut icon_file = std::fs::File::create(&icon_path)?;
    icon_dir.write(&mut icon_file)?;

    Ok(icon_path)
}
