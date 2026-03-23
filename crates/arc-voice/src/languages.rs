// SPDX-License-Identifier: MIT
use std::collections::HashMap;

/// 20+ supported speech recognition languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    English,
    Spanish,
    French,
    German,
    Italian,
    Portuguese,
    Dutch,
    Russian,
    Chinese,
    Japanese,
    Korean,
    Arabic,
    Hindi,
    Turkish,
    Polish,
    Swedish,
    Danish,
    Norwegian,
    Finnish,
    Czech,
    Ukrainian,
    Thai,
    Vietnamese,
    Indonesian,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
            Self::French => "fr",
            Self::German => "de",
            Self::Italian => "it",
            Self::Portuguese => "pt",
            Self::Dutch => "nl",
            Self::Russian => "ru",
            Self::Chinese => "zh",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::Arabic => "ar",
            Self::Hindi => "hi",
            Self::Turkish => "tr",
            Self::Polish => "pl",
            Self::Swedish => "sv",
            Self::Danish => "da",
            Self::Norwegian => "no",
            Self::Finnish => "fi",
            Self::Czech => "cs",
            Self::Ukrainian => "uk",
            Self::Thai => "th",
            Self::Vietnamese => "vi",
            Self::Indonesian => "id",
        }
    }

    pub fn all() -> &'static [Language] {
        &[
            Self::English,
            Self::Spanish,
            Self::French,
            Self::German,
            Self::Italian,
            Self::Portuguese,
            Self::Dutch,
            Self::Russian,
            Self::Chinese,
            Self::Japanese,
            Self::Korean,
            Self::Arabic,
            Self::Hindi,
            Self::Turkish,
            Self::Polish,
            Self::Swedish,
            Self::Danish,
            Self::Norwegian,
            Self::Finnish,
            Self::Czech,
            Self::Ukrainian,
            Self::Thai,
            Self::Vietnamese,
            Self::Indonesian,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Italian => "Italiano",
            Self::Portuguese => "Português",
            Self::Dutch => "Nederlands",
            Self::Russian => "Русский",
            Self::Chinese => "中文",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Arabic => "العربية",
            Self::Hindi => "हिन्दी",
            Self::Turkish => "Türkçe",
            Self::Polish => "Polski",
            Self::Swedish => "Svenska",
            Self::Danish => "Dansk",
            Self::Norwegian => "Norsk",
            Self::Finnish => "Suomi",
            Self::Czech => "Čeština",
            Self::Ukrainian => "Українська",
            Self::Thai => "ไทย",
            Self::Vietnamese => "Tiếng Việt",
            Self::Indonesian => "Bahasa Indonesia",
        }
    }
}

pub fn language_model_map() -> HashMap<Language, &'static str> {
    let mut m = HashMap::new();
    for lang in Language::all() {
        m.insert(*lang, lang.code());
    }
    m
}
