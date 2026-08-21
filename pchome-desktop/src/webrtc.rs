//! WebRTC media stack integration (enabled by the `webrtc` feature).
//!
//! This module is the integration point for the `webrtc` crate. It is kept
//! behind a feature flag so the default desktop build does not require the
//! optional dependency. The real peer-connection wiring belongs here once the
//! `webrtc` crate is added under `[dependencies]` and this feature is enabled.

use anyhow::Result;

pub fn create_peer_connection() -> Result<()> {
    anyhow::bail!(
        "webrtc-rs integration is not compiled in; enable the `webrtc` feature \
         and add the `webrtc` crate dependency to build the real media stack"
    )
}
