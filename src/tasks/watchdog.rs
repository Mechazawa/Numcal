use embassy_executor::Spawner;
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{Peri, peripherals};
use embassy_time::{Duration, Timer};
use log::info;

const WATCHDOG_TIMEOUT_SECS: u32 = 8;
const WATCHDOG_FEED_INTERVAL: Duration = Duration::from_secs(4);
const BOOTSEL_MAGIC_SCRATCH5: u32 = 0xb007c0d3;
const BOOTSEL_MAGIC_SCRATCH6: u32 = 0;  // flags: 0 = BOOTSEL mode

#[embassy_executor::task]
async fn watchdog_task(mut watchdog: Watchdog) {
    info!("Watchdog enabled: timeout={}s, feed_interval={}s",
          WATCHDOG_TIMEOUT_SECS,
          WATCHDOG_FEED_INTERVAL.as_secs());

    loop {
        watchdog.feed();

        Timer::after(WATCHDOG_FEED_INTERVAL).await;
    }
}

pub async fn init(spawner: &Spawner, watchdog_peripheral: Peri<'static, peripherals::WATCHDOG>) {
    let mut watchdog = Watchdog::new(watchdog_peripheral);

    watchdog.set_scratch(5, BOOTSEL_MAGIC_SCRATCH5);
    watchdog.set_scratch(6, BOOTSEL_MAGIC_SCRATCH6);

    watchdog.start(Duration::from_secs(u64::from(WATCHDOG_TIMEOUT_SECS)));

    spawner.spawn(watchdog_task(watchdog).unwrap());
}