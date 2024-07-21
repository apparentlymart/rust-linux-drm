pub mod raw;

extern crate alloc;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub enum DrmEvent {
    /// An event of a generic type that's defined for all DRM drivers.
    Generic(GenericDrmEvent),

    /// An event of a driver-specific type.
    Driver(UnsupportedDrmEvent),

    /// An event that is neither driver-specific nor recognized as a
    /// supported generic event type.
    ///
    /// This is included primarily for error-reporting purposes. A
    /// generic event type that's currently unsupported might become
    /// supported by an additional [`GenericDrmEvent`] variant in
    /// a future version, so callers that wish to continue working
    /// against future releases should not use this to actually handle
    /// any events beyond reporting that an event is unsupported.
    Unsupported(UnsupportedDrmEvent),
}

impl DrmEvent {
    pub fn from_raw(raw: &raw::DrmEvent) -> Self {
        if raw.hdr.typ >= 0x80000000 {
            let evt = UnsupportedDrmEvent::from_raw(raw);
            Self::Driver(evt)
        } else if let Ok(evt) = GenericDrmEvent::try_from_raw(raw) {
            Self::Generic(evt)
        } else {
            let evt = UnsupportedDrmEvent::from_raw(raw);
            Self::Unsupported(evt)
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum GenericDrmEvent {
    VBlank(DrmVblankEvent),
    FlipComplete(DrmVblankEvent),
    CrtcSequence(DrmCrtcSequenceEvent),
}

impl GenericDrmEvent {
    pub fn try_from_raw(raw: &raw::DrmEvent) -> Result<Self, ()> {
        match raw.hdr.typ {
            raw::DRM_EVENT_VBLANK => {
                // Safety: All bit patterns are defined for DrmEventVblank
                let body = unsafe { raw.body_as::<raw::DrmEventVblank>() }.ok_or(())?;
                Ok(Self::VBlank(body.into()))
            }
            raw::DRM_EVENT_FLIP_COMPLETE => {
                // Safety: All bit patterns are defined for DrmEventVblank
                let body = unsafe { raw.body_as::<raw::DrmEventVblank>() }.ok_or(())?;
                Ok(Self::FlipComplete(body.into()))
            }
            raw::DRM_EVENT_CRTC_SEQUENCE => {
                // Safety: All bit patterns are defined for DrmEventCrtcSequence
                let body = unsafe { raw.body_as::<raw::DrmEventCrtcSequence>() }.ok_or(())?;
                Ok(Self::CrtcSequence(body.into()))
            }
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DrmVblankEvent {
    pub user_data: u64,
    pub tv_sec: u32,
    pub tv_usec: u32,
    pub sequence: u32,
    pub crtc_id: u32,
}

impl From<&raw::DrmEventVblank> for DrmVblankEvent {
    fn from(value: &raw::DrmEventVblank) -> Self {
        Self {
            user_data: value.user_data,
            tv_sec: value.tv_sec,
            tv_usec: value.tv_usec,
            sequence: value.sequence,
            crtc_id: value.crtc_id,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DrmCrtcSequenceEvent {
    pub user_data: u64,
    pub time_ns: i64,
    pub sequence: u64,
}

impl From<&raw::DrmEventCrtcSequence> for DrmCrtcSequenceEvent {
    fn from(value: &raw::DrmEventCrtcSequence) -> Self {
        Self {
            user_data: value.user_data,
            time_ns: value.time_ns,
            sequence: value.sequence,
        }
    }
}

/// Raw owned representation of a DRM event of a type that this
/// crate doesn't directly support.
#[derive(Debug, Clone)]
pub struct UnsupportedDrmEvent {
    typ: u32,
    body: Vec<u8>,
}

impl UnsupportedDrmEvent {
    pub fn from_raw(raw: &raw::DrmEvent) -> Self {
        let typ = raw.hdr.typ;
        let body = raw.body_bytes().to_vec();
        Self { typ, body }
    }

    #[inline(always)]
    pub fn typ(&self) -> u32 {
        self.typ
    }

    #[inline(always)]
    pub fn body_len(&self) -> usize {
        self.body.len()
    }

    /// Get the body of the event as a raw byte slice.
    #[inline(always)]
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    /// Get a pointer to the event body that interprets it as a
    /// value of `T`.
    ///
    /// Dereferencing the returned pointer is undefined behavior
    /// unless the body is long enough to contain a `T` and
    /// contains a valid representation of `T`.
    #[inline(always)]
    pub fn body_ptr<T: Sized>(&self) -> *const T {
        &self.body as *const _ as *const T
    }

    /// Get a reference to the body interpreted as type `T`
    /// only if the body is at least long enough to fit
    /// a value of that type.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the raw body is a valid
    /// representation of `T`. If all bit patterns are
    /// valid representations of `T` then this is always
    /// safe but the result might still be nonsense.
    pub unsafe fn body_as<T>(&self) -> Option<&T> {
        let min_size = core::mem::size_of::<T>();
        if self.body_len() < min_size {
            return None;
        }
        Some(self.body_as_unchecked::<T>())
    }

    /// Returns a reference to the body interpreted as type `T`
    /// without checking whether the header's indicated length
    /// is long enough for that type.
    ///
    /// # Safety
    ///
    /// Caller must ensure that there's enough valid memory after
    /// the event header for a `T` and that the data there is
    /// a valid representation of `T`.
    #[inline(always)]
    pub unsafe fn body_as_unchecked<T>(&self) -> &T {
        let ptr = self.body_ptr::<T>();
        &*ptr
    }
}
