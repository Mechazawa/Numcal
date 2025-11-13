use embassy_executor::Spawner;
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{Peri, peripherals};
use embassy_time::{Duration, Timer};
use log::info;

/// Watchdog timeout in seconds (8 seconds)
/// If the watchdog isn't fed within this time, it will trigger a reboot to BOOTSEL
const WATCHDOG_TIMEOUT_SECS: u32 = 8;

/// How often to feed the watchdog (4 seconds - half the timeout for safety margin)
const WATCHDOG_FEED_INTERVAL: Duration = Duration::from_secs(4);

/// Magic values for RP2040 bootrom BOOTSEL mode (from Pico SDK)
/// These are written to watchdog scratch registers to signal the bootrom
/// to enter USB mass storage mode on the next reset
const BOOTSEL_MAGIC_SCRATCH5: u32 = 0xb007c0d3;
const BOOTSEL_MAGIC_SCRATCH6: u32 = 0;  // flags: 0 = BOOTSEL mode

#[embassy_executor::task]
async fn watchdog_task(mut watchdog: Watchdog) {
    info!("Watchdog enabled: timeout={}s, feed_interval={}s",
          WATCHDOG_TIMEOUT_SECS,
          WATCHDOG_FEED_INTERVAL.as_secs());

    loop {
        // Feed the watchdog to prevent reboot
        watchdog.feed();

        // Wait before next feed
        Timer::after(WATCHDOG_FEED_INTERVAL).await;
    }
}

/// Initialize the watchdog timer
///
/// The watchdog will automatically reboot the device to BOOTSEL mode if not fed
/// within the timeout period. This helps recover from crashes or hangs.
///
/// When the device hangs, the watchdog will timeout and reset the RP2040.
/// The bootrom will check scratch registers 5 and 6, see the magic values,
/// and enter USB mass storage mode for easy reflashing.
pub async fn init(spawner: &Spawner, watchdog_peripheral: Peri<'static, peripherals::WATCHDOG>) {
    // Create watchdog instance
    let mut watchdog = Watchdog::new(watchdog_peripheral);

    // Set scratch registers to tell bootrom to enter BOOTSEL mode on reset
    // This way, if the device crashes/hangs and the watchdog times out,
    // it will automatically enter BOOTSEL mode for easy recovery
    watchdog.set_scratch(5, BOOTSEL_MAGIC_SCRATCH5);
    watchdog.set_scratch(6, BOOTSEL_MAGIC_SCRATCH6);

    // Start the watchdog with the configured timeout
    watchdog.start(Duration::from_secs(WATCHDOG_TIMEOUT_SECS as u64));

    // Spawn the watchdog feeding task
    spawner.spawn(watchdog_task(watchdog).unwrap());
}
