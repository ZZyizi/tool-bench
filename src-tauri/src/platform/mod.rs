pub mod port_scanner;
#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub mod unix;

use port_scanner::PortScanner;
use std::sync::Arc;

#[cfg(windows)]
pub fn create_scanner() -> Arc<dyn PortScanner> {
    Arc::new(windows::WindowsPortScanner)
}

#[cfg(unix)]
pub fn create_scanner() -> Arc<dyn PortScanner> {
    Arc::new(unix::UnixPortScanner)
}
