use core::ffi::c_int as int;
use core::ffi::c_ulong as ulong;
use core::mem::MaybeUninit;

use linux_io::fd::ioctl::FromIoctlResult;
use linux_io::fd::ioctl::IoctlReq;
use linux_io::fd::ioctl::{ioctl_no_arg, IoDevice, IoctlReqNoArgs};
use linux_unsafe::args::AsRawV;

pub struct DrmCardDevice;

impl IoDevice for DrmCardDevice {}

const DRM_IOCTL_BASE: u64 = 100;

#[allow(non_snake_case)]
const fn _IO(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IO(DRM_IOCTL_BASE, nr)
}

#[allow(non_snake_case)]
const fn _IOW<T>(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IOW(DRM_IOCTL_BASE, nr, core::mem::size_of::<T>() as u64)
}

#[allow(non_snake_case)]
const fn _IOR<T>(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IOR(DRM_IOCTL_BASE, nr, core::mem::size_of::<T>() as u64)
}

#[allow(non_snake_case)]
const fn _IOWR<T>(nr: ulong) -> ulong {
    linux_io::fd::ioctl::_IOWR(DRM_IOCTL_BASE, nr, core::mem::size_of::<T>() as u64)
}

#[doc(hidden)]
#[repr(transparent)]
pub struct IoctlReqWriteRead<Device: IoDevice, Arg, Result = int>
where
    *const Arg: AsRawV,
{
    request: ulong,
    _phantom: core::marker::PhantomData<(Device, Arg, Result)>,
}

unsafe impl<'a, Arg, Result> IoctlReq<'a, DrmCardDevice>
    for IoctlReqWriteRead<DrmCardDevice, Arg, Result>
where
    *const Arg: AsRawV,
    Arg: 'a,
    Result: 'a + FromIoctlResult<int>,
{
    type ExtArg = &'a mut Arg;
    type TempMem = ();
    type RawArg = *mut Arg;
    type Result = Result;

    #[inline(always)]
    fn prepare_ioctl_args(
        &self,
        arg: &Self::ExtArg,
        _: &mut MaybeUninit<Self::TempMem>,
    ) -> (ulong, *mut Arg) {
        (self.request, (*arg) as *const Arg as *mut Arg)
    }

    #[inline(always)]
    fn prepare_ioctl_result(
        &self,
        ret: int,
        _: &Self::ExtArg,
        _: &MaybeUninit<Self::TempMem>,
    ) -> Self::Result {
        Result::from_ioctl_result(&ret)
    }
}

const unsafe fn ioctl_writeread<Device, Arg, Result>(
    request: ulong,
) -> IoctlReqWriteRead<Device, Arg, Result>
where
    *mut Result: AsRawV,
    Device: IoDevice,
    Result: Copy,
{
    IoctlReqWriteRead::<Device, Arg, Result> {
        request,
        _phantom: core::marker::PhantomData,
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmVersion {
    pub version_major: int,
    pub version_minor: int,
    pub version_patchlevel: int,
    pub name_len: usize,
    pub name: *const i8,
    pub date_len: usize,
    pub date: *const i8,
    pub desc_len: usize,
    pub desc: *const i8,
}

impl DrmVersion {
    #[inline(always)]
    pub const fn zeroed() -> Self {
        // Safety: All of the field types in DrmVersion
        // treat all-zeroes as a valid bit pattern.
        unsafe { core::mem::zeroed() }
    }
}

pub const DRM_IOCTL_VERSION: IoctlReqWriteRead<DrmCardDevice, DrmVersion, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmVersion>(0x00)) };

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DrmSetVersion {
    pub drm_di_major: int,
    pub drm_di_minor: int,
    pub drm_dd_major: int,
    pub drm_dd_minor: int,
}

impl DrmSetVersion {
    #[inline(always)]
    pub const fn zeroed() -> Self {
        // Safety: All of the field types in DrmVersion
        // treat all-zeroes as a valid bit pattern.
        unsafe { core::mem::zeroed() }
    }
}

pub const DRM_IOCTL_SET_VERSION: IoctlReqWriteRead<DrmCardDevice, DrmSetVersion, int> =
    unsafe { ioctl_writeread(_IOWR::<DrmSetVersion>(0x07)) };

pub const DRM_IOCTL_SET_MASTER: IoctlReqNoArgs<DrmCardDevice, int> =
    unsafe { ioctl_no_arg(_IO(0x1e)) };

pub const DRM_IOCTL_DROP_MASTER: IoctlReqNoArgs<DrmCardDevice, int> =
    unsafe { ioctl_no_arg(_IO(0x1f)) };
