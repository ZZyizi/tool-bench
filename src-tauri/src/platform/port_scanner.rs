use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub protocol: Protocol,
    pub port: u16,
    pub pid: u32,
    pub state: String,
    pub process_name: Option<String>,
}

#[derive(Debug, Error)]
pub enum PortError {
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("port not found")]
    NotFound,
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

pub trait PortScanner: Send + Sync {
    fn list(&self) -> Result<Vec<PortInfo>, PortError>;
    fn kill(&self, pid: u32) -> Result<(), PortError>;
}
