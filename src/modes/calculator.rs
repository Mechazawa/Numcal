use rust_decimal::Decimal;
use rust_decimal::prelude::*;

/// Calculator input types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Digit(u8),      // 0-9
    Decimal,        // .
    Operator(Operator),
    Equals,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Operator {
    fn apply(&self, left: Decimal, right: Decimal) -> Result<Decimal, CalculatorError> {
        match self {
            Operator::Add => Ok(left + right),
            Operator::Subtract => Ok(left - right),
            Operator::Multiply => Ok(left * right),
            Operator::Divide => {
                if right.is_zero() {
                    Err(CalculatorError::DivideByZero)
                } else {
                    Ok(left / right)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalculatorError {
    DivideByZero,
    InvalidInput,
    Overflow,
}

/// Calculator states as defined in the state machine diagram
#[derive(Debug, Clone, PartialEq)]
enum State {
    /// Display: 0, Clear all registers
    Initial,
    /// Display: Current input, Building first operand
    EnteringFirst,
    /// Display: 0, Current input cleared, Operator and first operand retained
    InputCleared,
    /// Display: First operand, Operator stored, Awaiting second operand
    OperatorSelected,
    /// Display: Current input, Building second operand
    EnteringSecond,
    /// Display: Calculation result, Result becomes first operand for chained operations
    ShowingResult,
}

/// Calculator state machine
pub struct Calculator {
    state: State,
    current_input: heapless::String<32>,
    first_operand: Option<Decimal>,
    operator: Option<Operator>,
    result: Option<Decimal>,
    clear_count: u8, // Track consecutive clear presses
}

impl Calculator {
    /// Create a new calculator in Initial state
    pub fn new() -> Self {
        Self {
            state: State::Initial,
            current_input: heapless::String::new(),
            first_operand: None,
            operator: None,
            result: None,
            clear_count: 0,
        }
    }

    /// Get the current display value
    pub fn display(&self) -> &str {
        match self.state {
            State::Initial => "0",
            State::EnteringFirst => {
                if self.current_input.is_empty() {
                    "0"
                } else {
                    &self.current_input
                }
            }
            State::InputCleared => "0",
            State::OperatorSelected => {
                if let Some(operand) = self.first_operand {
                    // Store formatted operand in current_input for display
                    // This is a bit hacky but necessary since we need to return &str
                    if let Some(result) = self.result {
                        // Use result as display if available
                        self.format_decimal(result)
                    } else {
                        self.format_decimal(operand)
                    }
                } else {
                    "0"
                }
            }
            State::EnteringSecond => {
                if self.current_input.is_empty() {
                    "0"
                } else {
                    &self.current_input
                }
            }
            State::ShowingResult => {
                if let Some(result) = self.result {
                    self.format_decimal(result)
                } else {
                    "0"
                }
            }
        }
    }

    /// Helper to format a Decimal value
    fn format_decimal(&self, value: Decimal) -> &str {
        // For embedded systems, we'll just use the Display trait
        // In practice, you'd want to format this into a buffer
        // This is a simplified version - in the real implementation
        // we'll handle this in the Mode task
        "0" // Placeholder - actual formatting happens in the task
    }

    /// Process an input and transition state
    pub fn input(&mut self, input: Input) -> Result<(), CalculatorError> {
        match input {
            Input::Clear => self.handle_clear(),
            _ => {
                // Any non-clear input resets clear counter
                self.clear_count = 0;
                match input {
                    Input::Digit(d) => self.handle_digit(d),
                    Input::Decimal => self.handle_decimal(),
                    Input::Operator(op) => self.handle_operator(op),
                    Input::Equals => self.handle_equals(),
                    Input::Clear => unreachable!(),
                }
            }
        }
    }

    fn handle_clear(&mut self) -> Result<(), CalculatorError> {
        self.clear_count += 1;

        match self.state {
            State::Initial => {
                // C in Initial -> stay in Initial
                self.state = State::Initial;
            }
            State::EnteringFirst => {
                // First C in EnteringFirst -> InputCleared
                self.current_input.clear();
                self.clear_count = 1;
                self.state = State::InputCleared;
            }
            State::InputCleared => {
                // Second C in InputCleared -> Initial
                if self.clear_count >= 2 {
                    self.reset();
                }
            }
            State::OperatorSelected => {
                // C in OperatorSelected -> Initial
                self.reset();
            }
            State::EnteringSecond => {
                // First C in EnteringSecond -> InputCleared
                self.current_input.clear();
                self.clear_count = 1;
                self.state = State::InputCleared;
            }
            State::ShowingResult => {
                // C in ShowingResult -> Initial
                self.reset();
            }
        }

        Ok(())
    }

    fn handle_digit(&mut self, digit: u8) -> Result<(), CalculatorError> {
        if digit > 9 {
            return Err(CalculatorError::InvalidInput);
        }

        match self.state {
            State::Initial => {
                self.current_input.clear();
                let _ = self.current_input.push((b'0' + digit) as char);
                self.state = State::EnteringFirst;
            }
            State::EnteringFirst => {
                if self.current_input.len() < 32 {
                    let _ = self.current_input.push((b'0' + digit) as char);
                }
            }
            State::InputCleared => {
                self.current_input.clear();
                let _ = self.current_input.push((b'0' + digit) as char);
                self.state = State::EnteringFirst;
            }
            State::OperatorSelected => {
                self.current_input.clear();
                let _ = self.current_input.push((b'0' + digit) as char);
                self.state = State::EnteringSecond;
            }
            State::EnteringSecond => {
                if self.current_input.len() < 32 {
                    let _ = self.current_input.push((b'0' + digit) as char);
                }
            }
            State::ShowingResult => {
                // Starting new calculation
                self.current_input.clear();
                let _ = self.current_input.push((b'0' + digit) as char);
                self.first_operand = None;
                self.operator = None;
                self.result = None;
                self.state = State::EnteringFirst;
            }
        }

        Ok(())
    }

    fn handle_decimal(&mut self) -> Result<(), CalculatorError> {
        match self.state {
            State::Initial => {
                self.current_input.clear();
                let _ = self.current_input.push_str("0.");
                self.state = State::EnteringFirst;
            }
            State::EnteringFirst => {
                if !self.current_input.contains('.') && self.current_input.len() < 31 {
                    if self.current_input.is_empty() {
                        let _ = self.current_input.push_str("0.");
                    } else {
                        let _ = self.current_input.push('.');
                    }
                }
            }
            State::InputCleared => {
                self.current_input.clear();
                let _ = self.current_input.push_str("0.");
                self.state = State::EnteringFirst;
            }
            State::OperatorSelected => {
                self.current_input.clear();
                let _ = self.current_input.push_str("0.");
                self.state = State::EnteringSecond;
            }
            State::EnteringSecond => {
                if !self.current_input.contains('.') && self.current_input.len() < 31 {
                    if self.current_input.is_empty() {
                        let _ = self.current_input.push_str("0.");
                    } else {
                        let _ = self.current_input.push('.');
                    }
                }
            }
            State::ShowingResult => {
                // Starting new calculation
                self.current_input.clear();
                let _ = self.current_input.push_str("0.");
                self.first_operand = None;
                self.operator = None;
                self.result = None;
                self.state = State::EnteringFirst;
            }
        }

        Ok(())
    }

    fn handle_operator(&mut self, op: Operator) -> Result<(), CalculatorError> {
        match self.state {
            State::Initial => {
                // Operating on implicit 0
                self.first_operand = Some(Decimal::ZERO);
                self.operator = Some(op);
                self.state = State::OperatorSelected;
            }
            State::EnteringFirst => {
                // Parse current input as first operand
                let value = self.parse_current_input()?;
                self.first_operand = Some(value);
                self.operator = Some(op);
                self.state = State::OperatorSelected;
            }
            State::InputCleared => {
                // Change operator (first operand already stored)
                self.operator = Some(op);
                self.state = State::OperatorSelected;
            }
            State::OperatorSelected => {
                // Change operator
                self.operator = Some(op);
            }
            State::EnteringSecond => {
                // Complete current operation and chain
                let second_value = self.parse_current_input()?;
                if let (Some(first), Some(operator)) = (self.first_operand, self.operator) {
                    let result = operator.apply(first, second_value)?;
                    self.result = Some(result);
                    self.first_operand = Some(result);
                    self.operator = Some(op);
                    self.current_input.clear();
                    self.state = State::OperatorSelected;
                }
            }
            State::ShowingResult => {
                // Continue with result as first operand
                if let Some(result) = self.result {
                    self.first_operand = Some(result);
                    self.operator = Some(op);
                    self.state = State::OperatorSelected;
                }
            }
        }

        Ok(())
    }

    fn handle_equals(&mut self) -> Result<(), CalculatorError> {
        match self.state {
            State::Initial => {
                // = in Initial -> stay in Initial
            }
            State::EnteringFirst => {
                // = with just first operand -> show it as result
                let value = self.parse_current_input()?;
                self.result = Some(value);
                self.first_operand = Some(value);
                self.state = State::ShowingResult;
            }
            State::InputCleared => {
                // No second operand, stay in InputCleared
            }
            State::OperatorSelected => {
                // No second operand, nothing to compute
            }
            State::EnteringSecond => {
                // Complete the calculation
                let second_value = self.parse_current_input()?;
                if let (Some(first), Some(operator)) = (self.first_operand, self.operator) {
                    let result = operator.apply(first, second_value)?;
                    self.result = Some(result);
                    self.first_operand = Some(result);
                    self.current_input.clear();
                    self.state = State::ShowingResult;
                }
            }
            State::ShowingResult => {
                // = in ShowingResult -> stay in ShowingResult
            }
        }

        Ok(())
    }

    fn parse_current_input(&self) -> Result<Decimal, CalculatorError> {
        if self.current_input.is_empty() {
            return Ok(Decimal::ZERO);
        }

        Decimal::from_str(&self.current_input)
            .map_err(|_| CalculatorError::InvalidInput)
    }

    fn reset(&mut self) {
        self.state = State::Initial;
        self.current_input.clear();
        self.first_operand = None;
        self.operator = None;
        self.result = None;
        self.clear_count = 0;
    }

    /// Get current state for debugging/testing
    #[cfg(test)]
    pub fn get_state(&self) -> &str {
        match self.state {
            State::Initial => "Initial",
            State::EnteringFirst => "EnteringFirst",
            State::InputCleared => "InputCleared",
            State::OperatorSelected => "OperatorSelected",
            State::EnteringSecond => "EnteringSecond",
            State::ShowingResult => "ShowingResult",
        }
    }

    /// Get the result value for testing
    #[cfg(test)]
    pub fn get_result(&self) -> Option<Decimal> {
        self.result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let calc = Calculator::new();
        assert_eq!(calc.get_state(), "Initial");
        assert_eq!(calc.display(), "0");
    }

    #[test]
    fn test_entering_first_operand() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(1)).unwrap();
        assert_eq!(calc.get_state(), "EnteringFirst");
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        assert_eq!(calc.display(), "123");
    }

    #[test]
    fn test_entering_decimal() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(4)).unwrap();
        assert_eq!(calc.display(), "3.14");
    }

    #[test]
    fn test_decimal_at_start() {
        let mut calc = Calculator::new();
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        assert_eq!(calc.display(), "0.5");
    }

    #[test]
    fn test_simple_addition() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        assert_eq!(calc.get_state(), "OperatorSelected");
        calc.input(Input::Digit(3)).unwrap();
        assert_eq!(calc.get_state(), "EnteringSecond");
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_state(), "ShowingResult");
        assert_eq!(calc.get_result().unwrap(), Decimal::from(5));
    }

    #[test]
    fn test_simple_subtraction() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        calc.input(Input::Operator(Operator::Subtract)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(7));
    }

    #[test]
    fn test_simple_multiplication() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(4)).unwrap();
        calc.input(Input::Operator(Operator::Multiply)).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(20));
    }

    #[test]
    fn test_simple_division() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(8)).unwrap();
        calc.input(Input::Operator(Operator::Divide)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(4));
    }

    #[test]
    fn test_divide_by_zero() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Divide)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        let result = calc.input(Input::Equals);
        assert!(matches!(result, Err(CalculatorError::DivideByZero)));
    }

    #[test]
    fn test_chained_operations() {
        let mut calc = Calculator::new();
        // 2 + 3 = 5, then + 4 = 9
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(5));

        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(4)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(9));
    }

    #[test]
    fn test_chained_without_equals() {
        let mut calc = Calculator::new();
        // 2 + 3 + 4 (pressing operator chains the operation)
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        assert_eq!(calc.get_state(), "OperatorSelected");
        // At this point, 2+3 should be computed
        calc.input(Input::Digit(4)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(9));
    }

    #[test]
    fn test_clear_in_initial() {
        let mut calc = Calculator::new();
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "Initial");
    }

    #[test]
    fn test_clear_while_entering_first() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "InputCleared");
        assert_eq!(calc.display(), "0");
    }

    #[test]
    fn test_double_clear_from_entering_first() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "InputCleared");
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "Initial");
    }

    #[test]
    fn test_clear_in_operator_selected() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "Initial");
    }

    #[test]
    fn test_clear_while_entering_second() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "InputCleared");
        assert_eq!(calc.display(), "0");
    }

    #[test]
    fn test_clear_after_result() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_state(), "ShowingResult");
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "Initial");
    }

    #[test]
    fn test_operator_change() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Operator(Operator::Multiply)).unwrap();
        assert_eq!(calc.get_state(), "OperatorSelected");
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(15));
    }

    #[test]
    fn test_equals_with_only_first_operand() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(7)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_state(), "ShowingResult");
        assert_eq!(calc.get_result().unwrap(), Decimal::from(7));
    }

    #[test]
    fn test_repeated_equals() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_state(), "ShowingResult");
    }

    #[test]
    fn test_decimal_operations() {
        let mut calc = Calculator::new();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(6));
    }

    /// Comprehensive flow test covering multiple scenarios
    #[test]
    fn test_comprehensive_flow() {
        let mut calc = Calculator::new();

        // Test 1: Basic calculation 10 + 5 = 15
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(15));
        assert_eq!(calc.get_state(), "ShowingResult");

        // Test 2: Continue with result: 15 * 2 = 30
        calc.input(Input::Operator(Operator::Multiply)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(30));

        // Test 3: Start new calculation: 100 / 4 = 25
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        calc.input(Input::Operator(Operator::Divide)).unwrap();
        calc.input(Input::Digit(4)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(25));

        // Test 4: Clear and start fresh: 7 - 3 = 4
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "Initial");
        calc.input(Input::Digit(7)).unwrap();
        calc.input(Input::Operator(Operator::Subtract)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(4));

        // Test 5: Clear during entry and continue: 8 + (clear) 5 = 13
        calc.input(Input::Digit(8)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(9)).unwrap();
        calc.input(Input::Digit(9)).unwrap();
        calc.input(Input::Clear).unwrap(); // Clear the 99
        assert_eq!(calc.get_state(), "InputCleared");
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(13));

        // Test 6: Chained operations without equals: 5 + 5 + 5 = 15
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(15));

        // Test 7: Decimal calculations: 0.5 + 0.5 = 1
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(1));

        // Test 8: Double clear to reset everything
        calc.input(Input::Digit(4)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Clear).unwrap();
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "Initial");

        // Test 9: Operator change mid-operation: 6 + (change to *) 3 = 18
        calc.input(Input::Digit(6)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Operator(Operator::Multiply)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(18));
    }

    /// Test complex decimal arithmetic
    #[test]
    fn test_complex_decimal_flow() {
        let mut calc = Calculator::new();

        // 12.5 + 7.3 = 19.8
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(7)).unwrap();
        calc.input(Input::Decimal).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();

        let result = calc.get_result().unwrap();
        assert_eq!(result, Decimal::from_str("19.8").unwrap());

        // Continue: 19.8 / 2 = 9.9
        calc.input(Input::Operator(Operator::Divide)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Equals).unwrap();

        let result = calc.get_result().unwrap();
        assert_eq!(result, Decimal::from_str("9.9").unwrap());
    }

    /// Test error handling flow
    #[test]
    fn test_error_handling_flow() {
        let mut calc = Calculator::new();

        // Division by zero
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Operator(Operator::Divide)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        let result = calc.input(Input::Equals);
        assert!(matches!(result, Err(CalculatorError::DivideByZero)));

        // Calculator should still be usable after error
        calc.input(Input::Clear).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap(), Decimal::from(5));
    }
}
