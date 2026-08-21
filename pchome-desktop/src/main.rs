use anyhow::Result;
use crate::encoder::Encoder;
use crate::metrics::ENCODE_LATENCY;
use crate::network::ConnectionManager;
use crate::pin::PinManager;
use crate::pipewire::init_capture;
use crate::uinput::UInputDevice;
use tokio::signal;

mod metrics;
#[cfg(target_family = "unix")]
mod uinput;
mod pin;
#[cfg(target_family = "unix")]
mod pipewire;
mod encoder;
mod network;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    log::info!("PChome Desktop starting");

    let pin_manager = PinManager::new();
    pin_manager.start().await?;
    let _pin = pin_manager.generate_and_register().await?;

    let _encoder = Encoder::init_encoder()?;
    #[cfg(target_family = "unix")]
    let _capture_stream = init_capture()?;
    #[cfg(target_family = "unix")]
    let _uinput = UInputDevice::open("/dev/uinput", "pchome-virtual-device")?;

    let connection_manager = ConnectionManager::new("ws://localhost:8080/ws");
    let _ = connection_manager.connect().await;

    log::info!("PChome Desktop initialized successfully");

    signal::ctrl_c().await.expect("failed to listen for event");
    log::info!("Shutting down");
    Ok(())
}
