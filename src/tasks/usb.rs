use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::{bind_interrupts, Peri};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcState};
use embassy_usb::{Builder, Config as UsbConfig};
use static_cell::StaticCell;

// Interrupts
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// Buffers
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<CdcState> = StaticCell::new();

pub async fn init(spawner: &Spawner, usb_peripheral: Peri<'static, USB>) {
    let mut config = UsbConfig::new(0x16c0, 0x27dd);
    config.manufacturer = Some("Mechazawa");
    config.product = Some("NumCal");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let mut builder = Builder::new(
        Driver::new(usb_peripheral, Irqs),
        config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [], // no msos descriptors
        CONTROL_BUF.init([0; 64]),
    );

    let cdc = CdcAcmClass::new(&mut builder, CDC_STATE.init(CdcState::new()), 64);
    let usb = builder.build();

    // Spawn device tasks
    spawner.spawn(usb_device_task(usb).unwrap());
    spawner.spawn(logger_task(cdc).unwrap());
}

#[embassy_executor::task]
async fn usb_device_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}

#[embassy_executor::task]
async fn logger_task(class: CdcAcmClass<'static, Driver<'static, USB>>) {
    embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, class).await;
}