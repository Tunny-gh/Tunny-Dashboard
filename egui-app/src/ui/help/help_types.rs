#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum HelpLanguage {
    #[default]
    En,
    Ja,
}

impl HelpLanguage {
    pub fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ja => "ja",
        }
    }
}

pub struct HelpContent {
    pub widget_name: &'static str,
    pub html_en: &'static str,
    pub html_ja: &'static str,
}

impl HelpContent {
    pub fn html(&self, lang: HelpLanguage) -> &'static str {
        match lang {
            HelpLanguage::En => self.html_en,
            HelpLanguage::Ja => self.html_ja,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_language_default_is_en() {
        assert_eq!(HelpLanguage::default(), HelpLanguage::En);
    }

    #[test]
    fn help_language_code_returns_correct_string() {
        assert_eq!(HelpLanguage::En.code(), "en");
        assert_eq!(HelpLanguage::Ja.code(), "ja");
    }

    #[test]
    fn help_content_html_returns_correct_language() {
        let content = HelpContent {
            widget_name: "test",
            html_en: "EN_HTML",
            html_ja: "JA_HTML",
        };
        assert_eq!(content.html(HelpLanguage::En), "EN_HTML");
        assert_eq!(content.html(HelpLanguage::Ja), "JA_HTML");
    }
}
