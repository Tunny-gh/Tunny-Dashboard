use crate::ui::help::help_types::HelpModalState;

/// Show the help modal. Called from app.rs update loop.
/// Follows the same pattern as artifact_modal.
pub fn show_help_modal(ctx: &egui::Context, state: &mut HelpModalState) {
    if !state.open {
        return;
    }

    let item = match &state.item {
        Some(i) => i.clone(),
        None => {
            state.open = false;
            return;
        }
    };

    let content = crate::ui::help::help_content::get_help_content(&item);
    let title = format!("{} \u{2014} Help", content.title);

    let mut still_open = true;
    egui::Window::new(title)
        .open(&mut still_open)
        .resizable(true)
        .min_width(500.0)
        .default_width(650.0)
        .min_height(400.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            let active = state.active_tab.min(content.tabs.len().saturating_sub(1));

            // Tab bar
            ui.horizontal(|ui| {
                for (i, tab) in content.tabs.iter().enumerate() {
                    if ui.selectable_label(active == i, tab.label).clicked() {
                        state.active_tab = i;
                    }
                }
            });
            ui.separator();

            // Tab content
            if let Some(tab) = content.tabs.get(active) {
                crate::ui::help::md_renderer::render_markdown(ui, tab.markdown);
            }
        });

    if !still_open {
        state.open = false;
        state.active_tab = 0;
    }
}
