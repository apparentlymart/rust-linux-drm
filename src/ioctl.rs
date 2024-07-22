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

/// Fixed-point unsigned 16.16-bit number type, represented as [`u32`].
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct fixedu16_16(u32);

impl fixedu16_16 {
    #[inline(always)]
    pub fn from_u16(v: u16) -> Self {
        Self((v as u32) << 16)
    }

    #[inline(always)]
    pub fn from_u16_frac(w: u16, f: u16) -> Self {
        Self(((w as u32) << 16) | (f as u32))
    }

    #[inline(always)]
    pub fn as_raw_u32(self) -> u32 {
        self.0
    }
}

impl From<u16> for fixedu16_16 {
    #[inline(always)]
    fn from(value: u16) -> Self {
        Self::from_u16(value)
    }
}

impl From<u8> for fixedu16_16 {
    #[inline(always)]
    fn from(value: u8) -> Self {
        Self::from_u16(value as u16)
    }
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

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeFbDirtyCmd {
    pub fb_id: u32,
    pub flags: u32,
    pub color: u32,
    pub num_clips: u32,
    pub clips_ptr: u64,
}

impl_zeroed!(DrmModeFbDirtyCmd);

///
/// Mark a region of a framebuffer as dirty.
///
/// Some hardware does not automatically update display contents
/// as a hardware or software draw to a framebuffer. This ioctl
/// allows userspace to tell the kernel and the hardware what
/// regions of the framebuffer have changed.
///
/// The kernel or hardware is free to update more then just the
/// region specified by the clip rects. The kernel or hardware
/// may also delay and/or coalesce several calls to dirty into a
/// single update.
///
/// Userspace may annotate the updates, the annotates are a
/// promise made by the caller that the change is either a copy
/// of pixels or a fill of a single color in the region specified.
///
/// If the [`DRM_MODE_FB_DIRTY_ANNOTATE_COPY`] flag is given then
/// the number of updated regions are half of num_clips given,
/// where the clip rects are paired in src and dst. The width and
/// height of each one of the pairs must match.
///
/// If the [`DRM_MODE_FB_DIRTY_ANNOTATE_FILL`] flag is given the caller
/// promises that the region specified of the clip rects is filled
/// completely with a single color as given in the color argument.
///
pub const DRM_IOCTL_MODE_DIRTYFB: IoctlReqWriteRead<DrmCardDevice, DrmModeFbDirtyCmd, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeFbDirtyCmd>(0xb1)) };

pub const DRM_MODE_FB_DIRTY_ANNOTATE_COPY: u32 = 0x01;
pub const DRM_MODE_FB_DIRTY_ANNOTATE_FILL: u32 = 0x02;
pub const DRM_MODE_FB_DIRTY_FLAGS: u32 = 0x03;
pub const DRM_MODE_FB_DIRTY_MAX_CLIPS: u32 = 256;

#[repr(C)]
#[derive(Debug)]
pub struct DrmModeCrtcPageFlip {
    pub crtc_id: u32,
    pub fb_id: u32,
    pub flags: u32,
    /// Must always be set to zero.
    pub reserved: u32,
    pub user_data: u64,
}

impl_zeroed!(DrmModeCrtcPageFlip);

/// Request a page flip on the specified crtc.
///
/// This ioctl will ask KMS to schedule a page flip for the specified
/// crtc.  Once any pending rendering targeting the specified fb (as of
/// ioctl time) has completed, the crtc will be reprogrammed to display
/// that fb after the next vertical refresh.  The ioctl returns
/// immediately, but subsequent rendering to the current fb will block
/// in the execbuffer ioctl until the page flip happens.  If a page
/// flip is already pending as the ioctl is called, EBUSY will be
/// returned.
///
/// Flag [`DRM_MODE_PAGE_FLIP_EVENT`] requests that drm sends back a vblank
/// event (see drm.h: struct drm_event_vblank) when the page flip is
/// done.  The user_data field passed in with this ioctl will be
/// returned as the user_data field in the vblank event struct.
///
/// Flag [`DRM_MODE_PAGE_FLIP_ASYNC`] requests that the flip happen
/// 'as soon as possible', meaning that it not delay waiting for vblank.
/// This may cause tearing on the screen.
///
/// The reserved field must be zero.
pub const DRM_IOCTL_MODE_PAGE_FLIP: IoctlReqWriteRead<DrmCardDevice, DrmModeCrtcPageFlip, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeCrtcPageFlip>(0xb0)) };

/// Request that the kernel sends back a vblank event (see
/// struct drm_event_vblank) with the [`crate::event::raw::DRM_EVENT_FLIP_COMPLETE`]
/// type when the page-flip is done.
pub const DRM_MODE_PAGE_FLIP_EVENT: u32 = 0x01;
/// Request that the page-flip is performed as soon as possible, ie. with no
/// delay due to waiting for vblank. This may cause tearing to be visible on
/// the screen.
///
/// When used with atomic uAPI, the driver will return an error if the hardware
/// doesn't support performing an asynchronous page-flip for this update.
/// User-space should handle this, e.g. by falling back to a regular page-flip.
///
/// Note, some hardware might need to perform one last synchronous page-flip
/// before being able to switch to asynchronous page-flips. As an exception,
/// the driver will return success even though that first page-flip is not
/// asynchronous.
pub const DRM_MODE_PAGE_FLIP_ASYNC: u32 = 0x02;
pub const DRM_MODE_PAGE_FLIP_TARGET_ABSOLUTE: u32 = 0x4;
pub const DRM_MODE_PAGE_FLIP_TARGET_RELATIVE: u32 = 0x8;
pub const DRM_MODE_PAGE_FLIP_TARGET: u32 =
    DRM_MODE_PAGE_FLIP_TARGET_ABSOLUTE | DRM_MODE_PAGE_FLIP_TARGET_RELATIVE;
/// Bitmask of flags suitable for [`DrmModeCrtcPageFlip::flags`].
pub const DRM_MODE_PAGE_FLIP_FLAGS: u32 =
    DRM_MODE_PAGE_FLIP_EVENT | DRM_MODE_PAGE_FLIP_ASYNC | DRM_MODE_PAGE_FLIP_TARGET;

pub const DRM_MODE_CURSOR_BO: u32 = 0x01;
pub const DRM_MODE_CURSOR_MOVE: u32 = 0x02;
pub const DRM_MODE_CURSOR_FLAGS: u32 = 0x03;

pub struct DrmModeAtomic {
    pub flags: u32,
    pub count_objs: u32,
    pub objs_ptr: u64,
    pub count_props_ptr: u64,
    pub props_ptr: u64,
    pub prop_values_ptr: u64,
    pub reserved: u64,
    pub user_data: u64,
}

impl_zeroed!(DrmModeAtomic);

pub const DRM_IOCTL_MODE_ATOMIC: IoctlReqWriteRead<DrmCardDevice, DrmModeAtomic, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeAtomic>(0xbc)) };

/// Do not apply the atomic commit, and instead check whether the hardware supports
/// this configuration.
pub const DRM_MODE_ATOMIC_TEST_ONLY: u32 = 0x0100;

/// Do not block while applying the atomic commit. The [`DRM_IOCTL_MODE_ATOMIC`]
/// request returns immediately instead of waiting for the changes to be applied
/// in hardware. Note, the driver will still check whether the update can be
/// applied before retuning.
pub const DRM_MODE_ATOMIC_NONBLOCK: u32 = 0x0200;

/// Allow the update to result in temporary or transient visible artifacts while
/// the update is being applied. Applying the update may also take significantly
/// more time than a page flip. All visual artifacts will disappear by the time
/// the update is completed, as signalled through the vblank event's timestamp.
///
/// This flag must be set when the KMS update might cause visible artifacts.
/// Without this flag such KMS update will return an `EINVAL` error. What kind of
/// update may cause visible artifacts depends on the driver and the hardware.
/// User-space that needs to know beforehand if an update might cause visible
/// artifacts can use [`DRM_MODE_ATOMIC_TEST_ONLY`] without
/// [`DRM_MODE_ATOMIC_ALLOW_MODESET`] to see if it fails.
///
/// To the best of the driver's knowledge, visual artifacts are guaranteed to
/// not appear when this flag is not set. Some sinks might display visual
/// artifacts outside of the driver's control.
pub const DRM_MODE_ATOMIC_ALLOW_MODESET: u32 = 0x0400;

/// Bitfield of flags accepted by [`DRM_IOCTL_MODE_ATOMIC`] in
/// [`DrmModeAtomic::flags`].
pub const DRM_MODE_ATOMIC_FLAGS: u32 = DRM_MODE_PAGE_FLIP_EVENT
    | DRM_MODE_PAGE_FLIP_ASYNC
    | DRM_MODE_ATOMIC_TEST_ONLY
    | DRM_MODE_ATOMIC_NONBLOCK
    | DRM_MODE_ATOMIC_ALLOW_MODESET;

pub struct DrmModeObjGetProperties {
    pub props_ptr: u64,
    pub prop_values_ptr: u64,
    pub count_props: u32,
    pub obj_id: u32,
    pub obj_type: u32,
}

impl_zeroed!(DrmModeObjGetProperties);

pub const DRM_IOCTL_MODE_OBJ_GETPROPERTIES: IoctlReqWriteRead<
    DrmCardDevice,
    DrmModeObjGetProperties,
    int,
> = unsafe { ioctl_writeread(_IOWR::<DrmModeObjGetProperties>(0xb9)) };

pub struct DrmModeObjSetProperty {
    pub value: u64,
    pub prop_id: u32,
    pub obj_id: u32,
    pub obj_type: u32,
}

impl_zeroed!(DrmModeObjSetProperty);

pub const DRM_IOCTL_MODE_OBJ_SETPROPERTY: IoctlReqWriteRead<
    DrmCardDevice,
    DrmModeObjSetProperty,
    int,
> = unsafe { ioctl_writeread(_IOWR::<DrmModeObjSetProperty>(0xba)) };

pub const DRM_MODE_OBJECT_CRTC: u32 = 0xcccccccc;
pub const DRM_MODE_OBJECT_CONNECTOR: u32 = 0xc0c0c0c0;
pub const DRM_MODE_OBJECT_ENCODER: u32 = 0xe0e0e0e0;
pub const DRM_MODE_OBJECT_MODE: u32 = 0xdededede;
pub const DRM_MODE_OBJECT_PROPERTY: u32 = 0xb0b0b0b0;
pub const DRM_MODE_OBJECT_FB: u32 = 0xfbfbfbfb;
pub const DRM_MODE_OBJECT_BLOB: u32 = 0xbbbbbbbb;
pub const DRM_MODE_OBJECT_PLANE: u32 = 0xeeeeeeee;
pub const DRM_MODE_OBJECT_ANY: u32 = 0;

pub struct DrmModeGetPlaneRes {
    pub plane_id_ptr: u64,
    pub count_planes: u32,
}

impl_zeroed!(DrmModeGetPlaneRes);

pub const DRM_IOCTL_MODE_GETPLANERESOURCES: IoctlReqWriteRead<
    DrmCardDevice,
    DrmModeGetPlaneRes,
    int,
> = unsafe { ioctl_writeread(_IOWR::<DrmModeGetPlaneRes>(0xb5)) };

pub struct DrmModeGetPlane {
    pub plane_id: u32,
    pub crtc_id: u32,
    pub fb_id: u32,
    pub possible_crtcs: u32,
    pub gamma_size: u32,
    pub count_format_types: u32,
    pub format_type_ptr: u32,
}

impl_zeroed!(DrmModeGetPlane);

pub const DRM_IOCTL_MODE_GETPLANE: IoctlReqWriteRead<DrmCardDevice, DrmModeGetPlane, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeGetPlane>(0xb6)) };

pub struct DrmModeSetPlane {
    pub plane_id: u32,
    pub crtc_id: u32,
    pub fb_id: u32, // fb object contains surface format type
    pub flags: u32, // DRM_MODE_PRESENT_ flags

    pub crtc_x: i32,
    pub crtc_y: i32,
    pub crtc_w: u32,
    pub crtc_h: u32,

    pub src_x: fixedu16_16,
    pub src_y: fixedu16_16,
    pub src_h: fixedu16_16,
    pub src_w: fixedu16_16,
}

impl_zeroed!(DrmModeSetPlane);

pub const DRM_IOCTL_MODE_SETPLANE: IoctlReqWriteRead<DrmCardDevice, DrmModeSetPlane, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeSetPlane>(0xb7)) };

pub const DRM_MODE_PRESENT_TOP_FIELD: u32 = 1 << 0;
pub const DRM_MODE_PRESENT_BOTTOM_FIELD: u32 = 1 << 1;

pub struct DrmModeCreateBlob {
    pub data: u64,
    pub length: u32,
    pub blob_id: u32,
}

impl_zeroed!(DrmModeCreateBlob);

pub const DRM_IOCTL_MODE_CREATEPROPBLOB: IoctlReqWriteRead<DrmCardDevice, DrmModeCreateBlob, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmModeCreateBlob>(0xbd)) };

pub struct DrmModeDestroyBlob {
    pub blob_id: u32,
}

impl_zeroed!(DrmModeDestroyBlob);

pub const DRM_IOCTL_MODE_DESTROYPROPBLOB: IoctlReqWriteRead<
    DrmCardDevice,
    DrmModeDestroyBlob,
    int,
> = unsafe { ioctl_writeread(_IOWR::<DrmModeDestroyBlob>(0xbe)) };
