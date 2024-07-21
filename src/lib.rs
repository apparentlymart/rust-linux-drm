#![no_std]
#![feature(ptr_metadata)]

extern crate alloc;

/// Types and other symbols used for event handling.
pub mod event;
/// Low-level `ioctl`-based access to DRM devices.
pub mod ioctl;
/// Types and other symbols used for modesetting.
pub mod modeset;
pub mod result;

use core::ptr::null_mut;

use alloc::sync::Arc;
use alloc::vec::Vec;
use linux_io::fd::ioctl::IoctlReq;
use modeset::{EncoderState, ModeInfo, ModeProp};
use result::{Error, InitError};

#[repr(transparent)]
#[derive(Debug)]
pub struct Card {
    f: Arc<linux_io::File<ioctl::DrmCardDevice>>,
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
        let ret = Self { f: Arc::new(f) };
        let mut v = ioctl::DrmVersion::zeroed();
        ret.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        Ok(ret)
    }

    pub unsafe fn from_file_unchecked<D>(f: linux_io::File<D>) -> Self {
        let f: linux_io::File<ioctl::DrmCardDevice> = unsafe { f.to_device(ioctl::DrmCardDevice) };
        Self { f: Arc::new(f) }
    }

    pub fn api_version(&self) -> Result<ApiVersion, Error> {
        let mut v = ioctl::DrmVersion::zeroed();
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
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
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        Ok(&mut into[..v.name_len])
    }

    pub fn driver_name(&self) -> Result<Vec<u8>, Error> {
        let mut v = ioctl::DrmVersion::zeroed();
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        let len = v.name_len;
        let mut ret = vec_with_capacity(len)?;
        v = ioctl::DrmVersion::zeroed();
        v.name_len = len;
        v.name = ret.as_mut_ptr() as *mut _;
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        unsafe { ret.set_len(v.name_len) };
        Ok(ret)
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
        self.ioctl(ioctl::DRM_IOCTL_GET_CAP, &mut s)?;
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
        self.ioctl(ioctl::DRM_IOCTL_SET_CLIENT_CAP, &s)?;
        Ok(())
    }

    #[inline]
    pub fn become_master(&mut self) -> Result<(), Error> {
        self.ioctl(ioctl::DRM_IOCTL_SET_MASTER, ())?;
        Ok(())
    }

    #[inline]
    pub fn drop_master(&mut self) -> Result<(), Error> {
        self.ioctl(ioctl::DRM_IOCTL_DROP_MASTER, ())?;
        Ok(())
    }

    pub fn resources(&self) -> Result<modeset::CardResources, Error> {
        // The sets of resources can potentially change due to hotplug events
        // while we're producing this result, and so we need to keep retrying
        // until we get a consistent result.
        loop {
            let mut r = ioctl::DrmModeCardRes::zeroed();
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETRESOURCES, &mut r)?;
            let fb_count = r.count_fbs as usize;
            let connector_count = r.count_connectors as usize;
            let crtc_count = r.count_crtcs as usize;
            let encoder_count = r.count_encoders as usize;

            let mut fb_ids = vec_with_capacity::<u32>(fb_count)?;
            let mut connector_ids = vec_with_capacity::<u32>(connector_count)?;
            let mut crtc_ids = vec_with_capacity::<u32>(crtc_count)?;
            let mut encoder_ids = vec_with_capacity::<u32>(encoder_count)?;

            r = ioctl::DrmModeCardRes::zeroed();
            r.count_fbs = fb_count as u32;
            r.fb_id_ptr = fb_ids.as_mut_ptr() as u64;
            r.count_connectors = connector_count as u32;
            r.connector_id_ptr = connector_ids.as_mut_ptr() as u64;
            r.count_crtcs = crtc_count as u32;
            r.crtc_id_ptr = crtc_ids.as_mut_ptr() as u64;
            r.count_encoders = encoder_count as u32;
            r.encoder_id_ptr = encoder_ids.as_mut_ptr() as u64;

            self.ioctl(ioctl::DRM_IOCTL_MODE_GETRESOURCES, &mut r)?;
            // If any of the counts changed since our original call then the kernel
            // would not have populated the arrays and we'll need to retry and
            // hope that we don't collide with hotplugging next time.
            if r.count_fbs as usize != fb_count {
                continue;
            }
            if r.count_connectors as usize != connector_count {
                continue;
            }
            if r.count_crtcs as usize != crtc_count {
                continue;
            }
            if r.count_encoders as usize != encoder_count {
                continue;
            }

            // Safety: We ensured the slices capacities above, and ensured
            // that the kernel has populated the number of ids we expected
            // in each case.
            unsafe {
                fb_ids.set_len(fb_count);
                connector_ids.set_len(connector_count);
                crtc_ids.set_len(crtc_count);
                encoder_ids.set_len(encoder_count);
            };
            return Ok(modeset::CardResources {
                fb_ids,
                connector_ids,
                crtc_ids,
                encoder_ids,
                min_width: r.min_width,
                max_width: r.max_width,
                min_height: r.min_height,
                max_height: r.max_height,
            });
        }
    }

    pub fn connector_state(&self, connector_id: u32) -> Result<modeset::ConnectorState, Error> {
        // Hotplug events can cause the state to change between our calls, so
        // we'll keep retrying until we get a consistent result.
        loop {
            let mut tmp = ioctl::DrmModeGetConnector::zeroed();
            tmp.connector_id = connector_id;
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETCONNECTOR, &mut tmp)?;

            let mode_count = tmp.count_modes;
            let encoder_count = tmp.count_encoders;
            let prop_count = tmp.count_props;

            let mut modes = vec_with_capacity::<ioctl::DrmModeInfo>(mode_count as usize)?;
            let mut ret_modes = vec_with_capacity::<ModeInfo>(mode_count as usize)?;
            let mut encoder_ids = vec_with_capacity::<u32>(encoder_count as usize)?;
            let mut prop_ids = vec_with_capacity::<u32>(prop_count as usize)?;
            let mut prop_values = vec_with_capacity::<u64>(prop_count as usize)?;
            let mut ret_props = vec_with_capacity::<ModeProp>(prop_count as usize)?;

            tmp = ioctl::DrmModeGetConnector::zeroed();
            tmp.connector_id = connector_id;
            tmp.count_modes = mode_count;
            tmp.modes_ptr = modes.as_mut_ptr() as u64;
            tmp.count_encoders = encoder_count;
            tmp.encoders_ptr = encoder_ids.as_mut_ptr() as u64;
            tmp.count_props = prop_count;
            tmp.props_ptr = prop_ids.as_mut_ptr() as u64;
            tmp.prop_values_ptr = prop_values.as_mut_ptr() as u64;
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETCONNECTOR, &mut tmp)?;

            if tmp.count_modes != mode_count
                || tmp.count_props != prop_count
                || tmp.count_encoders != encoder_count
            {
                // Seems like things have changed since our first call, so we need to start over.
                continue;
            }

            // We can now safely set the lengths of the various vectors,
            // because we confirmed above that the kernel gave us the
            // lengths we asked for.
            unsafe {
                modes.set_len(mode_count as usize);
                encoder_ids.set_len(encoder_count as usize);
                prop_ids.set_len(prop_count as usize);
                prop_values.set_len(prop_count as usize);
            }

            ret_modes.extend(modes.into_iter().map(|raw| {
                let r: modeset::ModeInfo = raw.into();
                r
            }));
            ret_props.extend(
                core::iter::zip(prop_ids.iter().copied(), prop_values.iter().copied())
                    .map(|(prop_id, value)| ModeProp { prop_id, value }),
            );
            return Ok(modeset::ConnectorState {
                id: tmp.connector_id,
                current_encoder_id: tmp.encoder_id,
                connector_type: tmp.connector_type.into(),
                connector_type_id: tmp.connector_type_id,
                connection_state: tmp.connection.into(),
                width_mm: tmp.mm_width,
                height_mm: tmp.mm_height,
                subpixel_type: tmp.subpixel.into(),
                modes: ret_modes,
                props: ret_props,
                available_encoder_ids: encoder_ids,
            });
        }
    }

    pub fn encoder_state(&self, encoder_id: u32) -> Result<modeset::EncoderState, Error> {
        let mut tmp = ioctl::DrmModeGetEncoder::zeroed();
        tmp.encoder_id = encoder_id;
        self.ioctl(ioctl::DRM_IOCTL_MODE_GETENCODER, &mut tmp)?;
        Ok(EncoderState {
            encoder_id: tmp.encoder_id,
            encoder_type: tmp.encoder_type,
            current_crtc_id: tmp.crtc_id,
            possible_crtcs: tmp.possible_crtcs,
            possible_clones: tmp.possible_clones,
        })
    }

    pub fn crtc_state(&self, crtc_id: u32) -> Result<modeset::CrtcState, Error> {
        let mut tmp = ioctl::DrmModeCrtc::zeroed();
        tmp.crtc_id = crtc_id;
        self.ioctl(ioctl::DRM_IOCTL_MODE_GETCRTC, &mut tmp)?;
        Ok(tmp.into())
    }

    pub fn reset_crtc(&mut self, crtc_id: u32) -> Result<modeset::CrtcState, Error> {
        let mut tmp = ioctl::DrmModeCrtc::zeroed();
        tmp.crtc_id = crtc_id;
        self.ioctl(ioctl::DRM_IOCTL_MODE_SETCRTC, &mut tmp)?;
        Ok(tmp.into())
    }

    pub fn set_crtc_dumb_buffer(
        &mut self,
        crtc_id: u32,
        buf: &modeset::DumbBuffer,
        mode: &ModeInfo,
        conn_ids: &[u32],
    ) -> Result<modeset::CrtcState, Error> {
        let mut tmp = ioctl::DrmModeCrtc::zeroed();
        tmp.crtc_id = crtc_id;
        if conn_ids.len() > (u32::MAX as usize) {
            return Err(Error::Invalid);
        }
        tmp.count_connectors = conn_ids.len() as u32;
        tmp.set_connectors_ptr = conn_ids.as_ptr() as u64;
        tmp.fb_id = buf.fb_id;
        tmp.mode = mode.into();
        tmp.mode_valid = 1;

        self.ioctl(ioctl::DRM_IOCTL_MODE_SETCRTC, &mut tmp)?;
        Ok(tmp.into())
    }

    pub fn crtc_page_flip_dumb_buffer(
        &mut self,
        crtd_id: u32,
        buf: &modeset::DumbBuffer,
        flags: modeset::PageFlipFlags,
    ) -> Result<(), Error> {
        let mut tmp = ioctl::DrmModeCrtcPageFlip::zeroed();
        tmp.crtc_id = crtd_id;
        tmp.fb_id = buf.fb_id;
        tmp.flags = flags.into();
        self.ioctl(ioctl::DRM_IOCTL_MODE_PAGE_FLIP, &mut tmp)?;
        Ok(())
    }

    pub fn create_dumb_buffer(
        &self,
        req: modeset::DumbBufferRequest,
    ) -> Result<modeset::DumbBuffer, Error> {
        let mut buf_req = ioctl::DrmModeCreateDumb::zeroed();
        buf_req.width = req.width;
        buf_req.height = req.height;
        buf_req.bpp = req.bpp;
        self.ioctl(ioctl::DRM_IOCTL_MODE_CREATE_DUMB, &mut buf_req)?;

        // FIXME: If we fail after this point then we should free the dumb buffer.

        let mut fb_req = ioctl::DrmModeFbCmd::zeroed();
        fb_req.width = buf_req.width;
        fb_req.height = buf_req.height;
        fb_req.bpp = buf_req.bpp;
        fb_req.depth = req.depth;
        fb_req.pitch = buf_req.pitch;
        fb_req.handle = buf_req.handle;
        self.ioctl(ioctl::DRM_IOCTL_MODE_ADDFB, &mut fb_req)?;

        // FIXME: If we fail after this point then we should free the framebuffer object.

        let mut map_req = ioctl::DrmModeMapDumb::zeroed();
        map_req.handle = buf_req.handle;
        self.ioctl(ioctl::DRM_IOCTL_MODE_MAP_DUMB, &mut map_req)?;

        let buf_ptr = unsafe {
            self.f.mmap_raw(
                map_req.offset as i64,
                buf_req.size as usize,
                null_mut(),
                0b11, // PROT_READ | PROT_WRITE,
                0x01, // MAP_SHARED,
            )?
        };

        // The DumbBuffer object's Drop is responsible for freeing
        // the mmap, framebuffer object, and dumb buffer.
        Ok(modeset::DumbBuffer {
            width: buf_req.width,
            height: buf_req.height,
            bpp: buf_req.bpp,
            pitch: buf_req.pitch,
            ptr: buf_ptr as *mut u8,
            len: buf_req.size as usize,
            fb_id: fb_req.fb_id,
            buffer_handle: buf_req.handle,
            file: Arc::downgrade(&self.f),
        })
    }

    /// Read raw events from the card's file descriptor.
    ///
    /// DRM deals with events by having clients read from the card file descriptor,
    /// at which point the driver writes as many whole pending events as will fit
    /// into the given buffer. To give callers control over the buffer size, this
    /// function takes a preallocated mutable buffer to use for the temporary
    /// storage and then interprets the data one event at a time as the resulting
    /// iterator is used. The buffer should be at least large enough to contain
    /// one instance of the largest event type the kernel might return.
    ///
    /// If this function returns successfully then the caller *must* read the
    /// resulting iterator until it produces `None`, or otherwise any unread events
    /// will be lost.
    ///
    /// All objects returned from the iterator are views into portions of the
    /// provided buffer.
    pub fn read_events_raw<'a>(
        &self,
        buf: &'a mut [u8],
    ) -> Result<impl Iterator<Item = &'a event::raw::DrmEvent> + 'a, Error> {
        let len = self.f.read(buf)?;
        let buf = &buf[0..len];
        Ok(event::raw::events_from_bytes(buf))
    }

    /// Read events from the card's file descriptor.
    ///
    /// If this function returns successfully then the caller *must* read the
    /// resulting iterator until it produces `None`, or otherwise any unread
    /// events will be lost.
    ///
    /// This uses `buf` in the same way as [`Self::read_events_raw`], but
    /// instead of returning direct references to parts of the buffer it
    /// copies the event data into owned objects that can therefore outlive
    /// the buffer. This is really just a convenience wrapper around
    /// passing the [`Self::read_events_raw`] results through
    /// [`event::DrmEvent::from_raw`].
    ///
    /// Unlike [`Self::read_events_raw`], this function's iterator will
    /// sometimes perform dynamic allocations to capture the bodies of
    /// events with unrecognized types.
    pub fn read_events<'a>(
        &self,
        buf: &'a mut [u8],
    ) -> Result<impl Iterator<Item = event::DrmEvent> + 'a, Error> {
        let raws = self.read_events_raw(buf)?;
        Ok(raws.map(|raw| event::DrmEvent::from_raw(raw)))
    }

    #[inline]
    pub fn close(self) -> linux_io::result::Result<()> {
        let f = self.take_file()?;
        f.close()
    }

    pub fn take_file(self) -> linux_io::result::Result<linux_io::File<ioctl::DrmCardDevice>> {
        Arc::into_inner(self.f).ok_or(linux_io::result::EBUSY)
    }

    #[inline(always)]
    pub fn borrow_file(&self) -> &linux_io::File<ioctl::DrmCardDevice> {
        self.f.as_ref()
    }

    #[inline(always)]
    fn ioctl<'a, Req: IoctlReq<'a, ioctl::DrmCardDevice> + Copy>(
        &'a self,
        request: Req,
        arg: Req::ExtArg,
    ) -> linux_io::result::Result<Req::Result> {
        drm_ioctl(&self.f, request, arg)
    }
}

pub(crate) fn drm_ioctl<'a, Req: IoctlReq<'a, ioctl::DrmCardDevice> + Copy>(
    f: &'a linux_io::File<ioctl::DrmCardDevice>,
    request: Req,
    arg: Req::ExtArg,
) -> linux_io::result::Result<Req::Result> {
    // All DRM ioctls can potentially be interrupted if our process
    // receives a signal while we're waiting, so we'll keep retrying
    // until we get a non-interrupted result.
    //
    // This requires some unsafe trickery because the borrow checker
    // doesn't understand that only the final non-interrupted call
    // will actually make use of "arg".
    let arg_ptr = &arg as *const _;
    loop {
        let arg = unsafe { core::ptr::read(arg_ptr) };
        let ret = f.ioctl(request, arg);
        if !matches!(ret, Err(linux_io::result::EINTR)) {
            return ret;
        }
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

// Returns a vector that is guaranteed to have the given capacity exactly, or
// an error if there isn't enough memory to reserve that capacity.
//
// This is intended for situations where the kernel will then populate the
// reserved buffer and then the caller will set the length to something no
// greater than the capacity before returning.
fn vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, alloc::collections::TryReserveError> {
    let mut ret = Vec::<T>::new();
    ret.try_reserve_exact(capacity)?;
    Ok(ret)
}
