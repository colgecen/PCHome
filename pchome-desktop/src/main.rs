use anyhow::Result;
use pchome_desktop::encoder::Encoder;
use pchome_desktop::metrics::ENCODE_LATENCY;
use pchome_desktop::network::ConnectionManager;
use pchome_desktop::pin::PinManager;
use pchome_desktop::pipewire::init_capture;
use pchome_desktop::uinput::UInputDevice;
use tokio::signal;

mod metrics;
mod uinput;
mod pin;
mod pipewire;
mod encoder;
mod network;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    tracing_subscriber::fmt::init();
    log::info!("PChome Desktop starting");

    let pin_manager = PinManager::new();
    pin_manager.start().await?;
    let _pin = pin_manager.generate_and_register().await?;

    let _encoder = Encoder::init_encoder()?;
    let _capture_stream = init_capture()?;

    let _uinput = UInputDevice::open("/dev/uinput", "pchome-virtual-device")?;

    let connection_manager = ConnectionManager::new("ws://localhost:8080/ws");
    let _ = connection_manager.connect().await;

    log::info!("PChome Desktop initialized successfully");

    signal::ctrl_c().await.expect("failed to listen for event");
    log::info!("Shutting down");
    Ok(())
}
