/// Lightweight Markdown → egui renderer.
///
/// Supports: H1–H3 headings, bold, inline code, bullet lists, tables, fenced
/// code blocks, and plain-text formulas.

// ---- public entry -----------------------------------------------------------

/// Render a Markdown string into an egui `ScrollArea`.
pub fn render_markdown(ui: &mut egui::Ui, markdown: &str) {
    egui::ScrollArea::vertical()
        .max_height(ui.available_height())
        .show(ui, |ui| {
            let mut in_code_block = false;
            for line in markdown.lines() {
                // --- fenced code blocks ---
                if line.trim_start().starts_with("```") {
                    in_code_block = !in_code_block;
                    ui.add_space(2.0);
                    continue;
                }
                if in_code_block {
                    ui.label(
                        egui::RichText::new(line)
                            .monospace()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(50, 50, 50)),
                    );
                    continue;
                }

                // --- blank line → paragraph spacing ---
                if line.trim().is_empty() {
                    ui.add_space(4.0);
                    continue;
                }

                render_line(ui, line);
            }
        });
}

// ---- line-level dispatch ----------------------------------------------------

fn render_line(ui: &mut egui::Ui, line: &str) {
    let trimmed = line.trim_start();

    // headings
    if let Some(rest) = trimmed.strip_prefix("# ") {
        ui.heading(rest);
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("## ") {
        ui.label(egui::RichText::new(rest).strong().size(16.0));
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("### ") {
        ui.label(egui::RichText::new(rest).strong().size(14.0));
        return;
    }

    // bullet / unordered list
    if let Some(rest) = trimmed.strip_prefix("- ") {
        ui.horizontal(|ui| {
            ui.label("  •");
            render_inline(ui, rest);
        });
        return;
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        ui.horizontal(|ui| {
            ui.label("  •");
            render_inline(ui, rest);
        });
        return;
    }

    // table separator (skip)
    if trimmed.starts_with('|') && trimmed.contains("---") {
        return;
    }

    // table row
    if trimmed.starts_with('|') && trimmed.ends_with('|') {
        render_table_row(ui, trimmed);
        return;
    }

    // normal paragraph
    render_inline(ui, trimmed);
}

// ---- inline formatting ------------------------------------------------------

fn render_inline(ui: &mut egui::Ui, text: &str) {
    ui.horizontal_wrapped(|ui| {
        let mut remainder = text;
        while !remainder.is_empty() {
            // check for **bold**
            if let Some(start) = remainder.find("**") {
                if start > 0 {
                    emit_plain_with_code(ui, &remainder[..start]);
                }
                let after = &remainder[start + 2..];
                if let Some(end) = after.find("**") {
                    ui.label(egui::RichText::new(&after[..end]).strong());
                    remainder = &after[end + 2..];
                } else {
                    ui.label(egui::RichText::new("**").strong());
                    remainder = after;
                }
            } else {
                emit_plain_with_code(ui, remainder);
                break;
            }
        }
    });
}

/// Emit plain text, handling `code` spans.
fn emit_plain_with_code(ui: &mut egui::Ui, text: &str) {
    let mut remainder = text;
    while !remainder.is_empty() {
        if let Some(start) = remainder.find('`') {
            if start > 0 {
                ui.label(&remainder[..start]);
            }
            let after = &remainder[start + 1..];
            if let Some(end) = after.find('`') {
                ui.label(
                    egui::RichText::new(&after[..end])
                        .monospace()
                        .color(egui::Color32::from_rgb(180, 60, 30)),
                );
                remainder = &after[end + 1..];
            } else {
                ui.label("`");
                remainder = after;
            }
        } else {
            ui.label(remainder);
            break;
        }
    }
}

// ---- table ------------------------------------------------------------------

fn render_table_row(ui: &mut egui::Ui, row: &str) {
    let cells: Vec<&str> = row
        .split('|')
        .filter(|c| !c.is_empty())
        .map(|c| c.trim())
        .collect();
    if cells.is_empty() {
        return;
    }
    let col_width = (ui.available_width() / cells.len() as f32).max(60.0);
    ui.horizontal(|ui| {
        for cell in &cells {
            ui.add_sized(
                egui::vec2(col_width, 0.0),
                egui::Label::new(
                    egui::RichText::new(*cell).size(12.0),
                )
                .wrap(),
            );
        }
    });
}

// ---- tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_h1_stripped() {
        let line = "# Title";
        let trimmed = line.trim_start();
        assert!(trimmed.starts_with("# "));
        assert_eq!(trimmed.strip_prefix("# "), Some("Title"));
    }

    #[test]
    fn heading_h2_stripped() {
        let line = "## Subtitle";
        let trimmed = line.trim_start();
        assert!(trimmed.starts_with("## "));
        assert_eq!(trimmed.strip_prefix("## "), Some("Subtitle"));
    }

    #[test]
    fn heading_h3_stripped() {
        let line = "### Section";
        let trimmed = line.trim_start();
        assert!(trimmed.starts_with("### "));
        assert_eq!(trimmed.strip_prefix("### "), Some("Section"));
    }

    #[test]
    fn bullet_dash_stripped() {
        let line = "- item text";
        let trimmed = line.trim_start();
        assert!(trimmed.starts_with("- "));
        assert_eq!(trimmed.strip_prefix("- "), Some("item text"));
    }

    #[test]
    fn bullet_star_stripped() {
        let line = "* item text";
        let trimmed = line.trim_start();
        assert!(trimmed.starts_with("* "));
        assert_eq!(trimmed.strip_prefix("* "), Some("item text"));
    }

    #[test]
    fn table_separator_detected() {
        let line = "| --- | --- |";
        let trimmed = line.trim_start();
        assert!(trimmed.starts_with('|') && trimmed.contains("---"));
    }

    #[test]
    fn table_row_parsed() {
        let row = "| col1 | col2 | col3 |";
        let cells: Vec<&str> = row.split('|').filter(|c| !c.is_empty()).map(|c| c.trim()).collect();
        assert_eq!(cells, vec!["col1", "col2", "col3"]);
    }

    #[test]
    fn code_block_toggle() {
        let line = "```rust";
        assert!(line.trim_start().starts_with("```"));
    }

    #[test]
    fn bold_detected() {
        let text = "**bold text**";
        assert!(text.contains("**"));
        let start = text.find("**").unwrap();
        let after = &text[start + 2..];
        let end = after.find("**").unwrap();
        assert_eq!(&after[..end], "bold text");
    }

    #[test]
    fn inline_code_detected() {
        let text = "use `code` here";
        let start = text.find('`').unwrap();
        let after = &text[start + 1..];
        let end = after.find('`').unwrap();
        assert_eq!(&after[..end], "code");
    }
}
