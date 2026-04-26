use std::path::PathBuf;

/// アーティファクトモーダルの表示状態
#[derive(Default)]
pub struct ArtifactModalState {
    pub open: bool,
    pub trial_id: u32,
    pub files: Vec<PathBuf>,
    pub selected_file_idx: usize,
    pub csv_preview: Option<Vec<Vec<String>>>,
    pub image_texture: Option<egui::TextureHandle>,
}

/// アーティファクトプレビューモーダルを表示する
pub fn show_artifact_modal(ctx: &egui::Context, state: &mut ArtifactModalState) {
    if !state.open {
        return;
    }

    let title = format!("Artifacts — Trial #{}", state.trial_id);
    let files_count = state.files.len();
    let selected_idx = state.selected_file_idx;

    // ファイルタブのラベルを先に収集（borrow 解放のため）
    let file_names: Vec<String> = state
        .files
        .iter()
        .map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string()
        })
        .collect();
    let path_opt = state.files.get(selected_idx).cloned();

    let mut still_open = true;
    egui::Window::new(title)
        .open(&mut still_open)
        .resizable(true)
        .min_width(400.0)
        .show(ctx, |ui| {
            if files_count > 1 {
                ui.horizontal(|ui| {
                    for (i, name) in file_names.iter().enumerate() {
                        if ui.selectable_label(selected_idx == i, name).clicked() {
                            state.selected_file_idx = i;
                            state.csv_preview = None;
                            state.image_texture = None;
                        }
                    }
                });
                ui.separator();
            }

            if let Some(ref path) = path_opt {
                show_file_preview(ui, ctx, path, state);
            }
        });
    if !still_open {
        state.open = false;
    }
}

fn show_file_preview(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    path: &PathBuf,
    state: &mut ArtifactModalState,
) {
    use crate::io::artifacts::ArtifactFileType;
    match ArtifactFileType::from_path(path) {
        ArtifactFileType::Image => show_image_preview(ui, ctx, path, state),
        ArtifactFileType::Csv => show_csv_preview(ui, path, state),
        ArtifactFileType::Other => show_other_preview(ui, path),
    }
}

fn show_image_preview(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    path: &PathBuf,
    state: &mut ArtifactModalState,
) {
    if state.image_texture.is_none() {
        if let Ok(img_bytes) = std::fs::read(path) {
            if let Ok(img) = image::load_from_memory(&img_bytes) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let texture = ctx.load_texture(
                    "artifact_preview",
                    egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                    egui::TextureOptions::default(),
                );
                state.image_texture = Some(texture);
            }
        }
    }

    if let Some(texture) = &state.image_texture {
        let max_size = egui::Vec2::new(600.0, 400.0);
        ui.add(egui::Image::new(texture).fit_to_exact_size(max_size));
    } else {
        ui.label("Failed to load image");
    }
}

fn show_csv_preview(ui: &mut egui::Ui, path: &PathBuf, state: &mut ArtifactModalState) {
    if state.csv_preview.is_none() {
        if let Ok(content) = std::fs::read_to_string(path) {
            let rows: Vec<Vec<String>> = content
                .lines()
                .take(100) // REQ-007-F: 先頭100行
                .map(|line| line.split(',').map(|s| s.trim().to_string()).collect())
                .collect();
            state.csv_preview = Some(rows);
        }
    }

    if let Some(rows) = &state.csv_preview {
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("csv_preview_grid")
                .striped(true)
                .show(ui, |ui| {
                    for row in rows {
                        for cell in row {
                            ui.label(cell);
                        }
                        ui.end_row();
                    }
                });
        });
    } else {
        ui.label("Failed to load CSV");
    }
}

fn show_other_preview(ui: &mut egui::Ui, path: &PathBuf) {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let size_str = std::fs::metadata(path)
        .map(|m| format!("{} bytes", m.len()))
        .unwrap_or_else(|_| "unknown size".to_string());

    ui.label(format!("File: {filename}"));
    ui.label(format!("Size: {size_str}"));

    if ui.button("Open with OS").clicked() {
        let _ = open::that(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_modal_state_default() {
        let state = ArtifactModalState::default();
        assert!(!state.open);
        assert!(state.files.is_empty());
        assert!(state.csv_preview.is_none());
        assert!(state.image_texture.is_none());
    }

    #[test]
    fn csv_takes_at_most_100_rows() {
        let csv = (0..150)
            .map(|i| format!("{},{}", i, i * 2))
            .collect::<Vec<_>>()
            .join("\n");
        let rows: Vec<Vec<String>> = csv
            .lines()
            .take(100)
            .map(|line| line.split(',').map(|s| s.trim().to_string()).collect())
            .collect();
        assert!(rows.len() <= 100);
        assert_eq!(rows.len(), 100);
    }
}
