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
