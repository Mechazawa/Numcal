#![no_std]
#![no_main]

mod tasks;

use core::str::FromStr;
use embassy_executor::Spawner;
use embassy_time::Timer;
use embassy_rp::config::Config;
use {defmt_rtt as _, panic_probe as _};
use log::info;

use tasks::init_usb;
use tasks::init_display;
use crate::tasks::DISPLAY_CHANNEL;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Config::default());

    init_usb(
        &spawner,
        peripherals.USB
    ).await;

    init_display(
        &spawner,
        peripherals.SPI1,
        peripherals.PIN_14,
        peripherals.PIN_15,
        peripherals.PIN_13,
        peripherals.PIN_3,
        peripherals.PIN_10,
    ).await;

    // Wait for USB to enumerate and logger to be ready
    // todo add this to the init_usb with a timeout
    Timer::after_secs(2).await;

    // Draw text on display
    DISPLAY_CHANNEL.try_send(heapless::String::from_str("Hello World!").unwrap()).unwrap();

    // Wait a bit so the message can be seen
    Timer::after_secs(3).await;

    info!("Rebooting to BOOTSEL mode...");

    // Give time for the log message to be transmitted
    Timer::after_millis(100).await;

    // Reboot into bootsel mode
    embassy_rp::rom_data::reset_to_usb_boot(0, 0);

    // Should never reach here
    loop {
        Timer::after_secs(1).await;
    }
}
