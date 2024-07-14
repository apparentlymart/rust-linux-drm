use core::ffi::c_int as int;
use core::ffi::c_ulong as ulong;

use linux_io::fd::ioctl::{
    ioctl_no_arg, ioctl_write, ioctl_writeread, IoDevice, IoctlReqNoArgs, IoctlReqWrite,
    IoctlReqWriteRead,
};

pub struct DrmCardDevice;

impl IoDevice for DrmCardDevice {}

const DRM_IOCTL_BASE: u64 = 100;

#[allow(non_snake_case)]
const fn _IO(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IO(DRM_IOCTL_BASE, nr)
}

#[allow(non_snake_case)]
const fn _IOW<T>(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IOW(DRM_IOCTL_BASE, nr, core::mem::size_of::<T>() as u64)
}

#[allow(non_snake_case)]
const fn _IOR<T>(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IOR(DRM_IOCTL_BASE, nr, core::mem::size_of::<T>() as u64)
}

#[allow(non_snake_case)]
const fn _IOWR<T>(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IOWR(DRM_IOCTL_BASE, nr, core::mem::size_of::<T>() as u64)
}

macro_rules! impl_zeroed {
    ($t:ty) => {
        impl $t {
            #[inline(always)]
            pub const fn zeroed() -> Self {
                // Safety: All of the field types in $t must
                // treat all-zeroes as a valid bit pattern.
                unsafe { ::core::mem::zeroed() }
            }
        }
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmVersion {
    pub version_major: int,
    pub version_minor: int,
    pub version_patchlevel: int,
    pub name_len: usize,
    pub name: *mut i8,
    pub date_len: usize,
    pub date: *mut i8,
    pub desc_len: usize,
    pub desc: *mut i8,
}

impl_zeroed!(DrmVersion);

pub const DRM_IOCTL_VERSION: IoctlReqWriteRead<DrmCardDevice, DrmVersion, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmVersion>(0x00)) };

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmSetVersion {
    pub drm_di_major: int,
    pub drm_di_minor: int,
    pub drm_dd_major: int,
    pub drm_dd_minor: int,
}

impl_zeroed!(DrmSetVersion);

pub const DRM_IOCTL_SET_VERSION: IoctlReqWriteRead<DrmCardDevice, DrmSetVersion, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmSetVersion>(0x07)) };

pub const DRM_IOCTL_SET_MASTER: IoctlReqNoArgs<DrmCardDevice, int> =
    unsafe { ioctl_no_arg(_IO(0x1e)) };

pub const DRM_IOCTL_DROP_MASTER: IoctlReqNoArgs<DrmCardDevice, int> =
    unsafe { ioctl_no_arg(_IO(0x1f)) };

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmGetCap {
    pub capability: DrmCap,
    pub value: u64,
}

impl_zeroed!(DrmGetCap);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct DrmCap(pub u64);

pub const DRM_IOCTL_GET_CAP: IoctlReqWriteRead<DrmCardDevice, DrmGetCap, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmGetCap>(0x0c)) };

pub const DRM_CAP_DUMB_BUFFER: DrmCap = DrmCap(0x1);
pub const DRM_CAP_VBLANK_HIGH_CRTC: DrmCap = DrmCap(0x2);
pub const DRM_CAP_DUMB_PREFERRED_DEPTH: DrmCap = DrmCap(0x3);
pub const DRM_CAP_DUMB_PREFER_SHADOW: DrmCap = DrmCap(0x4);
pub const DRM_CAP_PRIME: DrmCap = DrmCap(0x5);
pub const DRM_PRIME_CAP_IMPORT: DrmCap = DrmCap(0x1);
pub const DRM_PRIME_CAP_EXPORT: DrmCap = DrmCap(0x2);
pub const DRM_CAP_TIMESTAMP_MONOTONIC: DrmCap = DrmCap(0x6);
pub const DRM_CAP_ASYNC_PAGE_FLIP: DrmCap = DrmCap(0x7);
pub const DRM_CAP_CURSOR_WIDTH: DrmCap = DrmCap(0x8);
pub const DRM_CAP_CURSOR_HEIGHT: DrmCap = DrmCap(0x9);
pub const DRM_CAP_ADDFB2_MODIFIERS: DrmCap = DrmCap(0x10);
pub const DRM_CAP_PAGE_FLIP_TARGET: DrmCap = DrmCap(0x11);
pub const DRM_CAP_CRTC_IN_VBLANK_EVENT: DrmCap = DrmCap(0x12);
pub const DRM_CAP_SYNCOBJ: DrmCap = DrmCap(0x13);
pub const DRM_CAP_SYNCOBJ_TIMELINE: DrmCap = DrmCap(0x14);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmSetClientCap {
    pub capability: DrmClientCap,
    pub value: u64,
}

impl_zeroed!(DrmSetClientCap);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct DrmClientCap(pub u64);

pub const DRM_IOCTL_SET_CLIENT_CAP: IoctlReqWrite<DrmCardDevice, DrmSetClientCap, int> =
    unsafe { ioctl_write(_IOW::<DrmSetClientCap>(0x0d)) };

/**
 * If set to 1, the DRM core will expose the stereo 3D capabilities of the
 * monitor by advertising the supported 3D layouts in the flags of struct
 * drm_mode_modeinfo.
 */
pub const DRM_CLIENT_CAP_STEREO_3D: DrmClientCap = DrmClientCap(1);

/**
 * If set to 1, the DRM core will expose all planes (overlay, primary, and
 * cursor) to userspace.
 */
pub const DRM_CLIENT_CAP_UNIVERSAL_PLANES: DrmClientCap = DrmClientCap(2);

/**
 * If set to 1, the DRM core will expose atomic properties to userspace.
 */
pub const DRM_CLIENT_CAP_ATOMIC: DrmClientCap = DrmClientCap(3);

/**
 * If set to 1, the DRM core will provide aspect ratio information in modes.
 */
pub const DRM_CLIENT_CAP_ASPECT_RATIO: DrmClientCap = DrmClientCap(4);

/**
 * If set to 1, the DRM core will expose special connectors to be used for
 * writing back to memory the scene setup in the commit. Depends on client
 * also supporting DRM_CLIENT_CAP_ATOMIC
 */
pub const DRM_CLIENT_CAP_WRITEBACK_CONNECTORS: DrmClientCap = DrmClientCap(5);

#[repr(C)]
pub struct DrmModeCardRes {
    pub fb_id_ptr: u64,
    pub crtc_id_ptr: u64,
    pub connector_id_ptr: u64,
    pub encoder_id_ptr: u64,
    pub count_fbs: u32,
    pub count_crtcs: u32,
    pub count_connectors: u32,
    pub count_encoders: u32,
    pub min_width: u32,
    pub max_width: u32,
    pub min_height: u32,
    pub max_height: u32,
}

impl_zeroed!(DrmModeCardRes);

pub const DRM_IOCTL_MODE_GETRESOURCES: IoctlReqWriteRead<DrmCardDevice, DrmModeCardRes, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeCardRes>(0xa0)) };
