pub mod numpad;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format, Default)]
pub enum Mode {
    #[default]
    Numpad,
    Calculator,
    M2,
    M3,
}

impl Mode {
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Numpad => "[NUM]",
            Mode::Calculator => "[CALC]",
            Mode::M2 => "[M2]",
            Mode::M3 => "[M3]",
        }
    }
}
