use crate::state::layout_state::PanelItem;
use crate::ui::help::help_content::{get_help_html, get_widget_name};
use crate::ui::help::help_types::HelpLanguage;
use std::path::PathBuf;

pub fn open_help(item: &PanelItem, lang: HelpLanguage) -> Result<(), String> {
    let html = get_help_html(item, lang);
    let path = build_temp_path(item, lang);
    write_html_to_temp(&path, html)?;
    open_in_browser(&path)
}

fn build_temp_path(item: &PanelItem, lang: HelpLanguage) -> PathBuf {
    let widget_name = get_widget_name(item);
    let lang_code = lang.code();
    std::env::temp_dir().join(format!("tunny-help-{widget_name}-{lang_code}.html"))
}

fn write_html_to_temp(path: &PathBuf, html: &str) -> Result<(), String> {
    std::fs::write(path, html.as_bytes())
        .map_err(|e| format!("一時ファイルの書き出しに失敗: {e}"))
}

fn open_in_browser(path: &PathBuf) -> Result<(), String> {
    open::that(path).map_err(|e| format!("ブラウザの起動に失敗: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_temp_path_trial_table_en() {
        let path = build_temp_path(&PanelItem::TrialTable, HelpLanguage::En);
        let file_name = path.file_name().unwrap().to_string_lossy();
        assert_eq!(file_name, "tunny-help-trial-table-en.html");
    }

    #[test]
    fn build_temp_path_trial_table_ja() {
        let path = build_temp_path(&PanelItem::TrialTable, HelpLanguage::Ja);
        let file_name = path.file_name().unwrap().to_string_lossy();
        assert_eq!(file_name, "tunny-help-trial-table-ja.html");
    }

    #[test]
    fn write_html_to_temp_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-help.html");
        let html = "<html><body>test</body></html>";
        write_html_to_temp(&path, html).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, html);
    }
}
