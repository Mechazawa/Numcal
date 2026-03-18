use embassy_sync::pubsub::WaitResult;
use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X10};
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};
use portable_atomic::Ordering;

use crate::utils::calc_number::CalcNumber;
use crate::modes::Mode;
use crate::tasks::{DisplayProxy, Key, KEYPAD_CHANNEL};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Op {
    fn symbol(self) -> char {
        match self {
            Op::Add => '+',
            Op::Sub => '-',
            Op::Mul => '*',
            Op::Div => '/',
        }
    }
}

pub struct CalculatorMode {
    /// Current number being entered
    input: CalcNumber,
    /// Whether user is actively typing digits
    inputting: bool,
    /// Whether decimal point has been placed in current input
    has_dot: bool,
    /// The accumulated result (left operand)
    accumulator: CalcNumber,
    /// Pending operation waiting for right operand
    pending_op: Option<Op>,
    /// Error state (division by zero)
    error: bool,
}

impl CalculatorMode {
    pub fn new() -> Self {
        Self {
            input: CalcNumber::zero(),
            inputting: true,
            has_dot: false,
            accumulator: CalcNumber::zero(),
            pending_op: None,
            error: false,
        }
    }

    fn clear(&mut self) {
        self.input = CalcNumber::zero();
        self.inputting = true;
        self.has_dot = false;
        self.accumulator = CalcNumber::zero();
        self.pending_op = None;
        self.error = false;
    }

    fn current_display_value(&self) -> &CalcNumber {
        if self.inputting {
            &self.input
        } else {
            &self.accumulator
        }
    }

    fn press_digit(&mut self, digit: u8) {
        if self.error {
            return;
        }

        if !self.inputting {
            self.input = CalcNumber::zero();
            self.has_dot = false;
            self.inputting = true;
        }

        // Replace initial zero with the digit (unless we have a decimal point)
        if self.input.is_zero() && !self.has_dot && self.input.decimal_places == 0 {
            self.input.digits.clear();
            self.input.digits.push(digit).ok();
        } else {
            let total_chars = self.input.digits.len()
                + usize::from(self.has_dot)
                + usize::from(self.input.negative);
            if total_chars >= 13 {
                return;
            }
            self.input.digits.push(digit).ok();
            if self.has_dot {
                self.input.decimal_places += 1;
            }
        }
    }

    fn press_dot(&mut self) {
        if self.error {
            return;
        }

        if !self.inputting {
            self.input = CalcNumber::zero();
            self.has_dot = true;
            self.inputting = true;
            return;
        }

        if !self.has_dot {
            self.has_dot = true;
        }
    }

    fn press_operator(&mut self, op: Op) {
        if self.error {
            return;
        }

        if self.inputting && self.pending_op.is_some() {
            self.evaluate();
            if self.error {
                return;
            }
        } else if self.inputting {
            self.accumulator = self.input.clone();
        }

        self.pending_op = Some(op);
        self.inputting = false;
    }

    fn press_negate(&mut self) {
        if self.error {
            return;
        }

        if self.inputting {
            self.input.negate();
        } else {
            self.accumulator.negate();
        }
    }

    fn evaluate(&mut self) {
        if let Some(op) = self.pending_op.take() {
            let result = match op {
                Op::Add => CalcNumber::add(&self.accumulator, &self.input),
                Op::Sub => CalcNumber::sub(&self.accumulator, &self.input),
                Op::Mul => CalcNumber::mul(&self.accumulator, &self.input),
                Op::Div => {
                    if let Some(r) = CalcNumber::div(&self.accumulator, &self.input) {
                        r
                    } else {
                        self.error = true;
                        return;
                    }
                }
            };
            self.accumulator = result;
            self.input = CalcNumber::zero();
            self.has_dot = false;
            self.inputting = false;
        }
    }

    fn press_enter(&mut self) {
        if self.error {
            return;
        }

        if self.inputting && self.pending_op.is_some() {
            self.evaluate();
        }
    }

    fn draw(&self, display: &mut DisplayProxy) {
        display.clear(BinaryColor::Off).unwrap();

        let small_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        let main_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build();

        let right_top = TextStyleBuilder::new()
            .alignment(Alignment::Right)
            .baseline(Baseline::Top)
            .build();

        // Main number display (or "Error")
        if self.error {
            Text::with_text_style("Error", Point::new(127, 38), main_style, right_top)
                .draw(display)
                .unwrap();
        } else {
            let display_val = self.current_display_value();
            let mut text = display_val.to_display_string();

            // If actively inputting and has_dot but no decimal digits yet, show trailing dot
            if self.inputting && self.has_dot && self.input.decimal_places == 0 {
                text.push('.').ok();
            }

            Text::with_text_style(text.as_str(), Point::new(127, 38), main_style, right_top)
                .draw(display)
                .unwrap();
        }

        // Pending operator indicator bottom-right
        if let Some(op) = self.pending_op {
            let mut buf = [0u8; 1];
            let sym = op.symbol().encode_utf8(&mut buf);
            Text::with_text_style(sym, Point::new(127, 58), small_style, right_top)
                .draw(display)
                .unwrap();
        }

        display.flush().unwrap();
    }

    fn handle_key(&mut self, key: Key) {
        match key {
            Key::D0 => self.press_digit(0),
            Key::D1 => self.press_digit(1),
            Key::D2 => self.press_digit(2),
            Key::D3 => self.press_digit(3),
            Key::D4 => self.press_digit(4),
            Key::D5 => self.press_digit(5),
            Key::D6 => self.press_digit(6),
            Key::D7 => self.press_digit(7),
            Key::D8 => self.press_digit(8),
            Key::D9 => self.press_digit(9),
            Key::Dot => self.press_dot(),
            Key::Add => self.press_operator(Op::Add),
            Key::Sub => {
                if self.inputting && self.input.is_zero() && self.input.digits.len() <= 1 && self.pending_op.is_none() {
                    self.press_negate();
                } else if !self.inputting {
                    self.input = CalcNumber::zero();
                    self.inputting = true;
                    self.has_dot = false;
                    self.input.negative = true;
                } else {
                    self.press_operator(Op::Sub);
                }
            }
            Key::Mul => self.press_operator(Op::Mul),
            Key::Div => self.press_operator(Op::Div),
            Key::Enter => self.press_enter(),
            Key::Lock => self.clear(),
            Key::F1 | Key::F2 | Key::F3 | Key::F4 | Key::NC => {}
        }
    }
}

impl Mode for CalculatorMode {
    async fn task(&mut self) {
        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();
        let mut display = DisplayProxy::new();
        let mode_running = &super::MODE_RUNNING;

        self.draw(&mut display);

        while mode_running.load(Ordering::Relaxed) {
            if let WaitResult::Message(event) = keypad.next_message().await {
                if !event.pressed {
                    continue;
                }

                self.handle_key(event.key);
                self.draw(&mut display);
            }
        }
    }
}
