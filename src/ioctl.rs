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
#[derive(Debug)]
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

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeInfo {
    pub clock: u32,
    pub hdisplay: u16,
    pub hsync_start: u16,
    pub hsync_end: u16,
    pub htotal: u16,
    pub hskew: u16,
    pub vdisplay: u16,
    pub vsync_start: u16,
    pub vsync_end: u16,
    pub vtotal: u16,
    pub vscan: u16,
    pub vrefresh: u32,
    pub flags: u32,
    pub typ: u32,
    pub name: [core::ffi::c_char; 32],
}

pub const DRM_MODE_TYPE_PREFERRED: u32 = 1 << 3;
pub const DRM_MODE_TYPE_USERDEF: u32 = 1 << 5;
pub const DRM_MODE_TYPE_DRIVER: u32 = 1 << 6;

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeGetConnector {
    pub encoders_ptr: u64,
    pub modes_ptr: u64,
    pub props_ptr: u64,
    pub prop_values_ptr: u64,
    pub count_modes: u32,
    pub count_props: u32,
    pub count_encoders: u32,
    pub encoder_id: u32,
    pub connector_id: u32,
    pub connector_type: u32,
    pub connector_type_id: u32,
    pub connection: u32,
    pub mm_width: u32,
    pub mm_height: u32,
    pub subpixel: u32,
    #[doc(hidden)]
    pub _pad: u32,
}

impl_zeroed!(DrmModeGetConnector);

pub const DRM_IOCTL_MODE_GETCONNECTOR: IoctlReqWriteRead<DrmCardDevice, DrmModeGetConnector, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeGetConnector>(0xa7)) };

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeGetEncoder {
    pub encoder_id: u32,
    pub encoder_type: u32,
    pub crtc_id: u32,
    pub possible_crtcs: u32,
    pub possible_clones: u32,
}

impl_zeroed!(DrmModeGetEncoder);

pub const DRM_IOCTL_MODE_GETENCODER: IoctlReqWriteRead<DrmCardDevice, DrmModeGetEncoder, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeGetEncoder>(0xa6)) };

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeCrtc {
    pub set_connectors_ptr: u64,
    pub count_connectors: u32,
    pub crtc_id: u32,
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub gamma_size: u32,
    pub mode_valid: u32,
    pub mode: DrmModeInfo,
}

impl_zeroed!(DrmModeCrtc);

pub const DRM_IOCTL_MODE_GETCRTC: IoctlReqWriteRead<DrmCardDevice, DrmModeCrtc, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeCrtc>(0xa1)) };

pub const DRM_IOCTL_MODE_SETCRTC: IoctlReqWriteRead<DrmCardDevice, DrmModeCrtc, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeCrtc>(0xa2)) };

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeCreateDumb {
    pub height: u32,
    pub width: u32,
    pub bpp: u32,
    pub flags: u32,
    pub handle: u32,
    pub pitch: u32,
    pub size: u64,
}

impl_zeroed!(DrmModeCreateDumb);

pub const DRM_IOCTL_MODE_CREATE_DUMB: IoctlReqWriteRead<DrmCardDevice, DrmModeCreateDumb, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeCreateDumb>(0xb2)) };

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeMapDumb {
    pub handle: u32,
    pub pad: u32,
    pub offset: u64,
}

impl_zeroed!(DrmModeMapDumb);

pub const DRM_IOCTL_MODE_MAP_DUMB: IoctlReqWriteRead<DrmCardDevice, DrmModeMapDumb, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeMapDumb>(0xb3)) };

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeDestroyDumb {
    pub handle: u32,
}

impl_zeroed!(DrmModeDestroyDumb);

pub const DRM_IOCTL_MODE_DESTROY_DUMB: IoctlReqWriteRead<DrmCardDevice, DrmModeDestroyDumb, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeDestroyDumb>(0xb4)) };

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeFbCmd {
    pub fb_id: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
    pub depth: u32,
    pub handle: u32,
}

impl_zeroed!(DrmModeFbCmd);

pub const DRM_IOCTL_MODE_GETFB: IoctlReqWriteRead<DrmCardDevice, DrmModeFbCmd, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeFbCmd>(0xad)) };

pub const DRM_IOCTL_MODE_ADDFB: IoctlReqWriteRead<DrmCardDevice, DrmModeFbCmd, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeFbCmd>(0xae)) };

pub const DRM_IOCTL_MODE_RMFB: IoctlReqWriteRead<DrmCardDevice, linux_unsafe::uint, int> =
    unsafe { ioctl_writeread(_IOWR::<linux_unsafe::uint>(0xaf)) };
