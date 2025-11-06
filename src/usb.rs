use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler};
use usbd_hid::descriptor::KeyboardReport;

use crate::{KEYMAP, USB_CHANNEL};

// Empty request handler for HID
pub struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        log::info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> embassy_usb::control::OutResponse {
        log::info!("Set report for {:?}: {:?}", id, data);
        embassy_usb::control::OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        log::info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        log::info!("Get idle rate for {:?}", id);
        None
    }
}

#[embassy_executor::task]
pub async fn usb_device_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) {
    usb.run().await;
}

#[embassy_executor::task]
pub async fn usb_hid_task(mut writer: HidReaderWriter<'static, Driver<'static, USB>, 1, 8>) {
    let usb_receiver = USB_CHANNEL.receiver();

    // Track currently pressed keys (max 6 keys for NKRO)
    let mut pressed_keys: heapless::Vec<u8, 6> = heapless::Vec::new();

    log::info!("USB HID task started");

    loop {
        // Wait for key event
        let event = usb_receiver.receive().await;
        log::info!(
            "USB HID: Key event R{}C{} pressed={}",
            event.row,
            event.col,
            event.pressed
        );

        let keycode = KEYMAP[event.row][event.col];

        // Skip if keycode is 0x00 (unused key)
        if keycode == 0x00 {
            continue;
        }

        // Update pressed keys list
        if event.pressed {
            // Add key if not already in list and there's space
            if !pressed_keys.contains(&keycode) && pressed_keys.len() < 6 {
                let _ = pressed_keys.push(keycode);
                log::info!(
                    "USB HID: Added keycode 0x{:02x}, now {} keys pressed",
                    keycode,
                    pressed_keys.len()
                );
            }
        } else {
            // Remove key from list
            if let Some(pos) = pressed_keys.iter().position(|&k| k == keycode) {
                pressed_keys.swap_remove(pos);
                log::info!(
                    "USB HID: Removed keycode 0x{:02x}, now {} keys pressed",
                    keycode,
                    pressed_keys.len()
                );
            }
        }

        // Build HID report
        let mut report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [0u8; 6],
        };

        // Copy pressed keys into report
        for (i, &keycode) in pressed_keys.iter().enumerate() {
            report.keycodes[i] = keycode;
        }

        log::info!("USB HID: Sending report with {} keys", pressed_keys.len());

        // Send report
        match writer.write_serialize(&report).await {
            Ok(()) => {
                log::info!("USB HID: Report sent successfully");
            }
            Err(e) => {
                log::error!("USB HID: Failed to send report: {:?}", e);
            }
        }
    }
}
