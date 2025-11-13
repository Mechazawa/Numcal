#![cfg_attr(not(test), no_std)]

// Only include calculator for testing
#[cfg(test)]
#[path = "modes/calculator.rs"]
mod calculator;

#[cfg(test)]
pub use calculator::*;
