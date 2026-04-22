#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=assets/TunnyIcon.png");

    let icon_path = build_windows_icon().expect("failed to generate Windows icon");

    let mut resources = winres::WindowsResource::new();
    resources.set_icon(icon_path.to_string_lossy().as_ref());
    resources
        .compile()
        .expect("failed to compile Windows resources");
}

#[cfg(windows)]
fn build_windows_icon() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
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

#[cfg(not(windows))]
fn main() {
    println!("cargo:rerun-if-changed=assets/TunnyIcon.png");
}
