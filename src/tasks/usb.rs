use crate::tasks::claude_link::claude_link_task;
use crate::utils::line_buffer::LineBuffer;
use core::cell::RefCell;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::{Peri, bind_interrupts};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcState};
use embassy_usb::class::hid::{HidReader, HidReaderWriter, HidWriter, ReportId, RequestHandler, State as HidState};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config as UsbConfig};
use embassy_usb_logger::ReceiverHandler;
use portable_atomic::{AtomicU8, Ordering};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

/// Events that can be sent to the HID task
#[derive(Clone, Copy, Debug)]
pub enum HidEvent {
    Reset,
    SetKey(u8),
    ReleaseKey(u8),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum KeyboardLed {
    Num = 1 << 0,
    Caps = 1 << 1,
    Scroll = 1 << 2,
}

/// Channel for sending HID events to the HID task
pub static HID_CHANNEL: Channel<ThreadModeRawMutex, HidEvent, 32> = Channel::new();
pub static LED_STATE: LedState = LedState::new();
pub static LED_CHANGED: Signal<ThreadModeRawMutex, ()> = Signal::new();

// Interrupts
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// Buffers
static CONFIG_DESCRIPTOR: StaticCell<[u8; 512]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static CDC_STATE: StaticCell<CdcState> = StaticCell::new();
static CLAUDE_CDC_STATE: StaticCell<CdcState> = StaticCell::new();
static HID_STATE: StaticCell<HidState> = StaticCell::new();
static REQUEST_HANDLER_CELL: StaticCell<HidRequestHandler> = StaticCell::new();

// HID Request Handler
struct HidRequestHandler {}

impl RequestHandler for HidRequestHandler {
    fn get_report(&mut self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&mut self, _id: ReportId, data: &[u8]) -> OutResponse {
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
        let old = self.0.swap(state, Ordering::Relaxed);
        if old != state {
            LED_CHANGED.signal(());
        }
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
        CONFIG_DESCRIPTOR.init([0; 512]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [], // no msos descriptors
        CONTROL_BUF.init([0; 64]),
    );

    let cdc = CdcAcmClass::new(&mut builder, CDC_STATE.init(CdcState::new()), 64);
    let claude_cdc = CdcAcmClass::new(&mut builder, CLAUDE_CDC_STATE.init(CdcState::new()), 64);

    // Create HID class for keyboard
    let hid_config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: Some(REQUEST_HANDLER_CELL.init(HidRequestHandler {})),
        poll_ms: 1,
        max_packet_size: 8,
    };
    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, HID_STATE.init(HidState::new()), hid_config);

    let (reader, writer) = hid.split();

    // Spawn device tasks
    spawner.spawn(usb_device_task(builder.build()).unwrap());
    spawner.spawn(logger_task(cdc).unwrap());
    spawner.spawn(claude_link_task(claude_cdc).unwrap());
    spawner.spawn(hid_writer_task(writer).unwrap());
    spawner.spawn(hid_reader_task(reader).unwrap());
}

#[embassy_executor::task]
async fn usb_device_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}

/// Commands on the log port: `bootsel` reboots for flashing, and debug builds add their own.
struct ConsoleCommands(Mutex<CriticalSectionRawMutex, RefCell<LineBuffer>>);

impl ReceiverHandler for ConsoleCommands {
    async fn handle_data(&self, data: &[u8]) {
        self.0.lock(|lines| {
            lines.borrow_mut().feed(data, |line| match line {
                "bootsel" => reset_to_usb_boot(0, 0),
                #[cfg(feature = "debug-link")]
                _ => {
                    if let Some(command) = crate::debug_link::DebugCommand::parse(line) {
                        let _ = crate::debug_link::DEBUG_COMMANDS.try_send(command);
                    }
                }
                #[cfg(not(feature = "debug-link"))]
                _ => {}
            });
        });
    }

    fn new() -> Self {
        Self(Mutex::new(RefCell::new(LineBuffer::new())))
    }
}

#[embassy_executor::task]
async fn logger_task(class: CdcAcmClass<'static, Driver<'static, USB>>) {
    embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, class, ConsoleCommands).await;
}

#[embassy_executor::task]
async fn hid_writer_task(mut writer: HidWriter<'static, Driver<'static, USB>, 8>) {
    let receiver = HID_CHANNEL.receiver();

    // Track currently pressed keys (max 6 keys for NKRO)
    let mut pressed_keys: heapless::Vec<u8, 6> = heapless::Vec::new();

    log::info!("USB HID writer task started");

    loop {
        let event = receiver.receive().await;
        log::debug!("HID event: {event:?}");

        match event {
            HidEvent::Reset => {
                pressed_keys.clear();
            }
            HidEvent::SetKey(keycode) => {
                if !pressed_keys.contains(&keycode) && pressed_keys.len() < 6 {
                    let _ = pressed_keys.push(keycode);
                }
            }
            HidEvent::ReleaseKey(keycode) => {
                if let Some(pos) = pressed_keys.iter().position(|&k| k == keycode) {
                    pressed_keys.swap_remove(pos);
                }
            }
        }

        let mut report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [0u8; 6],
        };

        for (i, &keycode) in pressed_keys.iter().enumerate() {
            if i < 6 {
                report.keycodes[i] = keycode;
            }
        }

        if let Err(e) = writer.write_serialize(&report).await {
            log::error!("HID: Failed to send report: {e:?}");
        }
    }
}

#[embassy_executor::task]
async fn hid_reader_task(mut reader: HidReader<'static, Driver<'static, USB>, 1>) {
    let mut buf = [0u8; 1];

    log::info!("USB HID reader task started");

    loop {
        match reader.read(&mut buf).await {
            Ok(len) if len > 0 => LED_STATE.store(buf[0]),
            Ok(_) => {}
            Err(e) => log::error!("HID read error: {e:?}"),
        }
    }
}
