use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Directory hashing error: {0}")]
    DirectoryHashingError(String),

    #[error("Config packing error: {0}")]
    ConfigPackingError(String),

    #[error("Google Drive client error: {0}")]
    GoogleDriveError(String),

    #[error("Sync configuration error: {0}")]
    SyncConfigError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Base64 decoding error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("URL parsing error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Authentication required")]
    AuthenticationRequired,

    #[allow(dead_code)]
    #[error("No changes detected")]
    NoChangesDetected,

    #[allow(dead_code)]
    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[allow(dead_code)]
    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("General error: {0}")]
    GeneralError(#[from] anyhow::Error),
}

pub type SyncResult<T> = Result<T, SyncError>;
