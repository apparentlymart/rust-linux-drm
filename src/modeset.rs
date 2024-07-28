use core::ops::{BitAnd, BitOr};
use core::slice;

use alloc::sync::Weak;
use alloc::vec::Vec;

mod atomic;

pub use atomic::*;

#[derive(Debug)]
pub struct CardResources {
    pub fb_ids: Vec<u32>,
    pub crtc_ids: Vec<u32>,
    pub connector_ids: Vec<u32>,
    pub encoder_ids: Vec<u32>,
    pub plane_ids: Vec<u32>,
    pub min_width: u32,
    pub max_width: u32,
    pub min_height: u32,
    pub max_height: u32,
}

#[derive(Debug)]
pub struct ConnectorState {
    pub id: u32,
    pub current_encoder_id: u32,
    pub connector_type: ConnectorType,
    pub connector_type_id: u32,
    pub connection_state: ConnectionState,
    pub width_mm: u32,
    pub height_mm: u32,
    pub subpixel_type: SubpixelType,
    pub modes: Vec<ModeInfo>,
    pub props: Vec<ModeProp>,
    pub available_encoder_ids: Vec<u32>,
}

impl ConnectorState {
    pub fn preferred_mode(&self) -> Option<&ModeInfo> {
        for mode in &self.modes {
            if (mode.typ & crate::ioctl::DRM_MODE_TYPE_PREFERRED) != 0 {
                return Some(mode);
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u32)]
pub enum ConnectionState {
    Connected = 1,
    Disconnected = 2,
    Unknown = 3,
}

impl From<u32> for ConnectionState {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Connected,
            2 => Self::Disconnected,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum ConnectorType {
    Unknown = 0,
    Vga = 1,
    DviI = 2,
    DviD = 3,
    DviA = 4,
    Composite = 5,
    SVideo = 6,
    Lvds = 7,
    Component = 8,
    NinePinDin = 9,
    DisplayPort = 10,
    HdmiA = 11,
    HdmiB = 12,
    Tv = 13,
    Edp = 14,
    Virtual = 15,
    Dsi = 16,
    Dpi = 17,
    Writeback = 18,
    Spi = 19,
    Usb = 20,
    Other = !0, // Not used by kernel, but used by us if kernel returns something we don't know
}

impl From<u32> for ConnectorType {
    #[inline]
    fn from(value: u32) -> Self {
        if value < 21 {
            // Safety: all values in this range are valid representations
            // of this enum, as described above.
            unsafe { core::mem::transmute(value) }
        } else {
            Self::Other
        }
    }
}

#[derive(Debug)]
pub struct EncoderState {
    pub encoder_id: u32,
    pub encoder_type: u32,
    pub current_crtc_id: u32,
    pub possible_crtcs: u32,
    pub possible_clones: u32,
}

#[derive(Debug)]
pub struct CrtcState {
    pub crtc_id: u32,
    pub fb_id: u32,
    pub x: u32,
    pub y: u32,
    pub gamma_size: u32,
    pub mode_valid: u32,
    pub mode: ModeInfo,
}

impl From<crate::ioctl::DrmModeCrtc> for CrtcState {
    fn from(value: crate::ioctl::DrmModeCrtc) -> Self {
        Self {
            crtc_id: value.crtc_id,
            fb_id: value.fb_id,
            x: value.x,
            y: value.y,
            gamma_size: value.gamma_size,
            mode_valid: value.mode_valid,
            mode: value.mode.into(),
        }
    }
}

#[derive(Debug)]
pub struct PlaneState {
    pub id: u32,
    pub crtc_id: u32,
    pub fb_id: u32,
    pub possible_crtcs: u32,
    pub gamma_size: u32,
}

#[derive(Debug)]
pub struct ModeInfo {
    pub name: Vec<u8>,
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
}

impl From<crate::ioctl::DrmModeInfo> for ModeInfo {
    fn from(value: crate::ioctl::DrmModeInfo) -> Self {
        let name = value.name[..].split(|c| *c == 0).next().unwrap();
        let name: &[u8] = unsafe { core::mem::transmute(name) };
        Self {
            name: name.to_vec(),
            clock: value.clock,
            hdisplay: value.hdisplay,
            hsync_start: value.hsync_start,
            hsync_end: value.hsync_end,
            htotal: value.htotal,
            hskew: value.hskew,
            vdisplay: value.vdisplay,
            vsync_start: value.vsync_start,
            vsync_end: value.vsync_end,
            vtotal: value.vtotal,
            vscan: value.vscan,
            vrefresh: value.vrefresh,
            flags: value.flags,
            typ: value.typ,
        }
    }
}

impl From<&ModeInfo> for crate::ioctl::DrmModeInfo {
    fn from(value: &ModeInfo) -> Self {
        let mut name_raw = [0_8; 32];
        let name_len = core::cmp::min(name_raw.len() - 1, value.name.len());
        let name_raw_slice = &mut name_raw[0..name_len];
        name_raw_slice
            .copy_from_slice(unsafe { core::mem::transmute(&value.name.as_slice()[0..name_len]) });
        Self {
            clock: value.clock,
            hdisplay: value.hdisplay,
            hsync_start: value.hsync_start,
            hsync_end: value.hsync_end,
            htotal: value.htotal,
            hskew: value.hskew,
            vdisplay: value.vdisplay,
            vsync_start: value.vsync_start,
            vsync_end: value.vsync_end,
            vtotal: value.vtotal,
            vscan: value.vscan,
            vrefresh: value.vrefresh,
            flags: value.flags,
            typ: value.typ,
            name: name_raw,
        }
    }
}

#[derive(Debug)]
pub struct ModeProp {
    pub prop_id: u32,
    pub value: u64,
}

#[derive(Debug)]
#[repr(u32)]
pub enum SubpixelType {
    Unknown = 1,
    HorizontalRgb = 2,
    HorizontalBgr = 3,
    VerticalRgb = 4,
    VerticalBgr = 5,
    None = 6,
}

impl From<u32> for SubpixelType {
    fn from(value: u32) -> Self {
        match value {
            2 => Self::HorizontalRgb,
            3 => Self::HorizontalBgr,
            4 => Self::VerticalRgb,
            5 => Self::VerticalBgr,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct PageFlipFlags(u32);

impl PageFlipFlags {
    pub const NONE: Self = Self(0);
    pub const EVENT: Self = Self(crate::ioctl::DRM_MODE_PAGE_FLIP_EVENT);
    pub const ASYNC: Self = Self(crate::ioctl::DRM_MODE_PAGE_FLIP_ASYNC);
}

impl BitOr for PageFlipFlags {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for PageFlipFlags {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl From<PageFlipFlags> for u32 {
    #[inline(always)]
    fn from(value: PageFlipFlags) -> Self {
        value.0
    }
}

#[derive(Debug)]
#[non_exhaustive]
#[repr(u32)]
pub enum PropertyType {
    Unknown = 0,
    Range = crate::ioctl::DRM_MODE_PROP_RANGE,
    Enum = crate::ioctl::DRM_MODE_PROP_ENUM,
    Blob = crate::ioctl::DRM_MODE_PROP_BLOB,
    Bitmask = crate::ioctl::DRM_MODE_PROP_BITMASK,
    Object = crate::ioctl::DRM_MODE_PROP_OBJECT,
    SignedRange = crate::ioctl::DRM_MODE_PROP_SIGNED_RANGE,
}

impl PropertyType {
    pub fn from_raw_flags(flags: u32) -> (Self, bool) {
        let immutable = (flags & crate::ioctl::DRM_MODE_PROP_IMMUTABLE) != 0;
        let type_raw = flags
            & (crate::ioctl::DRM_MODE_PROP_LEGACY_TYPE | crate::ioctl::DRM_MODE_PROP_EXTENDED_TYPE);
        let typ = match type_raw {
            crate::ioctl::DRM_MODE_PROP_RANGE => Self::Range,
            crate::ioctl::DRM_MODE_PROP_ENUM => Self::Enum,
            crate::ioctl::DRM_MODE_PROP_BLOB => Self::Blob,
            crate::ioctl::DRM_MODE_PROP_BITMASK => Self::Bitmask,
            crate::ioctl::DRM_MODE_PROP_OBJECT => Self::Object,
            crate::ioctl::DRM_MODE_PROP_SIGNED_RANGE => Self::SignedRange,
            _ => Self::Unknown,
        };
        (typ, immutable)
    }
}

#[derive(Debug)]
pub struct ObjectPropMeta<'card> {
    pub(crate) raw: crate::ioctl::DrmModeGetProperty,
    pub(crate) card: &'card crate::Card,
}

impl<'card> ObjectPropMeta<'card> {
    #[inline(always)]
    pub(crate) fn new(raw: crate::ioctl::DrmModeGetProperty, card: &'card crate::Card) -> Self {
        Self { raw, card }
    }

    #[inline]
    pub fn property_id(&self) -> u32 {
        self.raw.prop_id
    }

    pub fn name(&self) -> &str {
        let raw = &self.raw.name[..];
        let raw = raw.split(|c| *c == 0).next().unwrap();
        // Safety: We assume that raw.name is always ASCII; that should
        // have been guaranteed by any codepath that instantiates
        // an ObjectPropMeta object.
        let raw = unsafe { raw.as_ascii_unchecked() };
        raw.as_str()
    }

    #[inline]
    pub fn property_type(&self) -> PropertyType {
        let (ret, _) = PropertyType::from_raw_flags(self.raw.flags);
        ret
    }

    #[inline]
    pub fn is_immutable(&self) -> bool {
        let (_, immut) = PropertyType::from_raw_flags(self.raw.flags);
        immut
    }

    #[inline]
    pub fn is_mutable(&self) -> bool {
        !self.is_immutable()
    }

    /// Get the minimum and maximum value for a range-typed property.
    ///
    /// This is essentially the same as [`Self::values`], except that because it's
    /// guaranteed that a range property always has exactly two values this function
    /// can avoid making a dynamic memory allocation and can instead retrieve the
    /// values directly into a stack object and then return those values.
    pub fn range(&self) -> Result<(u64, u64), crate::Error> {
        const RANGE_TYPES: u32 =
            crate::ioctl::DRM_MODE_PROP_RANGE | crate::ioctl::DRM_MODE_PROP_SIGNED_RANGE;
        let is_range = (self.raw.flags & RANGE_TYPES) != 0;
        if !is_range {
            return Err(crate::Error::NotSupported);
        }
        // Range types should always have exactly two values.
        if self.raw.count_values != 2 {
            return Err(crate::Error::RemoteFailure);
        }

        let mut pair = [0_u64; 2];

        let mut tmp = crate::ioctl::DrmModeGetProperty::zeroed();
        tmp.prop_id = self.raw.prop_id;
        tmp.count_values = 2;
        tmp.values_ptr = &mut pair as *mut _ as u64;
        self.card
            .ioctl(crate::ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut tmp)
            .unwrap();

        if tmp.count_values != 2 {
            // Something has gone horribly wrong.
            return Err(crate::Error::RemoteFailure);
        }

        Ok((pair[0], pair[1]))
    }

    /// Get a vector of describing the values that are acceptable for this property.
    ///
    /// The meaning of the result depends on the property type:
    /// - For a range or signed range, the result always has length 2 and describes
    ///   the minimum and maximum values respectively.
    ///
    ///     You can avoid a dynamic memory allocation in this case by using
    ///     [`Self::range`] instead.
    /// - For an enum or bitmask, the result describes the values of the
    ///   valid enumeration members.
    ///
    ///     For these it's typically better to use [`Self::enum_members`] since
    ///     that can also return the name associated with each value.
    pub fn values(&self) -> Result<Vec<u64>, crate::Error> {
        let mut count = self.raw.count_values as usize;
        loop {
            let mut values = crate::vec_with_capacity::<u64>(count)?;

            let mut tmp = crate::ioctl::DrmModeGetProperty::zeroed();
            tmp.prop_id = self.raw.prop_id;
            tmp.count_values = count as u32;
            tmp.values_ptr = values.as_mut_ptr() as u64;

            self.card
                .ioctl(crate::ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut tmp)
                .unwrap();

            let new_count = tmp.count_values as usize;
            if new_count != count {
                count = new_count;
                continue;
            }

            // Safety: We confirmed above that the kernel generated the number
            // of values we were expecting.
            unsafe {
                values.set_len(count);
            }
            return Ok(values);
        }
    }

    /// Get a vector describing the valid values for an enum, or the bitfield values
    /// for a bitmask.
    pub fn enum_members(&self) -> Result<Vec<ObjectPropEnumMember>, crate::Error> {
        const ENUM_TYPES: u32 =
            crate::ioctl::DRM_MODE_PROP_ENUM | crate::ioctl::DRM_MODE_PROP_BITMASK;
        let is_enum = (self.raw.flags & ENUM_TYPES) != 0;
        if !is_enum {
            return Err(crate::Error::NotSupported);
        }

        let mut count = self.raw.count_enum_blobs as usize;
        loop {
            // Safety: The following relies on ObjectPropEnumMember having identical
            // layout to ioctl::DrmModePropertyEnum, which we ensure by marking
            // it as repr(transparent).
            let mut members = crate::vec_with_capacity::<ObjectPropEnumMember>(count)?;

            let mut tmp = crate::ioctl::DrmModeGetProperty::zeroed();
            tmp.prop_id = self.raw.prop_id;
            tmp.count_enum_blobs = count as u32;
            tmp.enum_blob_ptr = members.as_mut_ptr() as u64;

            self.card
                .ioctl(crate::ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut tmp)
                .unwrap();

            let new_count = tmp.count_enum_blobs as usize;
            if new_count != count {
                count = new_count;
                continue;
            }

            // Safety: We confirmed above that the kernel generated the number
            // of values we were expecting.
            unsafe {
                members.set_len(count);
            }
            return Ok(members);
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ObjectPropEnumMember {
    raw: crate::ioctl::DrmModePropertyEnum,
}

impl ObjectPropEnumMember {
    #[inline]
    pub fn value(&self) -> u64 {
        self.raw.value
    }

    pub fn name(&self) -> &str {
        let raw = &self.raw.name[..];
        let raw = raw.split(|c| *c == 0).next().unwrap();
        // The following assumes that the kernel will only use ASCII
        // characters in enum member names, which has been true so
        // far.
        let raw = raw.as_ascii().unwrap();
        raw.as_str()
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ObjectId {
    Crtc(u32),
    Connector(u32),
    Encoder(u32),
    Mode(u32),
    Property(u32),
    Framebuffer(u32),
    Blob(u32),
    Plane(u32),
}

impl ObjectId {
    pub fn as_raw_type_and_id(self) -> (u32, u32) {
        use crate::ioctl;
        match self {
            ObjectId::Crtc(id) => (ioctl::DRM_MODE_OBJECT_CRTC, id),
            ObjectId::Connector(id) => (ioctl::DRM_MODE_OBJECT_CONNECTOR, id),
            ObjectId::Encoder(id) => (ioctl::DRM_MODE_OBJECT_ENCODER, id),
            ObjectId::Mode(id) => (ioctl::DRM_MODE_OBJECT_MODE, id),
            ObjectId::Property(id) => (ioctl::DRM_MODE_OBJECT_PROPERTY, id),
            ObjectId::Framebuffer(id) => (ioctl::DRM_MODE_OBJECT_FB, id),
            ObjectId::Blob(id) => (ioctl::DRM_MODE_OBJECT_BLOB, id),
            ObjectId::Plane(id) => (ioctl::DRM_MODE_OBJECT_PLANE, id),
        }
    }
}

#[derive(Debug)]
pub struct DumbBufferRequest {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub bpp: u32,
}

#[derive(Debug)]
pub struct DumbBuffer {
    pub(crate) ptr: *mut u8,
    pub(crate) len: usize,
    pub(crate) file: Weak<linux_io::File<crate::ioctl::DrmCardDevice>>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) bpp: u32,
    pub(crate) pitch: u32,
    pub(crate) fb_id: u32,
    pub(crate) buffer_handle: u32,
}

impl DumbBuffer {
    pub fn buffer(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pitch(&self) -> u32 {
        self.pitch
    }

    pub fn bpp(&self) -> u32 {
        self.bpp
    }

    pub fn framebuffer_id(&self) -> u32 {
        self.fb_id
    }

    pub fn pixel_idx(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y as usize * self.pitch as usize) + (x as usize * (self.bpp / 8) as usize))
    }

    pub fn clear_to_zero(&mut self) {
        unsafe { core::ptr::write_bytes(self.ptr, 0, self.len) }
    }
}

impl Drop for DumbBuffer {
    fn drop(&mut self) {
        let _ = unsafe { linux_unsafe::munmap(self.ptr as *mut _, self.len) };

        // If the associated file is still open then we'll also free the framebuffer and
        // dumb buffer. Otherwise we'll just hope that the file descriptor associated with
        // self.file got properly closed so that the kernel could free these automatically.
        let Some(f) = self.file.upgrade() else {
            return;
        };
        {
            let mut fb_id = self.fb_id;
            let _ = crate::drm_ioctl(f.as_ref(), crate::ioctl::DRM_IOCTL_MODE_RMFB, &mut fb_id);
        }
        {
            let mut msg = crate::ioctl::DrmModeDestroyDumb::zeroed();
            msg.handle = self.buffer_handle;
            let _ = crate::drm_ioctl(
                f.as_ref(),
                crate::ioctl::DRM_IOCTL_MODE_DESTROY_DUMB,
                &mut msg,
            );
        }
    }
}
