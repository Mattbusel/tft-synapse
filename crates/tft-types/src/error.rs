use thiserror::Error;

#[derive(Debug, Error)]
pub enum TftError {
    #[error("Data catalog error: {0}")]
    Catalog(String),

    #[error("Feature extraction error: {0}")]
    FeatureExtraction(String),

    #[error("ML model error: {0}")]
    Model(String),

    #[error("Model persistence error: {0}")]
    Persistence(String),

    #[error("Game capture error: {0}")]
    Capture(String),

    #[error("Riot Live API error: {0}")]
    LiveApi(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Invalid game state: {0}")]
    InvalidState(String),

    #[error("Augment not found: {0}")]
    AugmentNotFound(String),

    #[error("Champion not found: {0}")]
    ChampionNotFound(String),

    #[error("IO error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("JSON error: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },

    #[error("Configuration error: {0}")]
    Config(String),
}
