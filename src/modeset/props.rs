use alloc::boxed::Box;
use alloc::sync::Weak;
use alloc::vec::Vec;

use crate::ioctl::DrmCardDevice;

use super::{BlobId, PropertyId};

#[derive(Debug)]
pub struct ModeProp {
    pub prop_id: PropertyId,
    pub value: u64,
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
    pub fn property_id(&self) -> PropertyId {
        PropertyId(self.raw.prop_id)
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
        if self.raw.count_values() != 2 {
            return Err(crate::Error::RemoteFailure);
        }

        let mut pair = [0_u64; 2];

        let mut tmp = crate::ioctl::DrmModeGetProperty::zeroed();
        tmp.prop_id = self.raw.prop_id;
        unsafe { tmp.set_values_ptr(&mut pair as *mut u64, 2) };
        self.card
            .ioctl(crate::ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut tmp)
            .unwrap();

        if tmp.count_values() != 2 {
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
        let mut count = self.raw.count_values() as usize;
        if count == 0 {
            return Ok(Vec::new());
        }
        loop {
            let mut values = crate::vec_with_capacity::<u64>(count)?;

            let mut tmp = crate::ioctl::DrmModeGetProperty::zeroed();
            tmp.prop_id = self.raw.prop_id;
            unsafe { tmp.set_values_ptr(values.as_mut_ptr(), count as u32) };

            self.card
                .ioctl(crate::ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut tmp)
                .unwrap();

            let new_count = tmp.count_values() as usize;
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
            return Ok(Vec::new());
        }

        let mut count = self.raw.count_enum_blobs() as usize;
        loop {
            // Safety: The following relies on ObjectPropEnumMember having identical
            // layout to ioctl::DrmModePropertyEnum, which we ensure by marking
            // it as repr(transparent).
            let mut members = crate::vec_with_capacity::<ObjectPropEnumMember>(count)?;

            let mut tmp = crate::ioctl::DrmModeGetProperty::zeroed();
            tmp.prop_id = self.raw.prop_id;
            unsafe {
                tmp.set_enum_blob_ptr(
                    members.as_mut_ptr() as *mut crate::ioctl::DrmModePropertyEnum,
                    count as u32,
                )
            };

            self.card
                .ioctl(crate::ioctl::DRM_IOCTL_MODE_GETPROPERTY, &mut tmp)
                .unwrap();

            let new_count = tmp.count_enum_blobs() as usize;
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

/// Trait implemented by types that can be borrowed as raw property values.
///
/// For types that represent references to other objects already known by
/// the kernel, such as property blobs, the caller must keep the original
/// object live for as long as the result is being used in requests to the
/// kernel.
pub trait AsRawPropertyValue {
    fn as_raw_property_value(&self) -> u64;
}

/// Trait implemented by types that can be converted into raw property values
/// while transferring ownership.
///
/// This is an extension of [`AsRawPropertyValue`] for situations where the
/// recipient is taking ownership of the implementing object, so that the
/// object can be kept live long enough to use its raw representation.
pub trait IntoRawPropertyValue: AsRawPropertyValue {
    /// Return the raw `u64` representation to send to the kernel along with
    /// an optional object that needs to be kept live in order for that
    /// raw result to remain valid.
    ///
    /// The first result should typically be the same as would be returned
    /// from [`AsRawPropertyValue::as_raw_property_value`].
    fn into_raw_property_value(self) -> (u64, Option<Box<dyn core::any::Any>>);
}

macro_rules! trivial_as_property_value {
    ($t:ty) => {
        impl AsRawPropertyValue for $t {
            #[inline(always)]
            fn as_raw_property_value(&self) -> u64 {
                *self as u64
            }
        }
        impl IntoRawPropertyValue for $t {
            #[inline(always)]
            fn into_raw_property_value(self) -> (u64, Option<Box<dyn core::any::Any>>) {
                (self as u64, None)
            }
        }
    };
}

trivial_as_property_value!(u64);
trivial_as_property_value!(u32);
trivial_as_property_value!(u16);
trivial_as_property_value!(u8);
trivial_as_property_value!(usize);
trivial_as_property_value!(i64);
trivial_as_property_value!(i32);
trivial_as_property_value!(i16);
trivial_as_property_value!(i8);
trivial_as_property_value!(isize);
trivial_as_property_value!(bool);

/// A handle for a live property blob.
///
/// The [`Drop`] implementation for this type destroys the
#[derive(Debug)]
pub struct BlobHandle {
    pub(crate) id: Option<BlobId>,
    pub(crate) f: Weak<linux_io::File<DrmCardDevice>>,
}

impl<'card> BlobHandle {
    #[inline(always)]
    pub const fn id(&self) -> BlobId {
        let Some(ret) = self.id else {
            unreachable!();
        };
        ret
    }

    /// Consume the handle and destroy the underlying blob in the kernel.
    #[inline(always)]
    pub fn destroy(mut self) -> Result<(), crate::result::Error> {
        self.destroy_internal()
    }

    #[inline]
    fn destroy_internal(&mut self) -> Result<(), crate::result::Error> {
        if let Some(f) = self.f.upgrade() {
            if let Some(blob_id) = self.id.take() {
                let mut tmp = crate::ioctl::DrmModeDestroyBlob { blob_id: blob_id.0 };
                crate::drm_ioctl(&f, crate::ioctl::DRM_IOCTL_MODE_DESTROYPROPBLOB, &mut tmp)?;
            }
        }
        Ok(())
    }
}

impl Drop for BlobHandle {
    #[inline(always)]
    fn drop(&mut self) {
        let _ = self.destroy_internal();
    }
}

impl AsRawPropertyValue for BlobHandle {
    fn as_raw_property_value(&self) -> u64 {
        self.id().0 as u64
    }
}

impl IntoRawPropertyValue for BlobHandle {
    fn into_raw_property_value(self) -> (u64, Option<Box<dyn core::any::Any>>) {
        (self.id().0 as u64, Some(Box::new(self)))
    }
}
