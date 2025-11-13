// Integration test for calculator that can run on host
#[path = "../src/modes/calculator.rs"]
mod calculator;

use calculator::*;

// Re-run all the module tests here to ensure they work
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
    fn test_comprehensive_flow() {
        let mut calc = Calculator::new();

        // Test 1: Basic calculation 10 + 5 = 15
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        calc.input(Input::Operator(Operator::Add)).unwrap();
        calc.input(Input::Digit(5)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap().to_string(), "15");
        assert_eq!(calc.get_state(), "ShowingResult");

        // Test 2: Continue with result: 15 * 2 = 30
        calc.input(Input::Operator(Operator::Multiply)).unwrap();
        calc.input(Input::Digit(2)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap().to_string(), "30");

        // Test 3: Start new calculation: 100 / 4 = 25
        calc.input(Input::Digit(1)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        calc.input(Input::Digit(0)).unwrap();
        calc.input(Input::Operator(Operator::Divide)).unwrap();
        calc.input(Input::Digit(4)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap().to_string(), "25");

        // Test 4: Clear and start fresh: 7 - 3 = 4
        calc.input(Input::Clear).unwrap();
        assert_eq!(calc.get_state(), "Initial");
        calc.input(Input::Digit(7)).unwrap();
        calc.input(Input::Operator(Operator::Subtract)).unwrap();
        calc.input(Input::Digit(3)).unwrap();
        calc.input(Input::Equals).unwrap();
        assert_eq!(calc.get_result().unwrap().to_string(), "4");
    }
}
