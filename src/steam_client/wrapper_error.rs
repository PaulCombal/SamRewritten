#[derive(Debug)]
pub enum SteamError {
    NullVtable,
    PipeCreationFailed,
    PipeReleaseFailed,
    UserConnectionFailed,
    InterfaceCreationFailed(String),
    AppNotFound,
    UnknownError,
}

impl std::fmt::Display for SteamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteamError::NullVtable => write!(f, "Steam client vtable is null"),
            SteamError::PipeCreationFailed => write!(f, "Failed to create steam pipe"),
            SteamError::PipeReleaseFailed => write!(f, "Failed to release steam pipe"),
            SteamError::UserConnectionFailed => write!(f, "Failed to connect to steam server"),
            SteamError::InterfaceCreationFailed(name) => write!(f, "Failed to create steam interface: {}", name),
            SteamError::AppNotFound => write!(f, "App not found"),
            SteamError::UnknownError => write!(f, "Unknown Steam error"),
        }
    }
}

impl std::error::Error for SteamError {}