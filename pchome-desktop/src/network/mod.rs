pub mod socket;
pub mod webrtc;

pub async fn init_network() -> anyhow::Result<()> {
    log::info!("network module init stub");
    Ok(())
}
