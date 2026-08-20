use anyhow::Result;
use tokio::signal;

mod uinput;
mod pin;
mod pipewire;
mod encoder;
mod network;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    log::info!("PChome Desktop starting (stub)");

    // Start pin manager
    pin::start_pin_manager().await?;

    // Wait for ctrl+c
    signal::ctrl_c().await.expect("failed to listen for event");
    log::info!("Shutting down");
    Ok(())
}
