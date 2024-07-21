use core::ops::{BitAnd, BitOr};
use core::slice;

use alloc::sync::Weak;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct CardResources {
    pub fb_ids: Vec<u32>,
    pub crtc_ids: Vec<u32>,
    pub connector_ids: Vec<u32>,
    pub encoder_ids: Vec<u32>,
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
