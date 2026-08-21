//! PipeWire integration via the `pipewire` crate (enabled by the `pipewire-rs`
//! feature).
//!
//! The hand-rolled capture path in `pipewire.rs` is a placeholder. The real
//! DMA-BUF/stream capture using the `pipewire` crate should live here, behind
//! this feature flag, so the default build does not pull in the optional
//! dependency until it is wired up.

use anyhow::Result;

pub fn start_pipewire_capture() -> Result<()> {
    anyhow::bail!(
        "pipewire-rs integration is not compiled in; enable the `pipewire-rs` \
         feature and add the `pipewire` crate dependency to build real capture"
    )
}
