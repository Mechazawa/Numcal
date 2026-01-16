use crate::decimal::Decimal;
use crate::modes::{Mode, MODE_RUNNING};
use crate::tasks::{DisplayProxy, Key, KEYPAD_CHANNEL};
use core::fmt::Write;
use embassy_sync::pubsub::WaitResult;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;
use portable_atomic::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

impl Operator {
    fn apply(&self, left: Decimal, right: Decimal) -> Result<Decimal, ()> {
        match self {
            Operator::Add => Ok(left + right),
            Operator::Sub => Ok(left - right),
            Operator::Mul => Ok(left * right),
            Operator::Div => left / right,
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Operator::Add => "+",
            Operator::Sub => "-",
            Operator::Mul => "*",
            Operator::Div => "/",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CalcState {
    Initial,
    EnteringNumber { input: heapless::String<64> },
    EnteringRightOperand { left: Decimal, op: Operator, input: heapless::String<64> },
    ShowingResult { result: Decimal, last_op: Option<(Operator, Decimal)> },
    Error,
}

pub struct CalculatorMode {
    state: CalcState,
}

impl CalculatorMode {
    pub fn new() -> Self {
        Self {
            state: CalcState::Initial,
        }
    }

    fn press_button(&mut self, key: Key) {
        match &self.state {
            CalcState::Initial => {
                self.handle_initial_state(key);
            }
            CalcState::EnteringNumber { .. } => {
                self.handle_entering_number_state(key);
            }
            CalcState::EnteringRightOperand { .. } => {
                self.handle_entering_right_operand_state(key);
            }
            CalcState::ShowingResult { .. } => {
                self.handle_showing_result_state(key);
            }
            CalcState::Error => {
                // Any key press recovers from error
                self.state = CalcState::Initial;
                self.handle_initial_state(key);
            }
        }
    }

    fn handle_initial_state(&mut self, key: Key) {
        match key {
            Key::D0 | Key::D1 | Key::D2 | Key::D3 | Key::D4 | Key::D5 | Key::D6 | Key::D7
            | Key::D8 | Key::D9 => {
                let mut input = heapless::String::new();
                let _ = input.push(self.key_to_char(key));
                self.state = CalcState::EnteringNumber { input };
            }
            Key::Dot => {
                let mut input = heapless::String::new();
                let _ = input.push_str("0.");
                self.state = CalcState::EnteringNumber { input };
            }
            Key::Lock => {
                // Reset button - already in initial state, do nothing
            }
            _ => {
                // Ignore other keys in initial state
            }
        }
    }

    fn handle_entering_number_state(&mut self, key: Key) {
        let current_input = if let CalcState::EnteringNumber { input } = &self.state {
            input.clone()
        } else {
            return;
        };

        match key {
            Key::D0 | Key::D1 | Key::D2 | Key::D3 | Key::D4 | Key::D5 | Key::D6 | Key::D7
            | Key::D8 | Key::D9 => {
                let mut new_input = current_input.clone();
                let _ = new_input.push(self.key_to_char(key));
                self.state = CalcState::EnteringNumber { input: new_input };
            }
            Key::Dot => {
                // Only add decimal point if there isn't one already
                if !current_input.contains('.') {
                    let mut new_input = current_input.clone();
                    let _ = new_input.push('.');
                    self.state = CalcState::EnteringNumber { input: new_input };
                }
            }
            Key::Add | Key::Sub | Key::Mul | Key::Div => {
                if let Ok(value) = Decimal::from_str(&current_input) {
                    let op = self.key_to_operator(key).unwrap();
                    let input = heapless::String::new();
                    self.state = CalcState::EnteringRightOperand { left: value, op, input };
                }
            }
            Key::Enter => {
                // Just pressing enter on a number shows the number (no operation)
                if let Ok(value) = Decimal::from_str(&current_input) {
                    self.state = CalcState::ShowingResult { result: value, last_op: None };
                }
            }
            Key::Lock => {
                // First reset press - clear current input
                self.state = CalcState::Initial;
            }
            _ => {
                // Ignore other keys
            }
        }
    }

    fn handle_entering_right_operand_state(&mut self, key: Key) {
        let (left, op, current_input) = if let CalcState::EnteringRightOperand { left, op, input } = &self.state {
            (*left, *op, input.clone())
        } else {
            return;
        };

        match key {
            Key::D0 | Key::D1 | Key::D2 | Key::D3 | Key::D4 | Key::D5 | Key::D6 | Key::D7
            | Key::D8 | Key::D9 => {
                let mut new_input = current_input.clone();
                let _ = new_input.push(self.key_to_char(key));
                self.state = CalcState::EnteringRightOperand { left, op, input: new_input };
            }
            Key::Dot => {
                // Only add decimal point if there isn't one already
                if !current_input.contains('.') {
                    let mut new_input = current_input.clone();
                    let _ = new_input.push('.');
                    self.state = CalcState::EnteringRightOperand { left, op, input: new_input };
                }
            }
            Key::Add | Key::Sub | Key::Mul | Key::Div => {
                // Calculate the current operation first, then start new operation
                if current_input.is_empty() {
                    // No right operand entered yet, just replace operator
                    let new_op = self.key_to_operator(key).unwrap();
                    self.state = CalcState::EnteringRightOperand { left, op: new_op, input: heapless::String::new() };
                } else {
                    // Calculate current operation
                    if let Ok(right) = Decimal::from_str(&current_input) {
                        match op.apply(left, right) {
                            Ok(result) => {
                                let new_op = self.key_to_operator(key).unwrap();
                                self.state = CalcState::EnteringRightOperand {
                                    left: result,
                                    op: new_op,
                                    input: heapless::String::new()
                                };
                            }
                            Err(_) => {
                                self.state = CalcState::Error;
                            }
                        }
                    }
                }
            }
            Key::Enter => {
                // Calculate the result
                if !current_input.is_empty() {
                    if let Ok(right) = Decimal::from_str(&current_input) {
                        match op.apply(left, right) {
                            Ok(result) => {
                                self.state = CalcState::ShowingResult {
                                    result,
                                    last_op: Some((op, right)),
                                };
                            }
                            Err(_) => {
                                self.state = CalcState::Error;
                            }
                        }
                    }
                }
            }
            Key::Lock => {
                // Reset
                self.state = CalcState::Initial;
            }
            _ => {
                // Ignore
            }
        }
    }

    fn handle_showing_result_state(&mut self, key: Key) {
        let (result, last_op) = if let CalcState::ShowingResult { result, last_op } = &self.state
        {
            (*result, last_op.clone())
        } else {
            return;
        };

        match key {
            Key::D0 | Key::D1 | Key::D2 | Key::D3 | Key::D4 | Key::D5 | Key::D6 | Key::D7
            | Key::D8 | Key::D9 => {
                // Start new calculation
                let mut input = heapless::String::new();
                let _ = input.push(self.key_to_char(key));
                self.state = CalcState::EnteringNumber { input };
            }
            Key::Dot => {
                let mut input = heapless::String::new();
                let _ = input.push_str("0.");
                self.state = CalcState::EnteringNumber { input };
            }
            Key::Add | Key::Sub | Key::Mul | Key::Div => {
                // Use result as left operand for next operation
                let op = self.key_to_operator(key).unwrap();
                self.state = CalcState::EnteringRightOperand {
                    left: result,
                    op,
                    input: heapless::String::new()
                };
            }
            Key::Enter => {
                // Repeat last operation
                if let Some((op, right)) = last_op {
                    match op.apply(result, right) {
                        Ok(new_result) => {
                            self.state = CalcState::ShowingResult {
                                result: new_result,
                                last_op: Some((op, right)),
                            };
                        }
                        Err(_) => {
                            self.state = CalcState::Error;
                        }
                    }
                }
            }
            Key::Lock => {
                // Reset
                self.state = CalcState::Initial;
            }
            _ => {
                // Ignore
            }
        }
    }

    fn key_to_char(&self, key: Key) -> char {
        match key {
            Key::D0 => '0',
            Key::D1 => '1',
            Key::D2 => '2',
            Key::D3 => '3',
            Key::D4 => '4',
            Key::D5 => '5',
            Key::D6 => '6',
            Key::D7 => '7',
            Key::D8 => '8',
            Key::D9 => '9',
            _ => '?',
        }
    }

    fn key_to_operator(&self, key: Key) -> Option<Operator> {
        match key {
            Key::Add => Some(Operator::Add),
            Key::Sub => Some(Operator::Sub),
            Key::Mul => Some(Operator::Mul),
            Key::Div => Some(Operator::Div),
            _ => None,
        }
    }

    fn get_display_text(&self) -> heapless::String<64> {
        let mut text = heapless::String::new();

        match &self.state {
            CalcState::Initial => {
                let _ = write!(&mut text, "0");
            }
            CalcState::EnteringNumber { input } => {
                let _ = write!(&mut text, "{}", input);
            }
            CalcState::EnteringRightOperand { left, op, input } => {
                let mut left_str = heapless::String::new();
                let _ = left.format_to_string(&mut left_str);

                if input.is_empty() {
                    let _ = write!(&mut text, "{} {}", left_str, op.symbol());
                } else {
                    let _ = write!(&mut text, "{} {} {}", left_str, op.symbol(), input);
                }
            }
            CalcState::ShowingResult { result, .. } => {
                let _ = result.format_to_string(&mut text);
            }
            CalcState::Error => {
                let _ = write!(&mut text, "Error");
            }
        }

        text
    }

    fn render_display(&self) {
        let mut display = DisplayProxy::new();
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        display.clear(BinaryColor::Off).unwrap();

        let display_text = self.get_display_text();

        // Calculate text width for scrolling
        // Each character in FONT_6X10 is 6 pixels wide
        let text_width = display_text.len() * 6;
        let screen_width = 128;

        let x_pos = if text_width > screen_width {
            // Scroll left to show the rightmost part
            -(text_width as i32 - screen_width as i32)
        } else {
            // Right-align
            (screen_width - text_width) as i32
        };

        Text::with_baseline(
            &display_text,
            Point::new(x_pos, 38),
            text_style,
            Baseline::Middle,
        )
        .draw(&mut display)
        .unwrap();

        display.flush().unwrap();
    }
}

impl Mode for CalculatorMode {
    async fn task(&mut self) {
        let mut keypad = KEYPAD_CHANNEL.subscriber().unwrap();

        // Initial display
        self.render_display();

        while MODE_RUNNING.load(Ordering::Relaxed) {
            if let WaitResult::Message(event) = keypad.next_message().await {
                // Only process key presses, not releases
                if !event.pressed {
                    continue;
                }

                // Ignore function keys (used for mode switching)
                if matches!(event.key, Key::F1 | Key::F2 | Key::F3 | Key::F4) {
                    continue;
                }

                // Process the button press
                self.press_button(event.key);

                // Update display
                self.render_display();
            }
        }
    }
}
