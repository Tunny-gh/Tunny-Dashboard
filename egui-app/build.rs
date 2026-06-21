use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/TunnyIcon.png");
    println!("cargo:rerun-if-changed=theory/");
    println!("cargo:rerun-if-changed=help-assets/");

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
    } else {
        println!("cargo:rustc-link-search=native={}", libs_dir.display());
        println!("cargo:rustc-link-lib=dylib=lib_lightgbm");
    }

    generate_help_html_files();
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

fn generate_help_html_files() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let katex_css = read_asset(&manifest_dir, "help-assets/katex.min.css");
    let katex_js = read_asset(&manifest_dir, "help-assets/katex.min.js");
    let auto_render_js = read_asset(&manifest_dir, "help-assets/auto-render.min.js");

    for lang in &["en", "ja"] {
        let theory_dir = manifest_dir
            .parent()
            .unwrap_or(&manifest_dir)
            .join("theory")
            .join(lang);

        if !theory_dir.exists() {
            continue;
        }

        let out_lang_dir = out_dir.join("help").join(lang);
        convert_dir(
            &theory_dir,
            &theory_dir,
            &out_lang_dir,
            &katex_css,
            &katex_js,
            &auto_render_js,
        );
    }
}

/// 依存クレートのライセンス情報を `cargo metadata` から収集し、
/// `OUT_DIR/licenses.rs` に `pub static LICENSES: &[LicenseEntry]` として生成する。
///
/// 生成物は `src/licenses.rs` から `include!` で取り込む。収集に失敗しても
/// ビルドは止めず、空配列を書き出してアプリがコンパイルできるようにする。
fn generate_license_data() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // 依存関係が変わったら再生成する。
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

/// `cargo metadata` を実行し、配布バイナリに含まれる外部クレートのライセンス
/// エントリを Rust 配列リテラルの本体（各行 `LicenseEntry { ... },`）として返す。
fn collect_license_entries(manifest_dir: &Path) -> Result<String, String> {
    use cargo_metadata::{DependencyKind, MetadataCommand};
    use std::collections::{BTreeMap, HashSet, VecDeque};

    let metadata = MetadataCommand::new()
        .manifest_path(manifest_dir.join("Cargo.toml"))
        .exec()
        .map_err(|e| e.to_string())?;

    // パッケージ id → Package の索引。
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

    // ルート（このクレート）を名前で特定する。ワークスペースでは resolve.root が None になりうる。
    let root_id = metadata
        .packages
        .iter()
        .find(|p| p.name == "tunny-desktop")
        .map(|p| p.id.clone())
        .ok_or_else(|| "tunny-desktop package not found".to_string())?;

    // ルートから Normal / Build 依存のみを辿り、配布物に含まれる閉包を求める
    // （dev-dependencies は実行バイナリに入らないので除外）。
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

    // ローカル/ワークスペースのクレート（source = None）は対象外。
    // 名前順に整列して安定した出力にする。
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

/// クレートディレクトリ直下の LICENSE / COPYING / NOTICE 等を読み、全文を連結して返す。
/// 複数ファイルはファイル名見出しを付けて区切る。
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
            continue; // バイナリ等は読み飛ばす
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

/// ライセンス全文を含むと推定されるファイル名か判定する（大文字小文字無視）。
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

fn read_asset(manifest_dir: &Path, relative: &str) -> String {
    let path = manifest_dir.join(relative);
    fs::read_to_string(&path).unwrap_or_default()
}

fn convert_dir(
    base: &Path,
    current: &Path,
    out_base: &Path,
    katex_css: &str,
    katex_js: &str,
    auto_render_js: &str,
) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            convert_dir(base, &path, out_base, katex_css, katex_js, auto_render_js);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let relative = path.strip_prefix(base).unwrap();
            let out_path = out_base.join(relative).with_extension("html");

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|e| panic!("failed to create {}: {e}", parent.display()));
            }

            let md_content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            let html =
                markdown_to_standalone_html(&md_content, katex_css, katex_js, auto_render_js);
            fs::write(&out_path, &html)
                .unwrap_or_else(|e| panic!("failed to write {}: {e}", out_path.display()));
        }
    }
}

fn markdown_to_html_body(md_content: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md_content, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn markdown_to_standalone_html(
    md_content: &str,
    katex_css: &str,
    katex_js: &str,
    auto_render_js: &str,
) -> String {
    let body = markdown_to_html_body(md_content);
    wrap_as_standalone_html(&body, katex_css, katex_js, auto_render_js)
}

fn wrap_as_standalone_html(
    body: &str,
    katex_css: &str,
    katex_js: &str,
    auto_render_js: &str,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
body {{
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 14px;
  line-height: 1.6;
  color: #4B5563;
  background: #ffffff;
  max-width: 860px;
  margin: 0 auto;
  padding: 24px;
}}
h1, h2, h3 {{
  font-weight: 800;
  margin-top: 1.5em;
  margin-bottom: 0.5em;
  color: #111827;
  letter-spacing: -0.025em;
}}
h1 {{ font-size: 1.8em; border-bottom: 1px solid #E5E7EB; padding-bottom: 0.3em; }}
h2 {{ font-size: 1.4em; border-bottom: 1px solid #E5E7EB; padding-bottom: 0.2em; }}
a {{ color: #2563EB; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
code {{ background: #F3F4F6; border-radius: 3px; padding: 0.1em 0.3em; font-size: 0.9em; }}
pre {{ background: #F3F4F6; border-radius: 6px; padding: 16px; overflow: auto; }}
pre code {{ background: none; padding: 0; }}
table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
th, td {{ border: 1px solid #E5E7EB; padding: 8px 12px; text-align: left; }}
th {{ background: #F3F4F6; font-weight: 600; color: #111827; }}
{katex_css}
</style>
</head>
<body>
{body}
<script>{katex_js}</script>
<script>{auto_render_js}</script>
<script>
if (typeof renderMathInElement === 'function') {{
  renderMathInElement(document.body, {{
    delimiters: [
      {{left: '$$', right: '$$', display: true}},
      {{left: '$', right: '$', display: false}}
    ]
  }});
}}
</script>
</body>
</html>"#,
        katex_css = katex_css,
        body = body,
        katex_js = katex_js,
        auto_render_js = auto_render_js,
    )
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
