use thiserror::Error;

#[derive(Error, Debug)]
pub enum DmgrError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Sysfs parse error at {path}: {reason}")]
    SysfsParse { path: String, reason: String },

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Driver not found: {0}")]
    DriverNotFound(String),

    #[error("Bind operation failed: {0}")]
    BindFailed(String),

    #[error("Unbind operation failed: {0}")]
    UnbindFailed(String),

    #[error("Property not editable: {0}")]
    PropertyNotEditable(String),

    #[error("DBus error: {0}")]
    DBus(String),

    #[error("Udev error: {0}")]
    Udev(String),

    #[error("{0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, DmgrError>;
