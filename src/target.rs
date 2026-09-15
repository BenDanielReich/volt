/// A backend description. Volt emits C; the target only changes headers,
/// pointer width, and how interrupts are wired.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub name: &'static str,
    pub pointer_width: u8,
    pub includes: &'static [&'static str],
    pub isr_style: IsrStyle,
    pub defines: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsrStyle {
    None,
    Avr,
    CortexM,
}

pub fn all_targets() -> &'static [Target] {
    static ALL: [Target; 3] = [HOST, AVR_ATMEGA328P, CORTEX_M0];
    &ALL
}

pub fn find_target(name: &str) -> Option<&'static Target> {
    all_targets().iter().find(|t| t.name == name)
}

pub static HOST: Target = Target {
    name: "host",
    pointer_width: 64,
    includes: &["<stdint.h>", "<stdbool.h>"],
    isr_style: IsrStyle::None,
    defines: &[],
};

pub static AVR_ATMEGA328P: Target = Target {
    name: "avr-atmega328p",
    pointer_width: 16,
    includes: &[
        "<stdint.h>",
        "<stdbool.h>",
        "<avr/io.h>",
        "<avr/interrupt.h>",
    ],
    isr_style: IsrStyle::Avr,
    defines: &[("F_CPU", "16000000UL")],
};

pub static CORTEX_M0: Target = Target {
    name: "cortex-m0",
    pointer_width: 32,
    includes: &["<stdint.h>", "<stdbool.h>"],
    isr_style: IsrStyle::CortexM,
    defines: &[],
};
