# Rust API for Linux Direct Rendering Manager and Kernel Modesetting

The crate `linux-drm` wraps `linux-io` to provide more convenient access to
the Linux DRM/KMS API without depending on any C libraries.

This is currently experimental and the API is likely to change before becoming
stable in a 1.0 release as we explore different API designs.

In particular, the `ioctl` module currently exposes some IOCTL requests that
are unsound because they allow safe Rust to ask the kernel to write to
arbitrary pointers. Dealing with this will likely require some new features
in the underlying `linux-io` crate, but for now the focus is on designing
the higher-level API that wraps the raw IOCTL requests.

This crate also currently relies on some unstable features and is therefore
only usable on a nightly Rust toolchain.
