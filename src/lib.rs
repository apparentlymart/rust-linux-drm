#![no_std]

use core::ops::{Deref, DerefMut};

/// Low-level `ioctl`-based access to DRM devices.
pub mod ioctl;

const ENOTTY: linux_io::result::Error = linux_io::result::Error(25);

#[repr(transparent)]
pub struct Card {
    f: linux_io::File<ioctl::DrmCardDevice>,
}

impl Card {
    pub fn open(path: &core::ffi::CStr) -> Result<Self, InitError> {
        let f = linux_io::File::open(path, linux_io::OpenOptions::read_write())?;
        Self::from_file(f)
    }

    pub fn from_file<D>(f: linux_io::File<D>) -> Result<Self, InitError> {
        // We'll use the VERSION ioctl to decide whether this file
        // seems to be a DRM card device. To do that we need to
        // first optimistically convert it to a DrmCardDevice,
        // so that our ioctl constant will be compatible.
        // Safety: We'll return this new f only if our ioctl
        // probe is successful, which therefore suggests that
        // this ought to be a DRM card device.
        let f: linux_io::File<ioctl::DrmCardDevice> = unsafe { f.to_device(ioctl::DrmCardDevice) };
        let mut v = ioctl::DrmVersion::zeroed();
        f.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        Ok(Self { f })
    }

    pub unsafe fn from_file_unchecked<D>(f: linux_io::File<D>) -> Self {
        let f: linux_io::File<ioctl::DrmCardDevice> = unsafe { f.to_device(ioctl::DrmCardDevice) };
        Self { f }
    }

    pub fn into_master(self) -> Result<CardMaster, (linux_io::result::Error, Self)> {
        if let Err(e) = self.f.ioctl(ioctl::DRM_IOCTL_SET_MASTER, ()) {
            return Err((e, self));
        }
        Ok(CardMaster { card: self })
    }

    pub fn close(self) -> linux_io::result::Result<()> {
        let f = self.take_file();
        f.close()
    }

    pub fn take_file(self) -> linux_io::File<ioctl::DrmCardDevice> {
        self.f
    }

    pub fn borrow_file(&self) -> &linux_io::File<ioctl::DrmCardDevice> {
        &self.f
    }

    pub fn borrow_file_mut(&mut self) -> &mut linux_io::File<ioctl::DrmCardDevice> {
        &mut self.f
    }
}

pub struct CardMaster {
    card: Card,
}

impl CardMaster {
    pub fn drop_master(self) -> Result<Card, (linux_io::result::Error, Self)> {
        if let Err(e) = self.f.ioctl(ioctl::DRM_IOCTL_DROP_MASTER, ()) {
            return Err((e, self));
        }
        Ok(self.card)
    }

    pub fn close(self) -> linux_io::result::Result<()> {
        let f = self.take_file();
        f.close()
    }

    pub fn take_file(self) -> linux_io::File<ioctl::DrmCardDevice> {
        self.card.f
    }
}

impl Deref for CardMaster {
    type Target = Card;

    fn deref(&self) -> &Self::Target {
        &self.card
    }
}

impl DerefMut for CardMaster {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.card
    }
}

impl<D> TryFrom<linux_io::File<D>> for Card {
    type Error = InitError;

    #[inline(always)]
    fn try_from(value: linux_io::File<D>) -> Result<Self, InitError> {
        Card::from_file(value)
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
            InitError::NotDrmCard => ENOTTY,
            InitError::Other(e) => e,
        }
    }
}

impl From<linux_io::result::Error> for InitError {
    fn from(value: linux_io::result::Error) -> Self {
        match value {
            ENOTTY => {
                // ENOTTY, so the file doesn't support this ioctl request
                // and so presumably isn't a DRM card.
                InitError::NotDrmCard
            }
            _ => InitError::Other(value),
        }
    }
}
