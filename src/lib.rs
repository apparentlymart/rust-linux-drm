#![no_std]

/// Low-level `ioctl`-based access to DRM devices.
pub mod ioctl;
pub mod result;

use result::{Error, InitError};

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

    pub fn api_version(&self) -> Result<ApiVersion, Error> {
        let mut v = ioctl::DrmVersion::zeroed();
        self.f.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        Ok(ApiVersion {
            major: v.version_major as i64,
            minor: v.version_minor as i64,
            patch: v.version_patchlevel as i64,
        })
    }

    pub fn read_driver_name<'a>(&self, into: &'a mut [u8]) -> Result<&'a mut [u8], Error> {
        let mut v = ioctl::DrmVersion::zeroed();
        let ptr = into.as_mut_ptr();
        v.name_len = into.len();
        v.name = ptr as *mut _;
        self.f.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        Ok(&mut into[..v.name_len])
    }

    #[inline(always)]
    pub fn get_device_cap(&self, capability: DeviceCap) -> Result<u64, Error> {
        self.get_device_cap_raw(capability.into())
    }

    #[inline]
    pub fn get_device_cap_raw(&self, capability: ioctl::DrmCap) -> Result<u64, Error> {
        let mut s = ioctl::DrmGetCap {
            capability,
            value: 0,
        };
        self.f.ioctl(ioctl::DRM_IOCTL_GET_CAP, &mut s)?;
        Ok(s.value)
    }

    #[inline(always)]
    pub fn set_client_cap(&self, capability: ClientCap, value: u64) -> Result<(), Error> {
        self.set_client_cap_raw(capability.into(), value)
    }

    #[inline]
    pub fn set_client_cap_raw(
        &self,
        capability: ioctl::DrmClientCap,
        value: u64,
    ) -> Result<(), Error> {
        let s = ioctl::DrmSetClientCap { capability, value };
        self.f.ioctl(ioctl::DRM_IOCTL_SET_CLIENT_CAP, &s)?;
        Ok(())
    }

    #[inline]
    pub fn become_master(&mut self) -> Result<(), Error> {
        self.f.ioctl(ioctl::DRM_IOCTL_SET_MASTER, ())?;
        Ok(())
    }

    #[inline]
    pub fn drop_master(&mut self) -> Result<(), Error> {
        self.f.ioctl(ioctl::DRM_IOCTL_DROP_MASTER, ())?;
        Ok(())
    }

    #[inline]
    pub fn close(self) -> linux_io::result::Result<()> {
        let f = self.take_file();
        f.close()
    }

    #[inline(always)]
    pub fn take_file(self) -> linux_io::File<ioctl::DrmCardDevice> {
        self.f
    }

    #[inline(always)]
    pub fn borrow_file(&self) -> &linux_io::File<ioctl::DrmCardDevice> {
        &self.f
    }

    #[inline(always)]
    pub fn borrow_file_mut(&mut self) -> &mut linux_io::File<ioctl::DrmCardDevice> {
        &mut self.f
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
pub struct ApiVersion {
    pub major: i64,
    pub minor: i64,
    pub patch: i64,
}

impl core::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{}.{}.{}", self.major, self.minor, self.patch))
    }
}

#[repr(u64)]
#[non_exhaustive]
pub enum DeviceCap {
    DumbBuffer = ioctl::DRM_CAP_DUMB_BUFFER.0,
    VBlankHighCrtc = ioctl::DRM_CAP_VBLANK_HIGH_CRTC.0,
    DumbPreferredDepth = ioctl::DRM_CAP_DUMB_PREFERRED_DEPTH.0,
    DumbPreferShadow = ioctl::DRM_CAP_DUMB_PREFER_SHADOW.0,
    Prime = ioctl::DRM_CAP_PRIME.0,
    TimestampMonotonic = ioctl::DRM_CAP_TIMESTAMP_MONOTONIC.0,
    AsyncPageFlip = ioctl::DRM_CAP_ASYNC_PAGE_FLIP.0,
    CursorWidth = ioctl::DRM_CAP_CURSOR_WIDTH.0,
    CursorHeight = ioctl::DRM_CAP_CURSOR_HEIGHT.0,
    Addfb2Modifiers = ioctl::DRM_CAP_ADDFB2_MODIFIERS.0,
    PageFlipTarget = ioctl::DRM_CAP_PAGE_FLIP_TARGET.0,
    CrtcInVblankEvent = ioctl::DRM_CAP_CRTC_IN_VBLANK_EVENT.0,
    Syncobj = ioctl::DRM_CAP_SYNCOBJ.0,
    SyncobjTimeline = ioctl::DRM_CAP_SYNCOBJ_TIMELINE.0,
}

impl From<DeviceCap> for ioctl::DrmCap {
    #[inline(always)]
    fn from(value: DeviceCap) -> Self {
        // We always use the raw value as the enum representation,
        // so this conversion is free.
        ioctl::DrmCap(value as u64)
    }
}

#[repr(u64)]
#[non_exhaustive]
pub enum ClientCap {
    Stereo3d = ioctl::DRM_CLIENT_CAP_STEREO_3D.0,
    UniversalPlanes = ioctl::DRM_CLIENT_CAP_UNIVERSAL_PLANES.0,
    Atomic = ioctl::DRM_CLIENT_CAP_ATOMIC.0,
    AspectRatio = ioctl::DRM_CLIENT_CAP_ASPECT_RATIO.0,
    WritebackConnectors = ioctl::DRM_CLIENT_CAP_WRITEBACK_CONNECTORS.0,
}

impl From<ClientCap> for ioctl::DrmClientCap {
    #[inline(always)]
    fn from(value: ClientCap) -> Self {
        // We always use the raw value as the enum representation,
        // so this conversion is free.
        ioctl::DrmClientCap(value as u64)
    }
}
