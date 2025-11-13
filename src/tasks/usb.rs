use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::{bind_interrupts, Peri};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcState};
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State as HidState, Config as HidConfig, HidSubclass, HidBootProtocol};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config as UsbConfig};
use portable_atomic::{AtomicU8, Ordering};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

/// Events that can be sent to the HID task
#[derive(Clone, Copy, Debug)]
pub enum HidEvent {
    Reset,
    SetKey(u8),
    SetModifier(u8),
    ReleaseKey(u8),
    ReleaseModifier(u8),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum KeyboardLed {
    NumLock = 1 << 0,
    CapsLock = 1 << 1,
    ScrollLock = 1 << 2,
    Compose = 1 << 3,
    Kana = 1 << 4,
}

/// Channel for sending HID events to the HID task
pub static HID_CHANNEL: Channel<ThreadModeRawMutex, HidEvent, 32> = Channel::new();
pub static LED_STATE: LedState = LedState::new();

// Interrupts
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// Buffers
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<CdcState> = StaticCell::new();
static HID_STATE: StaticCell<HidState> = StaticCell::new();
static REQUEST_HANDLER_CELL: StaticCell<HidRequestHandler> = StaticCell::new();

// HID Request Handler
struct HidRequestHandler {}

impl RequestHandler for HidRequestHandler {
    fn get_report(&mut self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        // Host is asking for current report state
        // Not typically needed for keyboards
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        // Host is sending us data - typically LED status for keyboards
        log::debug!("HID set_report: id={id:?}, len={}", data.len());

        if data.is_empty() {
            OutResponse::Rejected
        } else {
            LED_STATE.store(data[0]);

            OutResponse::Accepted
        }
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        log::debug!("HID get_idle: id={id:?}");
        None
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        log::debug!("HID set_idle: id={id:?}, duration={dur}ms");
    }
}

pub struct LedState(AtomicU8);

impl LedState {
    const fn new() -> Self {
        Self(AtomicU8::new(0))
    }

    fn store(&self, state: u8) {
        self.0.store(state, Ordering::Relaxed);
    }

    pub fn test(&self, led: KeyboardLed) -> bool {
        self.0.load(Ordering::Relaxed) & led as u8 > 0
    }
}

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

    // Create CDC-ACM class for logging
    let cdc = CdcAcmClass::new(&mut builder, CDC_STATE.init(CdcState::new()), 64);

    // Create HID class for keyboard
    let hid_config = HidConfig {
        report_descriptor: KeyboardReport::desc(),
        request_handler: Some(REQUEST_HANDLER_CELL.init(HidRequestHandler {})),
        poll_ms: 60,
        max_packet_size: 8,
        hid_subclass: HidSubclass::Boot,
        hid_boot_protocol: HidBootProtocol::Keyboard,
    };
    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, HID_STATE.init(HidState::new()), hid_config);

    // Spawn device tasks
    spawner.spawn(usb_device_task(builder.build()).unwrap());
    spawner.spawn(logger_task(cdc).unwrap());
    spawner.spawn(hid_task(hid).unwrap());
}

#[embassy_executor::task]
async fn usb_device_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}

#[embassy_executor::task]
async fn logger_task(class: CdcAcmClass<'static, Driver<'static, USB>>) {
    embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, class).await;
}

#[embassy_executor::task]
async fn hid_task(mut writer: HidReaderWriter<'static, Driver<'static, USB>, 1, 8>) {
    let receiver = HID_CHANNEL.receiver();

    // Track currently pressed keys (max 6 keys for NKRO)
    let mut pressed_keys: heapless::Vec<u8, 6> = heapless::Vec::new();
    let mut modifiers: u8 = 0;

    log::info!("USB HID task started");

    loop {
        // Wait for HID event
        let event = receiver.receive().await;
        log::debug!("HID event: {event:?}");

        // Update state based on event
        match event {
            HidEvent::Reset => {
                pressed_keys.clear();
                modifiers = 0;
            }
            HidEvent::SetKey(keycode) => {
                // Add key if not already in list and there's space
                if !pressed_keys.contains(&keycode) && pressed_keys.len() < 6 {
                    let _ = pressed_keys.push(keycode);
                }
            }
            HidEvent::SetModifier(modifier) => {
                modifiers |= modifier;
            }
            HidEvent::ReleaseKey(keycode) => {
                // Remove key from list
                if let Some(pos) = pressed_keys.iter().position(|&k| k == keycode) {
                    pressed_keys.swap_remove(pos);
                }
            }
            HidEvent::ReleaseModifier(modifier) => {
                modifiers &= !modifier;
            }
        }

        // Build HID report
        let mut report = KeyboardReport {
            modifier: modifiers,
            reserved: 0,
            leds: 0,
            keycodes: [0u8; 6],
        };

        // Copy pressed keys into report
        for (i, &keycode) in pressed_keys.iter().enumerate() {
            if i < 6 {
                report.keycodes[i] = keycode;
            }
        }

        // Send report
        if let Err(e) = writer.write_serialize(&report).await {
            log::error!("HID: Failed to send report: {e:?}");
        }
    }
}