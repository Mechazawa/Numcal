/// Simple fixed-precision decimal type for no_std embedded environments
/// Stores value as i64 with 6 decimal places (micros)
/// Range: approximately ±9.2 trillion with 6 decimal places
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal(i64);

const SCALE: i64 = 1_000_000;
const DECIMAL_PLACES: usize = 6;

impl Decimal {
    #[allow(dead_code)]
    pub const ZERO: Self = Decimal(0);
    #[allow(dead_code)]
    pub const ONE: Self = Decimal(SCALE);

    /// Create from an integer value
    #[allow(dead_code)]
    pub const fn from_i64(value: i64) -> Self {
        Decimal(value.saturating_mul(SCALE))
    }

    /// Check if the value is zero
    #[allow(dead_code)]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Parse from a string representation
    pub fn from_str(s: &str) -> Result<Self, ()> {
        let s = s.trim();
        if s.is_empty() {
            return Err(());
        }

        let negative = s.starts_with('-');
        let s = if negative { &s[1..] } else { s };

        // Split into integer and fractional parts
        let (int_part, frac_part) = if let Some(dot_pos) = s.find('.') {
            (&s[..dot_pos], &s[dot_pos + 1..])
        } else {
            (s, "")
        };

        // Parse integer part
        let int_value: i64 = int_part.parse().map_err(|_| ())?;

        // Parse fractional part (up to DECIMAL_PLACES digits)
        let mut frac_value: i64 = 0;
        let _frac_len = frac_part.len().min(DECIMAL_PLACES);

        for (i, ch) in frac_part.chars().take(DECIMAL_PLACES).enumerate() {
            if !ch.is_ascii_digit() {
                return Err(());
            }
            let digit = (ch as u8 - b'0') as i64;
            frac_value += digit * 10i64.pow((DECIMAL_PLACES - 1 - i) as u32);
        }

        // Combine parts
        let mut result = int_value.saturating_mul(SCALE);
        result = result.saturating_add(frac_value);

        if negative {
            result = -result;
        }

        Ok(Decimal(result))
    }

    /// Format as string with proper decimal places
    pub fn format_to_string(&self, buf: &mut heapless::String<64>) -> Result<(), ()> {
        use core::fmt::Write;

        let value = self.0;
        let negative = value < 0;
        let abs_value = value.abs();

        if negative {
            buf.push('-').map_err(|_| ())?;
        }

        let int_part = abs_value / SCALE;
        let frac_part = abs_value % SCALE;

        write!(buf, "{}", int_part).map_err(|_| ())?;

        if frac_part > 0 {
            buf.push('.').map_err(|_| ())?;

            // Format fractional part, removing trailing zeros
            let mut frac_str = heapless::String::<16>::new();
            write!(&mut frac_str, "{:06}", frac_part).map_err(|_| ())?;

            // Remove trailing zeros
            let trimmed = frac_str.trim_end_matches('0');
            buf.push_str(trimmed).map_err(|_| ())?;
        }

        Ok(())
    }
}

// Arithmetic operations
impl core::ops::Add for Decimal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Decimal(self.0.saturating_add(other.0))
    }
}

impl core::ops::Sub for Decimal {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Decimal(self.0.saturating_sub(other.0))
    }
}

impl core::ops::Mul for Decimal {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        // Multiply and then divide by SCALE to maintain precision
        let result = (self.0 as i128)
            .saturating_mul(other.0 as i128)
            .saturating_div(SCALE as i128);
        Decimal(result as i64)
    }
}

impl core::ops::Div for Decimal {
    type Output = Result<Self, ()>;

    fn div(self, other: Self) -> Result<Self, ()> {
        if other.0 == 0 {
            return Err(());
        }

        // Multiply numerator by SCALE before dividing to maintain precision
        let result = (self.0 as i128)
            .saturating_mul(SCALE as i128)
            .saturating_div(other.0 as i128);
        Ok(Decimal(result as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer() {
        assert_eq!(Decimal::from_str("5").unwrap(), Decimal::from_i64(5));
        assert_eq!(Decimal::from_str("0").unwrap(), Decimal::ZERO);
        assert_eq!(Decimal::from_str("-3").unwrap(), Decimal::from_i64(-3));
    }

    #[test]
    fn test_parse_decimal() {
        let d = Decimal::from_str("3.14").unwrap();
        assert_eq!(d.0, 3_140_000);
    }

    #[test]
    fn test_addition() {
        let a = Decimal::from_str("2.5").unwrap();
        let b = Decimal::from_str("3.7").unwrap();
        let result = a + b;
        let mut s = heapless::String::new();
        result.format_to_string(&mut s).unwrap();
        assert_eq!(s.as_str(), "6.2");
    }

    #[test]
    fn test_multiplication() {
        let a = Decimal::from_str("2.5").unwrap();
        let b = Decimal::from_str("4").unwrap();
        let result = a * b;
        let mut s = heapless::String::new();
        result.format_to_string(&mut s).unwrap();
        assert_eq!(s.as_str(), "10");
    }

    #[test]
    fn test_division() {
        let a = Decimal::from_str("10").unwrap();
        let b = Decimal::from_str("4").unwrap();
        let result = (a / b).unwrap();
        let mut s = heapless::String::new();
        result.format_to_string(&mut s).unwrap();
        assert_eq!(s.as_str(), "2.5");
    }

    #[test]
    fn test_division_by_zero() {
        let a = Decimal::from_str("10").unwrap();
        let b = Decimal::ZERO;
        assert!(matches!(a / b, Err(())));
    }
}
