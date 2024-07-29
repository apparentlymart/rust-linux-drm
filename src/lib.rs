#![no_std]
#![feature(ptr_metadata)]
#![feature(ascii_char)]

extern crate alloc;

/// Types and other symbols used for event handling.
pub mod event;
/// Low-level `ioctl`-based access to DRM devices.
pub mod ioctl;
/// Types and other symbols used for modesetting.
pub mod modeset;
pub mod result;

mod util;

use core::iter::{self, zip};
use core::ptr::null_mut;

use alloc::sync::Arc;
use alloc::vec::Vec;
use linux_io::fd::ioctl::IoctlReq;
use modeset::{
    BlobId, BufferObjectId, ConnectorId, CrtcId, EncoderId, EncoderState, FramebufferId, ModeInfo,
    ModeProp, PlaneId,
};
use result::{Error, InitError};

#[repr(transparent)]
#[derive(Debug)]
pub struct Card {
    f: Arc<linux_io::File<ioctl::DrmCardDevice>>,
}

impl Card {
    /// Open the file at the given path and attempt to use it as a
    /// DRM card file descriptor.
    ///
    /// Returns [`result::InitError::NotDrmCard`] if the opened file
    /// does not support the `DRM_IOCTL_VERSION` ioctl request.
    pub fn open(path: &core::ffi::CStr) -> Result<Self, InitError> {
        let f = linux_io::File::open(path, linux_io::OpenOptions::read_write())?;
        Self::from_file(f)
    }

    /// Attempt to use the given file as a DRM card device.
    ///
    /// Returns [`result::InitError::NotDrmCard`] if the opened file
    /// does not support the `DRM_IOCTL_VERSION` ioctl request.
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

    /// Wraps the given file in [`Card`] without checking whether
    /// it supports any DRM card ioctl requests.
    pub unsafe fn from_file_unchecked<D>(f: linux_io::File<D>) -> Self {
        let f: linux_io::File<ioctl::DrmCardDevice> = unsafe { f.to_device(ioctl::DrmCardDevice) };
        Self { f: Arc::new(f) }
    }

    /// Get the open file descriptor for the card.
    ///
    /// Interacting with this file descriptor outside of the [`Card`] abstraction
    /// may cause the abstraction to malfunction. It's exposed primarily so
    /// it can be used with system functions like `poll` to wait for events
    /// on multiple file descriptors at once.
    pub fn fd(&self) -> linux_unsafe::int {
        self.f.fd()
    }

    /// Determine the DRM API version supported by this device.
    pub fn api_version(&self) -> Result<ApiVersion, Error> {
        let mut v = ioctl::DrmVersion::zeroed();
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        Ok(ApiVersion {
            major: v.version_major as i64,
            minor: v.version_minor as i64,
            patch: v.version_patchlevel as i64,
        })
    }

    /// Read the driver name into the given slice.
    pub fn read_driver_name<'a>(&self, into: &'a mut [u8]) -> Result<&'a mut [u8], Error> {
        let mut v = ioctl::DrmVersion::zeroed();
        let ptr = into.as_mut_ptr();
        unsafe { v.set_name_ptr(ptr as *mut _, into.len()) };
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        Ok(&mut into[..v.name_len()])
    }

    /// Read the driver name into a vector.
    pub fn driver_name(&self) -> Result<Vec<u8>, Error> {
        let mut v = ioctl::DrmVersion::zeroed();
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        let len = v.name_len();
        let mut ret = vec_with_capacity::<u8>(len)?;
        v = ioctl::DrmVersion::zeroed();
        unsafe { v.set_name_ptr(ret.as_mut_ptr() as *mut _, len) };
        self.ioctl(ioctl::DRM_IOCTL_VERSION, &mut v)?;
        unsafe { ret.set_len(v.name_len()) };
        Ok(ret)
    }

    /// Read a device capability value.
    #[inline(always)]
    pub fn get_device_cap(&self, capability: DeviceCap) -> Result<u64, Error> {
        self.get_device_cap_raw(capability.into())
    }

    /// Read a device capability value using a raw capability number.
    #[inline]
    pub fn get_device_cap_raw(&self, capability: ioctl::DrmCap) -> Result<u64, Error> {
        let mut s = ioctl::DrmGetCap {
            capability,
            value: 0,
        };
        self.ioctl(ioctl::DRM_IOCTL_GET_CAP, &mut s)?;
        Ok(s.value)
    }

    /// Read a device capability value using a raw capability number.
    #[inline(always)]
    pub fn set_client_cap(&mut self, capability: ClientCap, value: u64) -> Result<(), Error> {
        self.set_client_cap_raw(capability.into(), value)
    }

    /// Attempt to set a client capability, which might then change the behavior
    /// of other device functions.
    #[inline]
    pub fn set_client_cap_raw(
        &mut self,
        capability: ioctl::DrmClientCap,
        value: u64,
    ) -> Result<(), Error> {
        let s = ioctl::DrmSetClientCap { capability, value };
        self.ioctl(ioctl::DRM_IOCTL_SET_CLIENT_CAP, &s)?;
        Ok(())
    }

    /// Attempt to become the "master" of this device, which is required for
    /// modesetting.
    #[inline]
    pub fn become_master(&mut self) -> Result<(), Error> {
        self.ioctl(ioctl::DRM_IOCTL_SET_MASTER, ())?;
        Ok(())
    }

    /// Release the "master" status of this device, thus allowing other
    /// processes to claim it.
    #[inline]
    pub fn drop_master(&mut self) -> Result<(), Error> {
        self.ioctl(ioctl::DRM_IOCTL_DROP_MASTER, ())?;
        Ok(())
    }

    /// Get metadata about a DRM property using its id.
    ///
    /// Property ids are assigned dynamically and so must be detected at runtime.
    pub fn property_meta(
        &self,
        prop_id: modeset::PropertyId,
    ) -> Result<modeset::ObjectPropMeta, Error> {
        let mut tmp = ioctl::DrmModeGetProperty::zeroed();
        tmp.prop_id = prop_id.0;
        self.ioctl(ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut tmp)?;
        if !tmp.name.is_ascii() {
            // ObjectPropMeta assumes that the name is always ASCII so
            // we can cheaply treat it as a str, which has been true
            // so far but we'll make sure things stay sound by
            // rejecting any property with a non-ASCII name.
            return Err(Error::NotSupported);
        }
        Ok(modeset::ObjectPropMeta::new(tmp, &self))
    }

    /// Get the properties of the specified object in their raw form.
    ///
    /// Use either [`Self::property_meta`] or [`Self::each_object_property_meta`]
    /// to discover the name and type information for each property id.
    pub fn object_properties(
        &self,
        obj_id: impl Into<modeset::ObjectId>,
    ) -> Result<Vec<modeset::ModeProp>, Error> {
        fn real_object_properties(
            card: &Card,
            obj_id: modeset::ObjectId,
        ) -> Result<Vec<modeset::ModeProp>, Error> {
            let (type_id, raw_id) = obj_id.as_raw_type_and_id();
            let mut tmp = ioctl::DrmModeObjGetProperties::zeroed();
            tmp.obj_type = type_id;
            tmp.obj_id = raw_id;
            card.ioctl(ioctl::DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut tmp)?;

            // The sets of properties can potentially change due to hotplug events
            // while we're producing this result, and so we need to keep retrying
            // until we get a consistent result.
            loop {
                let prop_count = tmp.count_props() as usize;

                let mut prop_ids = vec_with_capacity::<u32>(prop_count)?;
                let mut prop_values = vec_with_capacity::<u64>(prop_count)?;

                unsafe {
                    tmp.set_prop_ptrs(
                        prop_ids.as_mut_ptr(),
                        prop_values.as_mut_ptr(),
                        prop_count as u32,
                    )
                };

                card.ioctl(ioctl::DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut tmp)?;

                let new_prop_count = tmp.count_props() as usize;
                if new_prop_count != prop_count {
                    // The number of properties has changed since the previous
                    // request, so we'll retry.
                    continue;
                }

                // Safety: We ensured the slices capacities above, and ensured
                // that the kernel has populated the number of ids we expected
                // in each case.
                unsafe {
                    prop_ids.set_len(prop_count);
                    prop_values.set_len(prop_count);
                };
                return Ok(iter::zip(prop_ids.into_iter(), prop_values.into_iter())
                    .map(|(id, val)| modeset::ModeProp {
                        prop_id: modeset::PropertyId(id),
                        value: val,
                    })
                    .collect());
            }
        }
        real_object_properties(self, obj_id.into())
    }

    /// Call `f` with the metadata for each property of the object with the given id.
    ///
    /// This is intended for use by callers that want to build a lookup
    /// table of property ids for later use in efficiently setting those
    /// properties. Pass a closure that mutates the lookup table only
    /// for the subset of properties that are interesting.
    pub fn each_object_property_meta(
        &self,
        obj_id: impl Into<modeset::ObjectId>,
        mut f: impl FnMut(modeset::ObjectPropMeta, u64),
    ) -> Result<(), Error> {
        let obj_id = obj_id.into();
        let (type_id, raw_id) = obj_id.as_raw_type_and_id();
        let mut tmp = ioctl::DrmModeObjGetProperties::zeroed();
        tmp.obj_type = type_id;
        tmp.obj_id = raw_id;
        self.ioctl(ioctl::DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut tmp)?;
        if tmp.count_props() == 0 {
            return Ok(());
        }

        let (prop_ids, prop_values) = loop {
            let prop_count = tmp.count_props() as usize;
            let mut prop_ids = vec_with_capacity::<u32>(prop_count)?;
            let mut prop_values = vec_with_capacity::<u64>(prop_count)?;
            unsafe {
                tmp.set_prop_ptrs(
                    prop_ids.as_mut_ptr(),
                    prop_values.as_mut_ptr(),
                    prop_count as u32,
                )
            };
            self.ioctl(ioctl::DRM_IOCTL_MODE_OBJ_GETPROPERTIES, &mut tmp)?;

            let new_prop_count = tmp.count_props() as usize;
            if new_prop_count != prop_count {
                // The number of properties has changed since the previous
                // request, so we'll retry.
                continue;
            }

            // Safety: We ensured the slice capacities above, and ensured
            // that the kernel has populated the number of ids we expected.
            unsafe {
                prop_ids.set_len(prop_count);
                prop_values.set_len(prop_count);
            };
            break (prop_ids, prop_values);
        };

        for (prop_id, value) in zip(prop_ids, prop_values) {
            let mut raw = ioctl::DrmModeGetProperty::zeroed();
            raw.prop_id = prop_id;
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut raw)?;
            // We can only produce a str from a property name that is all ASCII
            // characters, which is true for all property names used in the kernel
            // so far. We'll just ignore any properties that have non-ASCII names
            // for now, and then adjust this to do something else if an important
            // non-ASCII name shows up in a later kernel release.
            if !raw.name.is_ascii() {
                continue;
            }
            f(modeset::ObjectPropMeta::new(raw, &self), value);
        }

        Ok(())
    }

    /// Read information about the modesetting resources available for this device.
    ///
    /// The result includes ids for the available connectors, encoders, CRTCs,
    /// planes, and framebuffers.
    pub fn resources(&self) -> Result<modeset::CardResources, Error> {
        // The sets of resources can potentially change due to hotplug events
        // while we're producing this result, and so we need to keep retrying
        // until we get a consistent result.
        let mut ret = loop {
            let mut r = ioctl::DrmModeCardRes::zeroed();
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETRESOURCES, &mut r)?;
            let fb_count = r.count_fbs() as usize;
            let connector_count = r.count_connectors() as usize;
            let crtc_count = r.count_crtcs() as usize;
            let encoder_count = r.count_encoders() as usize;

            let mut fb_ids = vec_with_capacity::<FramebufferId>(fb_count)?;
            let mut connector_ids = vec_with_capacity::<ConnectorId>(connector_count)?;
            let mut crtc_ids = vec_with_capacity::<CrtcId>(crtc_count)?;
            let mut encoder_ids = vec_with_capacity::<EncoderId>(encoder_count)?;

            r = ioctl::DrmModeCardRes::zeroed();
            unsafe {
                r.set_fb_id_ptr(fb_ids.as_mut_ptr() as *mut u32, fb_count as u32);
                r.set_connector_id_ptr(
                    connector_ids.as_mut_ptr() as *mut u32,
                    connector_count as u32,
                );
                r.set_crtc_id_ptr(crtc_ids.as_mut_ptr() as *mut u32, crtc_count as u32);
                r.set_encoder_id_ptr(encoder_ids.as_mut_ptr() as *mut u32, encoder_count as u32);
            };

            self.ioctl(ioctl::DRM_IOCTL_MODE_GETRESOURCES, &mut r)?;
            // If any of the counts changed since our original call then the kernel
            // would not have populated the arrays and we'll need to retry and
            // hope that we don't collide with hotplugging next time.
            if r.count_fbs() as usize != fb_count {
                continue;
            }
            if r.count_connectors() as usize != connector_count {
                continue;
            }
            if r.count_crtcs() as usize != crtc_count {
                continue;
            }
            if r.count_encoders() as usize != encoder_count {
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
            break modeset::CardResources {
                fb_ids,
                connector_ids,
                crtc_ids,
                encoder_ids,
                plane_ids: Vec::new(),
                min_width: r.min_width,
                max_width: r.max_width,
                min_height: r.min_height,
                max_height: r.max_height,
            };
        };

        // The planes come from a different ioctl request so we'll deal
        // with those now too. Similar requirement to retry.
        loop {
            let mut tmp = ioctl::DrmModeGetPlaneRes::zeroed();
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETPLANERESOURCES, &mut tmp)?;

            let plane_count = tmp.count_planes() as usize;
            let mut plane_ids = vec_with_capacity::<modeset::PlaneId>(plane_count)?;
            unsafe { tmp.set_plane_id_ptr(plane_ids.as_mut_ptr() as *mut u32, plane_count as u32) };

            self.ioctl(ioctl::DRM_IOCTL_MODE_GETPLANERESOURCES, &mut tmp)?;
            if tmp.count_planes() as usize != plane_count {
                // Need to try again, then.
                continue;
            }

            // Safety: We ensured the slices capacity above, and ensured
            // that the kernel has populated the number of ids we expected.
            unsafe {
                plane_ids.set_len(plane_count);
            };
            ret.plane_ids = plane_ids;
            return Ok(ret);
        }
    }

    /// Read current state information for the connector with the given id.
    pub fn connector_state(
        &self,
        connector_id: ConnectorId,
    ) -> Result<modeset::ConnectorState, Error> {
        // Hotplug events can cause the state to change between our calls, so
        // we'll keep retrying until we get a consistent result.
        loop {
            let mut tmp = ioctl::DrmModeGetConnector::zeroed();
            tmp.connector_id = connector_id.0;
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETCONNECTOR, &mut tmp)?;

            let mode_count = tmp.count_modes();
            let encoder_count = tmp.count_encoders();
            let prop_count = tmp.count_props();

            let mut modes = vec_with_capacity::<ioctl::DrmModeInfo>(mode_count as usize)?;
            let mut ret_modes = vec_with_capacity::<ModeInfo>(mode_count as usize)?;
            let mut encoder_ids = vec_with_capacity::<u32>(encoder_count as usize)?;
            let mut prop_ids = vec_with_capacity::<u32>(prop_count as usize)?;
            let mut prop_values = vec_with_capacity::<u64>(prop_count as usize)?;
            let mut ret_props = vec_with_capacity::<ModeProp>(prop_count as usize)?;

            tmp = ioctl::DrmModeGetConnector::zeroed();
            tmp.connector_id = connector_id.0;
            unsafe {
                tmp.set_modes_ptr(modes.as_mut_ptr(), mode_count);
                tmp.set_encoders_ptr(encoder_ids.as_mut_ptr(), encoder_count);
                tmp.set_props_ptrs(prop_ids.as_mut_ptr(), prop_values.as_mut_ptr(), prop_count);
            };
            self.ioctl(ioctl::DRM_IOCTL_MODE_GETCONNECTOR, &mut tmp)?;

            if tmp.count_modes() != mode_count
                || tmp.count_props() != prop_count
                || tmp.count_encoders() != encoder_count
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
                core::iter::zip(prop_ids.iter().copied(), prop_values.iter().copied()).map(
                    |(prop_id, value)| ModeProp {
                        prop_id: modeset::PropertyId(prop_id),
                        value,
                    },
                ),
            );
            return Ok(modeset::ConnectorState {
                id: ConnectorId(tmp.connector_id),
                current_encoder_id: EncoderId(tmp.encoder_id),
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

    /// Read current state information for the encoder with the given id.
    pub fn encoder_state(&self, encoder_id: EncoderId) -> Result<modeset::EncoderState, Error> {
        let mut tmp = ioctl::DrmModeGetEncoder::zeroed();
        tmp.encoder_id = encoder_id.0;
        self.ioctl(ioctl::DRM_IOCTL_MODE_GETENCODER, &mut tmp)?;
        Ok(EncoderState {
            encoder_id: EncoderId(tmp.encoder_id),
            encoder_type: tmp.encoder_type,
            current_crtc_id: CrtcId(tmp.crtc_id),
            possible_crtcs: tmp.possible_crtcs,
            possible_clones: tmp.possible_clones,
        })
    }

    /// Read current state information for the CRTC with the given id.
    pub fn crtc_state(&self, crtc_id: CrtcId) -> Result<modeset::CrtcState, Error> {
        let mut tmp = ioctl::DrmModeCrtc::zeroed();
        tmp.crtc_id = crtc_id.0;
        self.ioctl(ioctl::DRM_IOCTL_MODE_GETCRTC, &mut tmp)?;
        Ok(tmp.into())
    }

    /// Read current state information for the plane with the given id.
    pub fn plane_state(&self, plane_id: PlaneId) -> Result<modeset::PlaneState, Error> {
        let mut tmp = ioctl::DrmModeGetPlane::zeroed();
        tmp.plane_id = plane_id.0;
        self.ioctl(ioctl::DRM_IOCTL_MODE_GETPLANE, &mut tmp)?;
        Ok(modeset::PlaneState {
            id: PlaneId(tmp.plane_id),
            crtc_id: CrtcId(tmp.crtc_id),
            fb_id: FramebufferId(tmp.fb_id),
            possible_crtcs: tmp.possible_crtcs,
            gamma_size: tmp.gamma_size,
        })
    }

    /// Attempt to commit an atomic modesetting request.
    ///
    /// Callers which intend to perform frequent modesetting, such as modesetting on
    /// every frame for double buffering, are encouraged to retain their
    /// [`modeset::AtomicRequest`] object and reset it to use again on a subsequent
    /// request if that request will involve a similar set of objects and properties,
    /// to minimize the need for reallocating the backing storage for the request
    /// on every frame.
    pub fn atomic_commit(
        &mut self,
        req: &modeset::AtomicRequest,
        flags: modeset::AtomicCommitFlags,
        user_data: u64,
    ) -> Result<(), Error> {
        let mut tmp = ioctl::DrmModeAtomic::zeroed();
        let mut raw_parts = req.for_ioctl_req();
        unsafe {
            tmp.set_ptrs(ioctl::DrmModeAtomicPtrs {
                count_objs: raw_parts.obj_ids.len() as u32,
                objs_ptr: raw_parts.obj_ids.as_mut_ptr(),
                count_props_ptr: raw_parts.obj_prop_counts.as_mut_ptr(),
                props_ptr: raw_parts.prop_ids.as_mut_ptr(),
                prop_values_ptr: raw_parts.prop_values.as_mut_ptr(),
            })
        };
        tmp.flags = flags.0;
        tmp.user_data = user_data;

        self.ioctl(ioctl::DRM_IOCTL_MODE_ATOMIC, &mut tmp)?;
        Ok(())
    }

    /// Send the given content to the kernel as a property blob, ready to use
    /// for assignment to a blob-typed object property.
    ///
    /// The returned [`modeset::BlobHandle`] must remain live long enough to
    /// be assigned to the target property. The blob object in the kernel
    /// will be destroyed when the blob handle is dropped.
    pub fn new_property_blob<'card, 'content>(
        &'card self,
        content: &'content [u8],
    ) -> Result<modeset::BlobHandle, Error> {
        let mut tmp = ioctl::DrmModeCreateBlob::zeroed();
        if content.len() > (u32::MAX as usize) {
            return Err(Error::Invalid);
        }
        unsafe { tmp.set_data(content.as_ptr(), content.len() as u32) };
        self.ioctl(ioctl::DRM_IOCTL_MODE_CREATEPROPBLOB, &mut tmp)?;
        Ok(modeset::BlobHandle {
            id: Some(BlobId(tmp.blob_id)),
            f: Arc::downgrade(&self.f),
        })
    }

    /// Reset the given CRTC to its default (zeroed) settings.
    pub fn reset_crtc(&mut self, crtc_id: u32) -> Result<modeset::CrtcState, Error> {
        let mut tmp = ioctl::DrmModeCrtc::zeroed();
        tmp.crtc_id = crtc_id;
        self.ioctl(ioctl::DRM_IOCTL_MODE_SETCRTC, &mut tmp)?;
        Ok(tmp.into())
    }

    /// Set the given CRTC to display the image from the given "dumb buffer",
    /// used for software rendering.
    pub fn set_crtc_dumb_buffer(
        &mut self,
        crtc_id: CrtcId,
        buf: &modeset::DumbBuffer,
        mode: &ModeInfo,
        conn_ids: &[ConnectorId],
    ) -> Result<modeset::CrtcState, Error> {
        let mut tmp = ioctl::DrmModeCrtc::zeroed();
        tmp.crtc_id = crtc_id.0;
        if conn_ids.len() > (u32::MAX as usize) {
            return Err(Error::Invalid);
        }
        unsafe {
            tmp.set_set_connectors_ptr(conn_ids.as_ptr() as *const u32, conn_ids.len() as u32)
        };
        tmp.fb_id = buf.fb_id.0;
        tmp.mode = mode.into();
        tmp.mode_valid = 1;

        self.ioctl(ioctl::DRM_IOCTL_MODE_SETCRTC, &mut tmp)?;
        Ok(tmp.into())
    }

    /// Use a page-flipping request to change the given CRTC to display the image
    /// from the given "dumb buffer".
    pub fn crtc_page_flip_dumb_buffer(
        &mut self,
        crtd_id: CrtcId,
        buf: &modeset::DumbBuffer,
        flags: modeset::PageFlipFlags,
    ) -> Result<(), Error> {
        let mut tmp = ioctl::DrmModeCrtcPageFlip::zeroed();
        tmp.crtc_id = crtd_id.0;
        tmp.fb_id = buf.fb_id.0;
        tmp.flags = flags.into();
        self.ioctl(ioctl::DRM_IOCTL_MODE_PAGE_FLIP, &mut tmp)?;
        Ok(())
    }

    /// Create a new "dumb buffer" that can be used for portable (hardware-agnostic)
    /// software rendering.
    pub fn create_dumb_buffer(
        &self,
        req: modeset::DumbBufferRequest,
    ) -> Result<modeset::DumbBuffer, Error> {
        let mut buf_req = ioctl::DrmModeCreateDumb::zeroed();
        buf_req.width = req.width;
        buf_req.height = req.height;
        buf_req.bpp = req.bpp;
        self.ioctl(ioctl::DRM_IOCTL_MODE_CREATE_DUMB, &mut buf_req)?;
        let buffer_handle = buf_req.handle;
        let mut cleanup_db = util::Cleanup::new(|| {
            let mut msg = crate::ioctl::DrmModeDestroyDumb::zeroed();
            msg.handle = buffer_handle;
            let _ = self.ioctl(crate::ioctl::DRM_IOCTL_MODE_DESTROY_DUMB, &mut msg);
        });

        let mut fb_req = ioctl::DrmModeFbCmd::zeroed();
        fb_req.width = buf_req.width;
        fb_req.height = buf_req.height;
        fb_req.bpp = buf_req.bpp;
        fb_req.depth = req.depth;
        fb_req.pitch = buf_req.pitch;
        fb_req.handle = buf_req.handle;
        self.ioctl(ioctl::DRM_IOCTL_MODE_ADDFB, &mut fb_req)?;
        let mut fb_id = fb_req.fb_id;
        let mut cleanup_fb = util::Cleanup::new(|| {
            let _ = self.ioctl(crate::ioctl::DRM_IOCTL_MODE_RMFB, &mut fb_id);
        });

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
        cleanup_fb.cancel();
        cleanup_db.cancel();
        Ok(modeset::DumbBuffer {
            width: buf_req.width,
            height: buf_req.height,
            bpp: buf_req.bpp,
            pitch: buf_req.pitch,
            ptr: buf_ptr as *mut u8,
            len: buf_req.size as usize,
            fb_id: FramebufferId(fb_req.fb_id),
            buffer_handle: BufferObjectId(buf_req.handle),
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

    /// Close the filehandle underlying the card object.
    #[inline]
    pub fn close(self) -> linux_io::result::Result<()> {
        let f = self.take_file()?;
        f.close()
    }

    /// Take the file from underneath this card object without closing it.
    pub fn take_file(self) -> linux_io::result::Result<linux_io::File<ioctl::DrmCardDevice>> {
        Arc::into_inner(self.f).ok_or(linux_io::result::EBUSY)
    }

    /// Borrow the file object that this card object wraps.
    #[inline(always)]
    pub fn borrow_file(&self) -> &linux_io::File<ioctl::DrmCardDevice> {
        self.f.as_ref()
    }

    /// Perform a direct ioctl request to the underlying card device filehandle.
    #[inline(always)]
    pub fn ioctl<'a, Req: IoctlReq<'a, ioctl::DrmCardDevice> + Copy>(
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

/// DRM API version information.
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

/// Enumeration of DRM device capabilities.
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

/// Enumeration of DRM client capabilities.
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
pub(crate) fn vec_with_capacity<T>(
    capacity: usize,
) -> Result<Vec<T>, alloc::collections::TryReserveError> {
    let mut ret = Vec::<T>::new();
    ret.try_reserve_exact(capacity)?;
    Ok(ret)
}
