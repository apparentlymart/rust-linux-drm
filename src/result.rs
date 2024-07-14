#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    NonExist,
    SystemMem,
    GraphicsMem,
    Permission,
    Disconnected,
    NotSupported,
    RemoteFailure,
    Died,
    Other(linux_io::result::Error),
}

impl From<linux_io::result::Error> for Error {
    fn from(value: linux_io::result::Error) -> Self {
        match value {
            linux_io::result::EINVAL => Self::Invalid,
            linux_io::result::ENOENT => Self::NonExist,
            linux_io::result::ENOMEM => Self::SystemMem,
            linux_io::result::ENOSPC => Self::SystemMem,
            linux_io::result::EPERM | linux_io::result::EACCES => Self::Permission,
            linux_io::result::ENODEV => Self::Disconnected,
            linux_io::result::EOPNOTSUPP => Self::NotSupported,
            linux_io::result::ENXIO => Self::RemoteFailure,
            linux_io::result::EIO => Self::Died,
            _ => Self::Other(value),
        }
    }
}

impl Into<linux_io::result::Error> for Error {
    fn into(self) -> linux_io::result::Error {
        match self {
            Error::Invalid => linux_io::result::EINVAL,
            Error::NonExist => linux_io::result::ENOENT,
            Error::SystemMem => linux_io::result::ENOMEM,
            Error::GraphicsMem => linux_io::result::ENOSPC,
            Error::Permission => linux_io::result::EPERM,
            Error::Disconnected => linux_io::result::ENODEV,
            Error::NotSupported => linux_io::result::EOPNOTSUPP,
            Error::RemoteFailure => linux_io::result::ENXIO,
            Error::Died => linux_io::result::EIO,
            Error::Other(v) => v,
        }
    }
}

impl From<alloc::collections::TryReserveError> for Error {
    #[inline(always)]
    fn from(_: alloc::collections::TryReserveError) -> Self {
        Self::SystemMem
    }
}

#[derive(Debug)]
pub enum InitError {
    NotDrmCard,
    Other(linux_io::result::Error),
}

impl Into<linux_io::result::Error> for InitError {
    fn into(self) -> linux_io::result::Error {
        match self {
            InitError::NotDrmCard => linux_io::result::ENOTTY,
            InitError::Other(e) => e,
        }
    }
}

impl From<linux_io::result::Error> for InitError {
    fn from(value: linux_io::result::Error) -> Self {
        match value {
            linux_io::result::ENOTTY => InitError::NotDrmCard,
            _ => InitError::Other(value),
        }
    }
}
