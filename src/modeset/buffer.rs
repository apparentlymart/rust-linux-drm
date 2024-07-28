use alloc::sync::Weak;
use core::slice;

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
