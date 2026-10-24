use crate::state::layout_state::PanelItem;
use crate::ui::help::help_url::{overview_url, widget_url};

/// Opens the online documentation page for a widget in the default browser.
pub fn open_help(item: &PanelItem) -> Result<(), String> {
    open_url(&widget_url(item))
}

/// Opens the documentation top page (Overview) in the default browser.
pub fn open_overview() -> Result<(), String> {
    open_url(&overview_url())
}

fn open_url(url: &str) -> Result<(), String> {
    open::that(url).map_err(|e| format!("Failed to open the documentation site: {e}"))
}
