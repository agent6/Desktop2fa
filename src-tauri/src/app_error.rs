use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("Could not find {0}")]
    NotFound(String),
    #[error("Secure storage is unavailable: {0}")]
    SecureStore(String),
    #[error("Persistence failed: {0}")]
    Persistence(String),
    #[error("Clipboard error: {0}")]
    Clipboard(arboard::Error),
    #[error("Window operation failed: {0}")]
    Window(String),
    #[error("Unexpected error: {0}")]
    Other(String),
}

impl AppError {
    pub fn into_command(self) -> String {
        self.to_string()
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::Persistence(value.to_string())
    }
}

impl From<tauri_plugin_store::Error> for AppError {
    fn from(value: tauri_plugin_store::Error) -> Self {
        Self::Persistence(value.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(value: tauri::Error) -> Self {
        Self::Window(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Persistence(value.to_string())
    }
}
