use crate::modes::{Mode, MODE_RUNNING};
use crate::show_text;
use crate::tasks::{HID_CHANNEL, HidEvent, KEYPAD_CHANNEL, Key, DisplayProxy};
use portable_atomic::{Ordering};
use embassy_sync::pubsub::{WaitResult};
use core::fmt::Write;
use rust_decimal::Decimal;

const MEMORY_SIZE: usize = 4;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct CalculatorMode {
    memory: [Option<Decimal>; MEMORY_SIZE],
    answer: Option<Decimal>,
    input: Decimal,
    operation: Option<Operation>,
}

impl CalculatorMode {
    pub fn new() -> Self {
        Self {
            memory: [None; MEMORY_SIZE],
        }
    }
}

impl Mode for CalculatorMode {
    async fn task(&mut self) {
        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();
        let mut display = DisplayProxy::new();

    }
}
