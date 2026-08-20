// Minimal stub for /dev/uinput wrapper

pub fn init() -> anyhow::Result<()> {
    // Real implementation should open /dev/uinput and configure device
    log::info!("uinput stub initialized");
    Ok(())
}
