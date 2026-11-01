//! Типизированные ошибки core (план: thiserror в core, anyhow в бинарнике).

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("unsupported locale: {0}")]
    UnsupportedLocale(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("settings parse error: {0}")]
    SettingsParse(#[from] toml::de::Error),

    #[error("settings serialize error: {0}")]
    SettingsSerialize(#[from] toml::ser::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid maven coordinate: {0}")]
    InvalidMaven(String),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}
