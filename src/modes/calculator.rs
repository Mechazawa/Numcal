// Calculator mode implementation with fixed-point arithmetic
// Uses Q16.16 fixed-point format for accurate decimal calculations

use core::fmt::Write;
use fixed::types::I32F32; // 32-bit integer, 32-bit fractional (high precision)

/// Fixed-point type for calculator operations
pub type CalcNum = I32F32;

/// Calculator state
pub struct Calculator {
    /// Current input buffer (what user is typing)
    input: heapless::String<32>,
    /// Current result
    result: CalcNum,
    /// Last calculation for history
    last_calculation: heapless::String<64>,
    /// Error message if any
    error: Option<heapless::String<32>>,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            input: heapless::String::new(),
            result: CalcNum::ZERO,
            last_calculation: heapless::String::new(),
            error: None,
        }
    }

    /// Handle a key press in calculator mode
    /// Returns true if display needs update
    pub fn handle_key(&mut self, key_char: char) -> bool {
        // Clear any previous error
        self.error = None;

        match key_char {
            // Digits
            '0'..='9' => {
                if self.input.push(key_char).is_err() {
                    self.set_error("Input too long");
                }
                true
            }
            // Decimal point
            '.' => {
                // Only allow one decimal point
                if !self.input.contains('.') && self.input.push('.').is_err() {
                    self.set_error("Input too long");
                }
                true
            }
            // Operators
            '+' | '-' | '*' | '/' => {
                if self.input.push(key_char).is_err() {
                    self.set_error("Input too long");
                }
                true
            }
            // Evaluate (Enter key)
            '=' => {
                self.evaluate();
                true
            }
            // Clear
            'C' => {
                self.clear();
                true
            }
            // Reset (when input is already empty)
            'R' => {
                self.reset();
                true
            }
            _ => false,
        }
    }

    /// Clear current input
    fn clear(&mut self) {
        if self.input.is_empty() {
            // If already empty, reset everything
            self.reset();
        } else {
            // Just clear input
            self.input.clear();
        }
    }

    /// Reset calculator completely
    fn reset(&mut self) {
        self.input.clear();
        self.result = CalcNum::ZERO;
        self.last_calculation.clear();
        self.error = None;
    }

    /// Evaluate the current expression
    fn evaluate(&mut self) {
        if self.input.is_empty() {
            return;
        }

        match self.parse_and_eval() {
            Ok(result) => {
                // Save to history
                let input_copy = self.input.clone();
                self.last_calculation.clear();
                write!(&mut self.last_calculation, "{} = ", input_copy.as_str()).ok();
                Self::format_number_static(&mut self.last_calculation, result);

                self.result = result;
                self.input.clear();
            }
            Err(e) => {
                self.set_error(e);
            }
        }
    }

    /// Parse and evaluate the expression
    /// Simple expression evaluator: handles one operation at a time
    fn parse_and_eval(&self) -> Result<CalcNum, &'static str> {
        let expr = self.input.as_str();

        // Find the operator (scan from right to handle negative numbers)
        let mut op_pos = None;
        let mut op = ' ';

        for (i, ch) in expr.char_indices().skip(1) {
            // Skip first char (could be minus sign)
            if matches!(ch, '+' | '-' | '*' | '/') {
                op_pos = Some(i);
                op = ch;
                // Don't break - we want the rightmost operator for left-to-right evaluation
            }
        }

        if let Some(pos) = op_pos {
            // Split into left and right operands
            let left_str = &expr[..pos];
            let right_str = &expr[pos + 1..];

            let left = self.parse_number(left_str)?;
            let right = self.parse_number(right_str)?;

            match op {
                '+' => Ok(left + right),
                '-' => Ok(left - right),
                '*' => Ok(left * right),
                '/' => {
                    if right == CalcNum::ZERO {
                        Err("Div by 0")
                    } else {
                        Ok(left / right)
                    }
                }
                _ => Err("Invalid op"),
            }
        } else {
            // No operator, just parse as number
            self.parse_number(expr)
        }
    }

    /// Parse a number string to fixed-point
    fn parse_number(&self, s: &str) -> Result<CalcNum, &'static str> {
        if s.is_empty() {
            return Err("Empty number");
        }

        // Parse manually since we're in no_std
        let mut negative = false;
        let mut s = s;

        if s.starts_with('-') {
            negative = true;
            s = &s[1..];
        }

        // Split on decimal point
        let parts: heapless::Vec<&str, 2> = s.split('.').collect();

        let integer_part: i64 = parts
            .first()
            .and_then(|p| self.parse_i64(p).ok())
            .ok_or("Invalid number")?;

        let mut result = CalcNum::from_num(integer_part);

        // Handle fractional part
        if let Some(&frac_str) = parts.get(1) {
            let mut frac_value = 0i64;
            let mut divisor = 1i64;

            for ch in frac_str.chars() {
                if let Some(digit) = ch.to_digit(10) {
                    frac_value = frac_value * 10 + digit as i64;
                    divisor *= 10;
                } else {
                    return Err("Invalid digit");
                }
            }

            let frac_result = CalcNum::from_num(frac_value) / CalcNum::from_num(divisor);
            result += frac_result;
        }

        if negative {
            result = -result;
        }

        Ok(result)
    }

    /// Parse string to i64 (helper for no_std)
    fn parse_i64(&self, s: &str) -> Result<i64, &'static str> {
        if s.is_empty() {
            return Err("Empty string");
        }

        let mut result = 0i64;
        for ch in s.chars() {
            if let Some(digit) = ch.to_digit(10) {
                result = result
                    .checked_mul(10)
                    .and_then(|r| r.checked_add(digit as i64))
                    .ok_or("Overflow")?;
            } else {
                return Err("Invalid digit");
            }
        }

        Ok(result)
    }

    /// Format a number with appropriate decimal places (static method)
    fn format_number_static(buf: &mut heapless::String<64>, num: CalcNum) {
        // Convert to string with up to 4 decimal places
        let integer = num.to_num::<i64>();
        let frac = (num.frac() * CalcNum::from_num(10000))
            .to_num::<i64>()
            .abs();

        if frac == 0 {
            // No fractional part
            write!(buf, "{integer}").ok();
        } else {
            // Has fractional part - show up to 4 decimals, trim trailing zeros
            let mut frac_str = heapless::String::<8>::new();
            write!(&mut frac_str, "{:04}", frac).ok();

            // Trim trailing zeros
            while frac_str.ends_with('0') {
                frac_str.pop();
            }

            write!(buf, "{}.{}", integer, frac_str).ok();
        }
    }

    /// Set error message
    fn set_error(&mut self, msg: &'static str) {
        let mut err_str = heapless::String::new();
        write!(&mut err_str, "{}", msg).ok();
        self.error = Some(err_str);
    }

    /// Format display output for calculator mode
    pub fn format_display(&self) -> heapless::String<64> {
        let mut text = heapless::String::new();

        if let Some(ref err) = self.error {
            // Show error
            write!(&mut text, "[CALC] Error: {}", err.as_str()).ok();
        } else if self.input.is_empty() {
            // Show result or ready state
            write!(&mut text, "[CALC] = ").ok();
            Self::format_number_static(&mut text, self.result);
        } else {
            // Show current input
            write!(&mut text, "[CALC] {}", self.input.as_str()).ok();
        }

        text
    }

    /// Get history line for display (optional second line)
    #[allow(dead_code)]
    pub fn get_history(&self) -> Option<&str> {
        if self.last_calculation.is_empty() {
            None
        } else {
            Some(self.last_calculation.as_str())
        }
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}
