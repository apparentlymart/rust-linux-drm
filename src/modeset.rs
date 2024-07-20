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
    pub connector_type: u32,
    pub connector_type_id: u32,
    pub connection_state: ConnectionState,
    pub width_mm: u32,
    pub height_mm: u32,
    pub subpixel_type: SubpixelType,
    pub modes: Vec<ModeInfo>,
    pub props: Vec<ModeProp>,
    pub available_encoder_ids: Vec<u32>,
}

#[derive(Debug)]
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
