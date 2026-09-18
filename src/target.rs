/// A backend description. Volt emits C; the target only changes headers,
/// pointer width, how interrupts are wired, and which board pack is linked.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub name: &'static str,
    pub pointer_width: u8,
    pub includes: &'static [&'static str],
    pub isr_style: IsrStyle,
    pub defines: &'static [(&'static str, &'static str)],
    pub has_fpu: bool,
    pub ram_bytes: u32,
    pub f_cpu: u64,
    pub family: Family,
    pub led: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsrStyle {
    None,
    Avr,
    CortexM,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Host,
    Avr,
    Rp2040,
    Stm32F1,
    Stm32F4,
    Esp32,
    Esp8266,
    Arm,
}

impl Family {
    pub fn avr(self) -> bool {
        self == Family::Avr
    }

    pub fn arm(self) -> bool {
        matches!(
            self,
            Family::Arm | Family::Rp2040 | Family::Stm32F1 | Family::Stm32F4
        )
    }

    pub fn rp2040(self) -> bool {
        self == Family::Rp2040
    }

    pub fn stm32(self) -> bool {
        matches!(self, Family::Stm32F1 | Family::Stm32F4)
    }

    pub fn esp32(self) -> bool {
        self == Family::Esp32
    }

    pub fn esp8266(self) -> bool {
        self == Family::Esp8266
    }

    pub fn esp(self) -> bool {
        matches!(self, Family::Esp32 | Family::Esp8266)
    }
}

pub fn all_targets() -> Vec<Target> {
    crate::board::all_targets()
}

pub fn find_target(name: &str) -> Option<&'static Target> {
    crate::board::find_board(name).map(|b| &b.target)
}

pub static HOST: Target = Target {
    name: "host",
    pointer_width: 64,
    includes: &["<stdint.h>", "<stdbool.h>"],
    isr_style: IsrStyle::None,
    defines: &[],
    has_fpu: true,
    ram_bytes: 8 * 1024 * 1024,
    f_cpu: 0,
    family: Family::Host,
    led: 13,
};

const AVR_HEADERS: &[&str] = &[
    "<stdint.h>",
    "<stdbool.h>",
    "<avr/io.h>",
    "<avr/interrupt.h>",
];

const C_HEADERS: &[&str] = &["<stdint.h>", "<stdbool.h>"];

pub static AVR_ATMEGA328P: Target = Target {
    name: "avr-atmega328p",
    pointer_width: 16,
    includes: AVR_HEADERS,
    isr_style: IsrStyle::Avr,
    defines: &[("F_CPU", "16000000UL")],
    has_fpu: false,
    ram_bytes: 2048,
    f_cpu: 16_000_000,
    family: Family::Avr,
    led: 13,
};

pub static CORTEX_M0: Target = Target {
    name: "cortex-m0",
    pointer_width: 32,
    includes: C_HEADERS,
    isr_style: IsrStyle::CortexM,
    defines: &[],
    has_fpu: false,
    ram_bytes: 4096,
    f_cpu: 48_000_000,
    family: Family::Arm,
    led: 13,
};

pub(crate) const fn avr(
    name: &'static str,
    ram_bytes: u32,
    f_cpu: u64,
    led: u8,
    defines: &'static [(&'static str, &'static str)],
) -> Target {
    Target {
        name,
        pointer_width: 16,
        includes: AVR_HEADERS,
        isr_style: IsrStyle::Avr,
        defines,
        has_fpu: false,
        ram_bytes,
        f_cpu,
        family: Family::Avr,
        led,
    }
}

pub(crate) const fn arm(
    name: &'static str,
    family: Family,
    ram_bytes: u32,
    f_cpu: u64,
    led: u8,
    has_fpu: bool,
) -> Target {
    Target {
        name,
        pointer_width: 32,
        includes: C_HEADERS,
        isr_style: IsrStyle::CortexM,
        defines: &[],
        has_fpu,
        ram_bytes,
        f_cpu,
        family,
        led,
    }
}

pub(crate) const fn esp(name: &'static str, ram_bytes: u32, f_cpu: u64, led: u8) -> Target {
    Target {
        name,
        pointer_width: 32,
        includes: C_HEADERS,
        isr_style: IsrStyle::None,
        defines: &[],
        has_fpu: true,
        ram_bytes,
        f_cpu,
        family: Family::Esp32,
        led,
    }
}

pub(crate) const fn esp8266(name: &'static str, ram_bytes: u32, f_cpu: u64, led: u8) -> Target {
    Target {
        name,
        pointer_width: 32,
        includes: C_HEADERS,
        isr_style: IsrStyle::None,
        defines: &[],
        has_fpu: false,
        ram_bytes,
        f_cpu,
        family: Family::Esp8266,
        led,
    }
}
