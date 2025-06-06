use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::iter;
use core::ops::BitOr;

use super::{IntoRawPropertyValue, ObjectId, PropertyId};

/// An atomic modesetting commit request.
#[derive(Debug)]
pub struct AtomicRequest {
    objs: BTreeMap<u32, AtomicRequestObj>,
    total_props: u32,

    // When a property is set to something that needs to be dropped
    // after use, we remember it here so that its drop can be deferred
    // until the atomic request is dropped or reset.
    drops: Vec<Box<dyn core::any::Any>>,
}

#[derive(Debug)]
struct AtomicRequestObj {
    prop_ids: Vec<u32>,
    prop_values: Vec<u64>,
}

impl AtomicRequest {
    pub fn new() -> Self {
        Self {
            objs: BTreeMap::new(),
            total_props: 0,
            drops: Vec::new(),
        }
    }

    /// Clear any previously-set properties, returning the object to empty.
    ///
    /// This function does, however, retain any allocated capacity for
    /// property-set requests for existing objects, so callers can avoid
    /// making lots of dynamic memory allocations on every change by
    /// retaining and reusing a single request object. This optimization
    /// will only be productive if the object is reused to set a similar
    /// set of properties on a similar set of objects.
    pub fn reset(&mut self) {
        // The idea here is to retain all of our existing BTreeMap elements,
        // but to reduce the length of the nested vectors to zero. This
        // therefore means that subsequent uses of the request with similar
        // object ids and a similar number of properties per object can
        // avoid the need for lots of reallocation.
        for (_, v) in self.objs.iter_mut() {
            v.prop_ids.truncate(0);
            v.prop_values.truncate(0);
        }
        self.total_props = 0;
        self.drops.truncate(0);
    }

    pub fn set_property(
        &mut self,
        obj_id: impl Into<ObjectId>,
        prop_id: PropertyId,
        value: impl IntoRawPropertyValue,
    ) {
        fn set(req: &mut AtomicRequest, obj_id: ObjectId, prop_id: PropertyId, value: u64) {
            let (_, obj_id) = obj_id.as_raw_type_and_id();
            let obj = req.objs.entry(obj_id).or_insert_with(|| AtomicRequestObj {
                prop_ids: Vec::new(),
                prop_values: Vec::new(),
            });

            // We'll reserve first to make sure that running out of memory can't
            // cause these two vecs to end up with different lengths when we're done.
            obj.prop_ids.reserve(1);
            obj.prop_values.reserve(1);

            obj.prop_ids.push(prop_id.0);
            obj.prop_values.push(value);
            req.total_props += 1; // panics if request contains more than u32::MAX total properties
            if req.objs.len() > (u32::MAX as usize) {
                panic!("too many distinct objects in request");
            }
        }
        let (raw_v, drop) = value.into_raw_property_value();
        if let Some(drop) = drop {
            self.drops.push(drop);
        }
        set(self, obj_id.into(), prop_id, raw_v);
    }

    pub(crate) fn for_ioctl_req(&self) -> AtomicRequestRawParts {
        let obj_count = self.objs.len();
        let mut obj_ids = Vec::<u32>::with_capacity(obj_count);
        let mut obj_prop_counts = Vec::<u32>::with_capacity(obj_count);
        let total_prop_count = self.total_props as usize;
        let mut prop_ids = Vec::<u32>::with_capacity(total_prop_count);
        let mut prop_values = Vec::<u64>::with_capacity(total_prop_count);

        for (obj_id, obj) in self.objs.iter() {
            obj_ids.push(*obj_id);
            obj_prop_counts.push(obj.prop_ids.len() as u32);

            for (prop_id, value) in iter::zip(
                obj.prop_ids.iter().copied(),
                obj.prop_values.iter().copied(),
            ) {
                prop_ids.push(prop_id);
                prop_values.push(value);
            }
        }

        AtomicRequestRawParts {
            obj_ids,
            obj_prop_counts,
            prop_ids,
            prop_values,
        }
    }
}

pub(crate) struct AtomicRequestRawParts {
    pub(crate) obj_ids: Vec<u32>,
    pub(crate) obj_prop_counts: Vec<u32>,
    pub(crate) prop_ids: Vec<u32>,
    pub(crate) prop_values: Vec<u64>,
}

pub struct AtomicCommitFlags(pub(crate) u32);

impl AtomicCommitFlags {
    pub const NONE: Self = Self(0);
    pub const TEST_ONLY: Self = Self(crate::ioctl::DRM_MODE_ATOMIC_TEST_ONLY);
    pub const NONBLOCK: Self = Self(crate::ioctl::DRM_MODE_ATOMIC_NONBLOCK);
    pub const ALLOW_MODESET: Self = Self(crate::ioctl::DRM_MODE_ATOMIC_ALLOW_MODESET);
    pub const PAGE_FLIP_EVENT: Self = Self(crate::ioctl::DRM_MODE_PAGE_FLIP_EVENT);
    pub const ASYNC: Self = Self(crate::ioctl::DRM_MODE_PAGE_FLIP_ASYNC);
}

impl BitOr for AtomicCommitFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
