use anyhow::Result;
use rand::RngCore;
use tokio::time::{sleep, Duration};

pub async fn start_pin_manager() -> Result<()> {
    // stub: generate a single 6-digit PIN and log it
    let mut rng = rand::rngs::OsRng;
    let pin = rng.next_u32() % 1_000_000;
    log::info!("Generated PIN: {:06}", pin);

    // keep alive for TTL (300s) in background
    tokio::spawn(async move {
        sleep(Duration::from_secs(300)).await;
        log::info!("PIN TTL expired (stub)");
    });

    Ok(())
}
