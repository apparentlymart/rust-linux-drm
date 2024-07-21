/// A raw DRM event.
///
/// This is a dynamically-sized type because DRM returns events of varying
/// sizes. DRM events typically come from reading the file descriptor
/// associated with a "card" device.
///
/// If you have a byte slice you've read from a card device then you can
/// use [`DrmEvent::from_bytes`] to safely build a `DrmEvent` from the
/// first event, if any.
#[repr(C)]
pub struct DrmEvent {
    pub hdr: DrmEventHeader,
    pub body: [u8],
}

impl DrmEvent {
    const HEADER_LEN: usize = core::mem::size_of::<DrmEventHeader>();

    /// Given a reference to the header part of a valid event,
    /// reinterprets it into a full [`DrmEvent`] object.
    ///
    /// # Safety
    ///
    /// The given reference must be to a valid header that is
    /// at the start of an event object whose length matches that
    /// given in the header. The result is a wide pointer
    /// that tracks the body length, which safe Rust cannot
    /// modify and so can be relied on by safe methods of
    /// [`DrmEvent`] while the header length cannot.
    pub unsafe fn from_event_header<'a>(hdr: &'a DrmEventHeader) -> &'a DrmEvent {
        let ptr = hdr as *const _ as *const ();
        let ptr = core::ptr::from_raw_parts(ptr, hdr.len as usize - Self::HEADER_LEN);
        &*ptr
    }

    /// Given a byte slice that contains zero or more DRM
    /// events, obtain the first event and a slice of the remaining
    /// bytes, or `None` if there aren't enough bytes left to extract
    /// even one event.
    ///
    /// The returned event does not necessarily have valid
    /// content. The only checking done by this function is
    /// that there are enough bytes in the slice for the
    /// length claimed in the header field.
    pub fn from_bytes<'a>(buf: &'a [u8]) -> Option<(&'a DrmEvent, &'a [u8])> {
        if buf.len() < Self::HEADER_LEN {
            return None;
        }
        let hdr_bytes = &buf[0..Self::HEADER_LEN];
        // Safety: We checked above that we have at least enough bytes for
        // an event header, and all bit patterns are defined values for
        // DrmEventHeader.
        let hdr = unsafe { &*(hdr_bytes.as_ptr() as *const DrmEventHeader) };
        let claimed_len = hdr.len as usize;
        if buf.len() < claimed_len {
            // The header thinks there are more bytes in this event
            // than we have left in our slice, so clearly something
            // has gone wrong here but we'll treat it as if there
            // aren't any more events.
            return None;
        }
        // Safety: We've checked that the header isn't asking for more
        // bytes than we have.
        let ret = unsafe { Self::from_event_header(hdr) };
        Some((ret, &buf[claimed_len..]))
    }

    /// Get the length of the body in bytes.
    ///
    /// This is based on the information stored in the wide pointer
    /// underlying `self`, and so ignores the length given in
    /// the header.
    pub fn body_len(&self) -> usize {
        let ptr = self as *const DrmEvent;
        let (_, body_len) = ptr.to_raw_parts();
        body_len
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

/// Raw DRM event header.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmEventHeader {
    pub typ: u32,
    pub len: u32,
}

/// Vertical blanking event.
///
/// This event is sent in response to `DRM_IOCTL_WAIT_VBLANK` with the
/// `DRM_VBLANK_EVENT` flag set.
///
/// The event body type is [`DrmEventVblank`].
pub const DRM_EVENT_VBLANK: u32 = 0x01;

/// Page-flip completion event.
///
/// This event is sent in response to an atomic commit or legacy page-flip with
/// the `DRM_MODE_PAGE_FLIP_EVENT` flag set.
///
/// The event body type is [`DrmEventVblank`].
pub const DRM_EVENT_FLIP_COMPLETE: u32 = 0x02;

/// CRTC sequence event.
///
/// This event is sent in response to `DRM_IOCTL_CRTC_QUEUE_SEQUENCE`.
///
/// The event body type is [`DrmEventCrtcSequence`].
pub const DRM_EVENT_CRTC_SEQUENCE: u32 = 0x03;

/// The body of a [`DRM_EVENT_VBLANK`] event.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmEventVblank {
    pub user_data: u64,
    pub tv_sec: u32,
    pub tv_usec: u32,
    pub sequence: u32,
    pub crtc_id: u32, // always zero in older kernels that don't support this
}

/// The body of a [`DRM_EVENT_CRTC_SEQUENCE`] event.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmEventCrtcSequence {
    pub user_data: u64,
    pub time_ns: i64,
    pub sequence: u64,
}

pub fn events_from_bytes<'a>(buf: &'a [u8]) -> impl Iterator<Item = &'a DrmEvent> + 'a {
    DrmEventsFromBytes { remain: buf }
}

struct DrmEventsFromBytes<'a> {
    remain: &'a [u8],
}

impl<'a> Iterator for DrmEventsFromBytes<'a> {
    type Item = &'a DrmEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let (ret, remain) = DrmEvent::from_bytes(self.remain)?;
        self.remain = remain;
        Some(ret)
    }
}
