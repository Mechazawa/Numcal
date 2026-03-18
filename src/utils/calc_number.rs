use core::cmp::Ordering;

use heapless::Vec;

const MAX_DIGITS: usize = 32;

#[derive(Clone, Debug)]
pub struct CalcNumber {
    /// Digits stored most-significant first, each 0-9
    pub digits: Vec<u8, MAX_DIGITS>,
    /// Number of digits after the decimal point (0 = integer)
    pub decimal_places: u8,
    pub negative: bool,
}

impl CalcNumber {
    pub fn zero() -> Self {
        let mut digits = Vec::new();
        digits.push(0).ok();
        Self { digits, decimal_places: 0, negative: false }
    }

    pub fn is_zero(&self) -> bool {
        self.digits.iter().all(|&d| d == 0)
    }

    fn trim_leading_zeros(&mut self) {
        let integer_digits = self.digits.len() - usize::from(self.decimal_places);
        let mut leading = 0usize;
        for &d in &self.digits {
            if d == 0 && leading + 1 < integer_digits {
                leading += 1;
            } else {
                break;
            }
        }
        if leading > 0 {
            let mut trimmed: Vec<u8, MAX_DIGITS> = Vec::new();
            for i in leading..self.digits.len() {
                trimmed.push(self.digits[i]).ok();
            }
            self.digits = trimmed;
        }
    }

    fn trim_trailing_zeros(&mut self) {
        while self.decimal_places > 0 {
            if self.digits.last() == Some(&0) {
                self.digits.pop();
                self.decimal_places -= 1;
            } else {
                break;
            }
        }
    }

    fn normalize(&mut self) {
        self.trim_leading_zeros();
        self.trim_trailing_zeros();
        if self.is_zero() {
            self.negative = false;
        }
    }

    pub fn to_display_string(&self) -> heapless::String<20> {
        let mut s: heapless::String<20> = heapless::String::new();
        if self.negative && !self.is_zero() {
            s.push('-').ok();
        }

        let integer_len = self.digits.len() - usize::from(self.decimal_places);
        for (i, &d) in self.digits.iter().enumerate() {
            if i == integer_len && self.decimal_places > 0 {
                s.push('.').ok();
            }
            s.push((b'0' + d) as char).ok();
        }
        s
    }

    /// Align two numbers to the same decimal places and integer width.
    fn align(a: &CalcNumber, b: &CalcNumber) -> (Vec<u8, MAX_DIGITS>, Vec<u8, MAX_DIGITS>, u8) {
        let max_dp = a.decimal_places.max(b.decimal_places);
        let mut ad: Vec<u8, MAX_DIGITS> = a.digits.clone();
        let mut bd: Vec<u8, MAX_DIGITS> = b.digits.clone();

        for _ in a.decimal_places..max_dp {
            ad.push(0).ok();
        }
        for _ in b.decimal_places..max_dp {
            bd.push(0).ok();
        }

        let max_dp_usize = usize::from(max_dp);
        let a_int = ad.len().saturating_sub(max_dp_usize);
        let b_int = bd.len().saturating_sub(max_dp_usize);
        let max_int = a_int.max(b_int);

        if a_int < max_int {
            ad = Self::prepend_zeros(&ad, max_int - a_int);
        }
        if b_int < max_int {
            bd = Self::prepend_zeros(&bd, max_int - b_int);
        }

        (ad, bd, max_dp)
    }

    fn prepend_zeros(src: &[u8], count: usize) -> Vec<u8, MAX_DIGITS> {
        let mut padded: Vec<u8, MAX_DIGITS> = Vec::new();
        for _ in 0..count {
            padded.push(0).ok();
        }
        padded.extend_from_slice(src).ok();
        padded
    }

    /// Compare magnitudes of two digit slices, handling different lengths.
    fn cmp_magnitude(a: &[u8], b: &[u8]) -> Ordering {
        let a_start = a.iter().position(|&x| x != 0).unwrap_or(a.len().saturating_sub(1));
        let b_start = b.iter().position(|&x| x != 0).unwrap_or(b.len().saturating_sub(1));
        let a_sig = &a[a_start..];
        let b_sig = &b[b_start..];

        match a_sig.len().cmp(&b_sig.len()) {
            Ordering::Equal => {
                for (da, db) in a_sig.iter().zip(b_sig.iter()) {
                    match da.cmp(db) {
                        Ordering::Equal => {}
                        other => return other,
                    }
                }
                Ordering::Equal
            }
            other => other,
        }
    }

    fn add_magnitudes(a: &[u8], b: &[u8]) -> Vec<u8, MAX_DIGITS> {
        let mut result: Vec<u8, MAX_DIGITS> = Vec::new();
        let mut carry: u8 = 0;

        for i in (0..a.len()).rev() {
            let sum = a[i] + b[i] + carry;
            carry = sum / 10;
            result.push(sum % 10).ok();
        }
        if carry > 0 {
            result.push(carry).ok();
        }
        result.reverse();
        result
    }

    /// Subtract magnitudes: a - b where a >= b (same-length aligned arrays).
    fn sub_magnitudes(a: &[u8], b: &[u8]) -> Vec<u8, MAX_DIGITS> {
        let mut result: Vec<u8, MAX_DIGITS> = Vec::new();
        let mut borrow: i16 = 0;

        for i in (0..a.len()).rev() {
            let mut diff = i16::from(a[i]) - i16::from(b[i]) - borrow;
            if diff < 0 {
                diff += 10;
                borrow = 1;
            } else {
                borrow = 0;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            result.push(diff as u8).ok();
        }
        result.reverse();
        result
    }

    fn trim_vec_leading_zeros(v: &mut Vec<u8, MAX_DIGITS>) {
        while v.len() > 1 && v[0] == 0 {
            let mut trimmed: Vec<u8, MAX_DIGITS> = Vec::new();
            for i in 1..v.len() {
                trimmed.push(v[i]).ok();
            }
            *v = trimmed;
        }
    }

    pub fn add(a: &CalcNumber, b: &CalcNumber) -> CalcNumber {
        let (ad, bd, dp) = Self::align(a, b);

        if a.negative == b.negative {
            let digits = Self::add_magnitudes(&ad, &bd);
            let mut r = CalcNumber { digits, decimal_places: dp, negative: a.negative };
            r.normalize();
            r
        } else {
            match Self::cmp_magnitude(&ad, &bd) {
                Ordering::Equal => CalcNumber::zero(),
                Ordering::Greater => {
                    let digits = Self::sub_magnitudes(&ad, &bd);
                    let mut r = CalcNumber { digits, decimal_places: dp, negative: a.negative };
                    r.normalize();
                    r
                }
                Ordering::Less => {
                    let digits = Self::sub_magnitudes(&bd, &ad);
                    let mut r = CalcNumber { digits, decimal_places: dp, negative: b.negative };
                    r.normalize();
                    r
                }
            }
        }
    }

    pub fn sub(a: &CalcNumber, b: &CalcNumber) -> CalcNumber {
        let mut neg_b = b.clone();
        neg_b.negative = !neg_b.negative;
        Self::add(a, &neg_b)
    }

    pub fn mul(a: &CalcNumber, b: &CalcNumber) -> CalcNumber {
        let a_len = a.digits.len();
        let b_len = b.digits.len();
        let result_len = a_len + b_len;

        let mut buf = [0u8; MAX_DIGITS * 2];
        for i in (0..a_len).rev() {
            let mut carry: u16 = 0;
            for j in (0..b_len).rev() {
                let pos = i + j + 1;
                let prod = u16::from(a.digits[i]) * u16::from(b.digits[j])
                    + u16::from(buf[pos]) + carry;
                #[allow(clippy::cast_possible_truncation)]
                { buf[pos] = (prod % 10) as u8; }
                carry = prod / 10;
            }
            #[allow(clippy::cast_possible_truncation)]
            { buf[i] += carry as u8; }
        }

        let dp = a.decimal_places + b.decimal_places;
        let negative = a.negative != b.negative;
        let min_digits = usize::from(dp) + 1;

        let mut digits: Vec<u8, MAX_DIGITS> = Vec::new();
        let mut started = false;
        let start = result_len.saturating_sub(MAX_DIGITS);
        for (idx, &val) in buf[start..result_len].iter().enumerate() {
            let remaining = result_len - start - idx;
            if val != 0 || started || remaining <= min_digits {
                started = true;
                if digits.push(val).is_err() {
                    break;
                }
            }
        }
        if digits.is_empty() {
            digits.push(0).ok();
        }

        let mut r = CalcNumber { digits, decimal_places: dp, negative };
        r.normalize();
        r
    }

    /// Long division. Returns None on division by zero.
    pub fn div(a: &CalcNumber, b: &CalcNumber) -> Option<CalcNumber> {
        if b.is_zero() {
            return None;
        }

        let negative = a.negative != b.negative;
        let target_dp: u8 = 10;

        // Scale dividend so that integer division yields the correct decimal places
        let extra_zeros = i16::from(target_dp) + i16::from(b.decimal_places)
            - i16::from(a.decimal_places);
        let mut dividend: Vec<u8, MAX_DIGITS> = a.digits.clone();
        if extra_zeros > 0 {
            for _ in 0..extra_zeros {
                dividend.push(0).ok();
            }
        }

        let divisor: &[u8] = &b.digits;
        let mut result: Vec<u8, MAX_DIGITS> = Vec::new();
        let mut remainder: Vec<u8, MAX_DIGITS> = Vec::new();
        remainder.push(0).ok();

        for &d in &dividend {
            // Shift remainder left and append next digit
            if remainder.len() == 1 && remainder[0] == 0 {
                remainder[0] = d;
            } else {
                remainder.push(d).ok();
            }

            // Count how many times divisor fits into remainder
            let mut count = 0u8;
            while Self::cmp_magnitude(&remainder, divisor) != Ordering::Less {
                let padded_div = Self::prepend_zeros(
                    divisor,
                    remainder.len() - divisor.len(),
                );
                remainder = Self::sub_magnitudes(&remainder, &padded_div);
                Self::trim_vec_leading_zeros(&mut remainder);
                count += 1;
            }

            result.push(count).ok();
        }

        if result.is_empty() {
            result.push(0).ok();
        }

        let mut r = CalcNumber { digits: result, decimal_places: target_dp, negative };
        r.normalize();
        Some(r)
    }

    pub fn negate(&mut self) {
        if !self.is_zero() {
            self.negative = !self.negative;
        }
    }
}
