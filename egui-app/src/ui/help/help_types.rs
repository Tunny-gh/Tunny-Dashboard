use crate::state::layout_state::PanelItem;

/// Help modal state
#[derive(Default)]
pub struct HelpModalState {
    pub open: bool,
    pub active_tab: usize,
    pub item: Option<PanelItem>,
}

/// Per-widget help content definition
pub struct HelpContent {
    pub title: &'static str,
    pub tabs: &'static [HelpTabDef],
}

/// Help modal tab definition
pub struct HelpTabDef {
    pub label: &'static str,
    pub markdown: &'static str,
}
