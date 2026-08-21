use anyhow::Result;
use crate::encoder::Encoder;
use crate::metrics::ENCODE_LATENCY;
use crate::network::ConnectionManager;
use crate::pin::PinManager;
use crate::pipewire::init_capture;
use crate::uinput::UInputDevice;
use tokio::signal;

mod metrics;
mod pixelformat;
#[cfg(target_family = "unix")]
mod uinput;
mod pin;
#[cfg(target_family = "unix")]
mod pipewire;
mod encoder;
mod network;
mod config;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    log::info!("PChome Desktop starting");

    let config = config::Config::from_env().unwrap_or_default();
    log::info!("Loaded config: {:?}", config);

    let pin_manager = PinManager::new();
    pin_manager.start().await?;
    let pin = pin_manager.generate_and_register().await?;
    log::info!("Registered PIN: {:06}", pin);

    let _encoder = Encoder::init_encoder()?;
    #[cfg(target_family = "unix")]
    let _capture_stream = init_capture()?;
    #[cfg(target_family = "unix")]
    let _uinput = UInputDevice::open("/dev/uinput", "pchome-virtual-device")?;

    let connection_manager = ConnectionManager::new(config.signal_url.clone());
    if let Err(e) = connection_manager.connect().await {
        log::warn!("Failed to connect to signal server: {}", e);
    }

    metrics::serve(config.metrics_addr);

    log::info!("PChome Desktop initialized successfully");

    signal::ctrl_c().await.expect("failed to listen for event");
    log::info!("Shutting down");
    Ok(())
}
