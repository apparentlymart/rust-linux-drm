use alloc::vec::Vec;
use core::ops::{BitAnd, BitOr};

mod atomic;
mod buffer;
mod props;

pub use atomic::*;
pub use buffer::*;
pub use props::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FramebufferId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct CrtcId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ConnectorId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct EncoderId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PlaneId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BufferObjectId(pub u32);

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ObjectId {
    Crtc(CrtcId),
    Connector(ConnectorId),
    Encoder(EncoderId),
    Mode(u32),
    Property(PropertyId),
    Framebuffer(FramebufferId),
    Blob(BlobId),
    Plane(PlaneId),
}

impl ObjectId {
    pub fn as_raw_type_and_id(self) -> (u32, u32) {
        use crate::ioctl;
        match self {
            ObjectId::Crtc(id) => (ioctl::DRM_MODE_OBJECT_CRTC, id.0),
            ObjectId::Connector(id) => (ioctl::DRM_MODE_OBJECT_CONNECTOR, id.0),
            ObjectId::Encoder(id) => (ioctl::DRM_MODE_OBJECT_ENCODER, id.0),
            ObjectId::Mode(id) => (ioctl::DRM_MODE_OBJECT_MODE, id),
            ObjectId::Property(id) => (ioctl::DRM_MODE_OBJECT_PROPERTY, id.0),
            ObjectId::Framebuffer(id) => (ioctl::DRM_MODE_OBJECT_FB, id.0),
            ObjectId::Blob(id) => (ioctl::DRM_MODE_OBJECT_BLOB, id.0),
            ObjectId::Plane(id) => (ioctl::DRM_MODE_OBJECT_PLANE, id.0),
        }
    }
}

impl From<CrtcId> for ObjectId {
    fn from(value: CrtcId) -> Self {
        Self::Crtc(value)
    }
}

impl From<ConnectorId> for ObjectId {
    fn from(value: ConnectorId) -> Self {
        Self::Connector(value)
    }
}

impl From<EncoderId> for ObjectId {
    fn from(value: EncoderId) -> Self {
        Self::Encoder(value)
    }
}

impl From<PropertyId> for ObjectId {
    fn from(value: PropertyId) -> Self {
        Self::Property(value)
    }
}

impl From<FramebufferId> for ObjectId {
    fn from(value: FramebufferId) -> Self {
        Self::Framebuffer(value)
    }
}

impl From<BlobId> for ObjectId {
    fn from(value: BlobId) -> Self {
        Self::Blob(value)
    }
}

impl From<PlaneId> for ObjectId {
    fn from(value: PlaneId) -> Self {
        Self::Plane(value)
    }
}

#[derive(Debug)]
pub struct CardResources {
    pub fb_ids: Vec<FramebufferId>,
    pub crtc_ids: Vec<CrtcId>,
    pub connector_ids: Vec<ConnectorId>,
    pub encoder_ids: Vec<EncoderId>,
    pub plane_ids: Vec<PlaneId>,
    pub min_width: u32,
    pub max_width: u32,
    pub min_height: u32,
    pub max_height: u32,
}

#[derive(Debug)]
pub struct ConnectorState {
    pub id: ConnectorId,
    pub current_encoder_id: EncoderId,
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
    pub encoder_id: EncoderId,
    pub encoder_type: u32,
    pub current_crtc_id: CrtcId,
    pub possible_crtcs: u32,
    pub possible_clones: u32,
}

#[derive(Debug)]
pub struct CrtcState {
    pub crtc_id: CrtcId,
    pub fb_id: FramebufferId,
    pub x: u32,
    pub y: u32,
    pub gamma_size: u32,
    pub mode_valid: u32,
    pub mode: ModeInfo,
}

impl From<crate::ioctl::DrmModeCrtc> for CrtcState {
    fn from(value: crate::ioctl::DrmModeCrtc) -> Self {
        Self {
            crtc_id: CrtcId(value.crtc_id),
            fb_id: FramebufferId(value.fb_id),
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
    pub id: PlaneId,
    pub crtc_id: CrtcId,
    pub fb_id: FramebufferId,
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
