//! Boards that Arduino IDE 2 installs on first run (`arduino:avr`) plus a few
//! extra cores. IDs and USB VID/PID match ArduinoCore-avr `boards.txt`.
//! FQBN format is the same as Arduino CLI: `vendor:arch:board`.

use crate::target::{self, Family, Target, HOST};

#[derive(Debug, Clone, Copy)]
pub struct Board {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub display: &'static str,
    pub mcu: &'static str,
    pub target: Target,
    pub analog_base: u8,
    pub analog_count: u8,
    pub define: &'static str,
    pub fqbn: Option<&'static str>,
    pub package: &'static str,
    pub usb: &'static [(u16, u16)],
    pub cc: &'static str,
    pub cc_flags: &'static str,
    pub flash: &'static str,
}

const F_16MHZ: &[(&str, &str)] = &[("F_CPU", "16000000UL")];
const F_8MHZ: &[(&str, &str)] = &[("F_CPU", "8000000UL")];

pub fn all_boards() -> &'static [Board] {
    BOARDS
}

pub fn all_targets() -> Vec<Target> {
    BOARDS.iter().map(|b| b.target).collect()
}

/// Resolve a Volt board id, Arduino FQBN (`arduino:avr:uno`), or display name.
pub fn find_board(name: &str) -> Option<&'static Board> {
    let raw = name.trim();
    if raw.is_empty() {
        return None;
    }
    let fqbn = sanitize_fqbn(raw);
    // Prefer an exact id / FQBN / alias hit so `nano:cpu=atmega328old` is not
    // collapsed onto the 115200 Nano.
    BOARDS
        .iter()
        .find(|b| board_matches_exact(b, raw))
        .or_else(|| {
            BOARDS.iter().find(|b| {
                b.fqbn.map(|f| eq(f, &fqbn)).unwrap_or(false)
                    || b.aliases.iter().any(|a| eq(a, &fqbn))
            })
        })
}

fn board_matches_exact(b: &Board, raw: &str) -> bool {
    eq(b.name, raw)
        || eq(b.display, raw)
        || eq(b.target.name, raw)
        || b.fqbn.map(|f| eq(f, raw)).unwrap_or(false)
        || b.aliases.iter().any(|a| eq(a, raw))
}

/// USB VID/PID pair as Arduino IDE `board list` uses for auto-detect.
pub fn board_for_usb(vid: u16, pid: u16) -> Option<&'static Board> {
    BOARDS
        .iter()
        .find(|b| b.usb.iter().any(|(v, p)| *v == vid && *p == pid))
}

pub fn known_names() -> String {
    format_board_list(BOARDS.iter())
}

pub fn search_boards(query: &str) -> Vec<&'static Board> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return BOARDS.iter().collect();
    }
    BOARDS
        .iter()
        .filter(|b| board_haystack(b).contains(&q))
        .collect()
}

/// Error text for `--board` / `--target` when nothing matches exactly.
pub fn unknown_board_message(query: &str) -> String {
    let q = query.trim();
    let mut msg = format!("unknown board `{q}`");
    let hits = search_boards(q);
    let had_hits = !hits.is_empty();
    let suggestions = if had_hits {
        hits
    } else {
        closest_boards(q, 6)
    };
    if suggestions.is_empty() {
        msg.push_str("\nnote: run `voltc boards` to list ids, or `voltc boards <query>` to search");
        return msg;
    }
    if had_hits {
        msg.push_str("\nmatching boards:");
    } else {
        msg.push_str("\ndid you mean:");
    }
    msg.push('\n');
    msg.push_str(&format_board_list(suggestions.into_iter()));
    msg.push_str("\nnote: run `voltc boards` to list ids, or `voltc boards <query>` to search");
    msg
}

fn format_board_list<'a, I>(boards: I) -> String
where
    I: Iterator<Item = &'a Board>,
{
    let mut lines = Vec::new();
    for b in boards {
        let fqbn = b.fqbn.unwrap_or("-");
        lines.push(format!("  {:<14} {:<28} {}", b.name, b.display, fqbn));
    }
    lines.join("\n")
}

fn board_haystack(b: &Board) -> String {
    let mut parts = vec![b.name, b.display, b.mcu, b.package, b.target.name];
    if let Some(fqbn) = b.fqbn {
        parts.push(fqbn);
    }
    parts.extend(b.aliases.iter().copied());
    parts.join(" ").to_ascii_lowercase()
}

fn closest_boards(query: &str, limit: usize) -> Vec<&'static Board> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(usize, &Board)> = BOARDS
        .iter()
        .map(|b| {
            let mut dist = edit_distance(&q, &b.name.to_ascii_lowercase());
            dist = dist.min(edit_distance(&q, &b.display.to_ascii_lowercase()));
            for alias in b.aliases {
                dist = dist.min(edit_distance(&q, &alias.to_ascii_lowercase()));
            }
            (dist, b)
        })
        .collect();
    scored.sort_by_key(|(d, b)| (*d, b.name));
    let max = (q.len() / 2).max(2);
    scored
        .into_iter()
        .filter(|(d, _)| *d <= max)
        .take(limit)
        .map(|(_, b)| b)
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

pub fn runtime_c(family: Family) -> &'static str {
    match family {
        Family::Host => include_str!("../runtime/host/volt_rt.c"),
        Family::Avr => include_str!("../runtime/avr/volt_rt.c"),
        Family::Rp2040 => include_str!("../runtime/rp2040/volt_rt.c"),
        Family::Stm32F1 => include_str!("../runtime/stm32f1/volt_rt.c"),
        Family::Stm32F4 => include_str!("../runtime/stm32f4/volt_rt.c"),
        Family::Esp32 => include_str!("../runtime/esp32/volt_rt.c"),
        Family::Esp8266 => include_str!("../runtime/esp8266/volt_rt.c"),
        Family::Arm => include_str!("../runtime/arm/volt_rt.c"),
    }
}

pub fn board_defines(board: &Board) -> String {
    let mut out = String::new();
    out.push_str(&format!("#define VOLT_BOARD_{} 1\n", board.define));
    if let Some(fqbn) = board.fqbn {
        out.push_str(&format!("#define VOLT_FQBN \"{fqbn}\"\n"));
    }
    out
}

pub fn flash_comment(board: &Board) -> String {
    let port = crate::ports::preferred_port().unwrap_or_else(|| crate::ports::port_placeholder().to_string());
    let flash = board.flash.replace("<PORT>", &port);
    let pico = if board.target.family.rp2040() && cfg!(target_os = "macos") {
        "  or:    cp firmware.uf2 /Volumes/RPI-RP2/\n"
    } else {
        ""
    };
    format!(
        "  board: {} ({})\n  mcu:   {}\n  led:   pin {}\n  fqbn:  {}\n  os:    {}\n  port:  {}\n  build: {} {} <file.c> -o firmware\n  flash: {}\n{}{}",
        board.display,
        board.name,
        board.mcu,
        board.target.led,
        board.fqbn.unwrap_or("-"),
        crate::ports::os_name(),
        port,
        board.cc,
        board.cc_flags,
        flash,
        board
            .fqbn
            .map(|f| format!("  or:    arduino-cli compile -b {f} && arduino-cli upload -b {f} -p {port}\n"))
            .unwrap_or_default(),
        pico
    )
}

fn sanitize_fqbn(s: &str) -> String {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() >= 3 {
        format!("{}:{}:{}", parts[0], parts[1], parts[2])
    } else {
        s.to_string()
    }
}

fn eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

const fn avr328(
    name: &'static str,
    display: &'static str,
    analog_count: u8,
    define: &'static str,
    fqbn: &'static str,
    aliases: &'static [&'static str],
    usb: &'static [(u16, u16)],
    flash: &'static str,
) -> Board {
    Board {
        name,
        aliases,
        display,
        mcu: "ATmega328P",
        target: target::avr(name, 2048, 16_000_000, 13, F_16MHZ),
        analog_base: 14,
        analog_count,
        define,
        fqbn: Some(fqbn),
        package: "arduino:avr",
        usb,
        cc: "avr-gcc",
        cc_flags: "-mmcu=atmega328p -Os -DF_CPU=16000000UL",
        flash,
    }
}

const fn avr32u4(
    name: &'static str,
    display: &'static str,
    fqbn: &'static str,
    aliases: &'static [&'static str],
    usb: &'static [(u16, u16)],
) -> Board {
    Board {
        name,
        aliases,
        display,
        mcu: "ATmega32U4",
        target: target::avr(name, 2560, 16_000_000, 13, F_16MHZ),
        analog_base: 18,
        analog_count: 12,
        define: "LEONARDO",
        fqbn: Some(fqbn),
        package: "arduino:avr",
        usb,
        cc: "avr-gcc",
        cc_flags: "-mmcu=atmega32u4 -Os -DF_CPU=16000000UL",
        flash: "avrdude -c avr109 -p atmega32u4 -P <PORT> -U flash:w:firmware.hex:i",
    }
}

static BOARDS: &[Board] = &[
    Board {
        name: "host",
        aliases: &["native", "desktop"],
        display: "Host (desktop)",
        mcu: "host",
        target: HOST,
        analog_base: 14,
        analog_count: 6,
        define: "HOST",
        fqbn: None,
        package: "volt",
        usb: &[],
        cc: "cc",
        cc_flags: "-O2",
        flash: "(run the host binary)",
    },
    // Arduino AVR Boards — same menu Arduino IDE 2 installs on first launch.
    avr328(
        "uno",
        "Arduino UNO",
        6,
        "UNO",
        "arduino:avr:uno",
        &["arduino-uno", "avr-atmega328p", "atmega328p"],
        &[
            (0x2341, 0x0043),
            (0x2341, 0x0001),
            (0x2A03, 0x0043),
            (0x2341, 0x0243),
            (0x2341, 0x006A),
        ],
        "avrdude -c arduino -p atmega328p -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    avr328(
        "unomini",
        "Arduino UNO Mini",
        6,
        "UNO",
        "arduino:avr:unomini",
        &["uno-mini"],
        &[(0x2341, 0x0062)],
        "avrdude -c arduino -p atmega328p -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    avr328(
        "nano",
        "Arduino Nano",
        8,
        "NANO",
        "arduino:avr:nano",
        &[
            "arduino-nano",
            "nano328",
            "nano-328",
            "arduino:avr:nano:cpu=atmega328",
        ],
        &[
            (0x0403, 0x6001),
            (0x1A86, 0x7523),
            (0x1A86, 0x5523),
            (0x1A86, 0x55D4),
            (0x10C4, 0xEA60),
        ],
        "avrdude -c arduino -p atmega328p -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    avr328(
        "nano-old",
        "Arduino Nano (Old Bootloader)",
        8,
        "NANO",
        "arduino:avr:nano:cpu=atmega328old",
        &[
            "nanoold",
            "nano-328old",
            "arduino-nano-old",
        ],
        &[
            (0x0403, 0x6001),
            (0x1A86, 0x7523),
            (0x1A86, 0x5523),
            (0x1A86, 0x55D4),
            (0x10C4, 0xEA60),
        ],
        "avrdude -c arduino -p atmega328p -b 57600 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    avr328(
        "diecimila",
        "Arduino Duemilanove or Diecimila",
        6,
        "UNO",
        "arduino:avr:diecimila",
        &["duemilanove"],
        &[],
        "avrdude -c arduino -p atmega328p -b 57600 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    avr328(
        "mini",
        "Arduino Mini",
        8,
        "NANO",
        "arduino:avr:mini",
        &[],
        &[],
        "avrdude -c arduino -p atmega328p -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    avr328(
        "ethernet",
        "Arduino Ethernet",
        6,
        "UNO",
        "arduino:avr:ethernet",
        &[],
        &[],
        "avrdude -c arduino -p atmega328p -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    avr328(
        "unowifi",
        "Arduino UNO WiFi",
        6,
        "UNO",
        "arduino:avr:unowifi",
        &["uno-wifi"],
        &[(0x2A03, 0x0057)],
        "avrdude -c arduino -p atmega328p -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    ),
    Board {
        name: "pro",
        aliases: &["pro-mini", "promini", "arduino-pro-mini"],
        display: "Arduino Pro or Pro Mini",
        mcu: "ATmega328P",
        target: target::avr("pro", 2048, 16_000_000, 13, F_16MHZ),
        analog_base: 14,
        analog_count: 8,
        define: "NANO",
        fqbn: Some("arduino:avr:pro"),
        package: "arduino:avr",
        usb: &[],
        cc: "avr-gcc",
        cc_flags: "-mmcu=atmega328p -Os -DF_CPU=16000000UL",
        flash: "avrdude -c arduino -p atmega328p -b 57600 -P <PORT> -U flash:w:firmware.hex:i",
    },
    Board {
        name: "fio",
        aliases: &[],
        display: "Arduino Fio",
        mcu: "ATmega328P",
        target: target::avr("fio", 2048, 8_000_000, 13, F_8MHZ),
        analog_base: 14,
        analog_count: 8,
        define: "NANO",
        fqbn: Some("arduino:avr:fio"),
        package: "arduino:avr",
        usb: &[],
        cc: "avr-gcc",
        cc_flags: "-mmcu=atmega328p -Os -DF_CPU=8000000UL",
        flash: "avrdude -c arduino -p atmega328p -b 57600 -P <PORT> -U flash:w:firmware.hex:i",
    },
    Board {
        name: "lilypad",
        aliases: &[],
        display: "LilyPad Arduino",
        mcu: "ATmega328P",
        target: target::avr("lilypad", 2048, 8_000_000, 13, F_8MHZ),
        analog_base: 14,
        analog_count: 6,
        define: "UNO",
        fqbn: Some("arduino:avr:lilypad"),
        package: "arduino:avr",
        usb: &[],
        cc: "avr-gcc",
        cc_flags: "-mmcu=atmega328p -Os -DF_CPU=8000000UL",
        flash: "avrdude -c arduino -p atmega328p -b 57600 -P <PORT> -U flash:w:firmware.hex:i",
    },
    avr32u4(
        "leonardo",
        "Arduino Leonardo",
        "arduino:avr:leonardo",
        &["arduino-leonardo"],
        &[
            (0x2341, 0x0036),
            (0x2341, 0x8036),
            (0x2A03, 0x0036),
            (0x2A03, 0x8036),
        ],
    ),
    avr32u4(
        "micro",
        "Arduino Micro",
        "arduino:avr:micro",
        &["arduino-micro"],
        &[
            (0x2341, 0x0037),
            (0x2341, 0x8037),
            (0x2A03, 0x0037),
            (0x2A03, 0x8037),
            (0x2341, 0x0237),
            (0x2341, 0x8237),
        ],
    ),
    avr32u4(
        "yun",
        "Arduino Yún",
        "arduino:avr:yun",
        &[],
        &[
            (0x2341, 0x0041),
            (0x2341, 0x8041),
            (0x2A03, 0x0041),
            (0x2A03, 0x8041),
        ],
    ),
    avr32u4(
        "yunmini",
        "Arduino Yún Mini",
        "arduino:avr:yunmini",
        &[],
        &[(0x2A03, 0x0050), (0x2A03, 0x8050)],
    ),
    avr32u4(
        "leonardoeth",
        "Arduino Leonardo ETH",
        "arduino:avr:leonardoeth",
        &[],
        &[(0x2A03, 0x0040), (0x2A03, 0x8040)],
    ),
    avr32u4(
        "esplora",
        "Arduino Esplora",
        "arduino:avr:esplora",
        &[],
        &[
            (0x2341, 0x003C),
            (0x2341, 0x803C),
            (0x2A03, 0x003C),
            (0x2A03, 0x803C),
        ],
    ),
    avr32u4(
        "LilyPadUSB",
        "LilyPad Arduino USB",
        "arduino:avr:LilyPadUSB",
        &["lilypad-usb"],
        &[(0x1B4F, 0x9207), (0x1B4F, 0x9208)],
    ),
    avr32u4(
        "chiwawa",
        "Arduino Industrial 101",
        "arduino:avr:chiwawa",
        &["industrial101"],
        &[(0x2A03, 0x0056), (0x2A03, 0x8056)],
    ),
    avr32u4(
        "one",
        "Linino One",
        "arduino:avr:one",
        &[],
        &[(0x2A03, 0x0001), (0x2A03, 0x8001)],
    ),
    Board {
        name: "mega",
        aliases: &["mega2560", "arduino-mega"],
        display: "Arduino Mega or Mega 2560",
        mcu: "ATmega2560",
        target: target::avr("mega", 8192, 16_000_000, 13, F_16MHZ),
        analog_base: 54,
        analog_count: 16,
        define: "MEGA",
        fqbn: Some("arduino:avr:mega"),
        package: "arduino:avr",
        usb: &[
            (0x2341, 0x0010),
            (0x2341, 0x0042),
            (0x2A03, 0x0010),
            (0x2A03, 0x0042),
            (0x2341, 0x0210),
            (0x2341, 0x0242),
        ],
        cc: "avr-gcc",
        cc_flags: "-mmcu=atmega2560 -Os -DF_CPU=16000000UL",
        flash: "avrdude -c wiring -p atmega2560 -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    },
    Board {
        name: "megaADK",
        aliases: &["mega-adk"],
        display: "Arduino Mega ADK",
        mcu: "ATmega2560",
        target: target::avr("megaADK", 8192, 16_000_000, 13, F_16MHZ),
        analog_base: 54,
        analog_count: 16,
        define: "MEGA",
        fqbn: Some("arduino:avr:megaADK"),
        package: "arduino:avr",
        usb: &[
            (0x2341, 0x003F),
            (0x2341, 0x0044),
            (0x2A03, 0x003F),
            (0x2A03, 0x0044),
        ],
        cc: "avr-gcc",
        cc_flags: "-mmcu=atmega2560 -Os -DF_CPU=16000000UL",
        flash: "avrdude -c wiring -p atmega2560 -b 115200 -P <PORT> -U flash:w:firmware.hex:i",
    },
    Board {
        name: "gemma",
        aliases: &[],
        display: "Arduino Gemma",
        mcu: "ATtiny85",
        target: target::avr("gemma", 512, 8_000_000, 1, F_8MHZ),
        analog_base: 0,
        analog_count: 3,
        define: "GEMMA",
        fqbn: Some("arduino:avr:gemma"),
        package: "arduino:avr",
        usb: &[(0x2341, 0x0C9F)],
        cc: "avr-gcc",
        cc_flags: "-mmcu=attiny85 -Os -DF_CPU=8000000UL",
        flash: "avrdude -c usbtiny -p attiny85 -U flash:w:firmware.hex:i",
    },
    // Extra cores (Boards Manager in Arduino IDE).
    Board {
        name: "pico",
        aliases: &["raspberry-pi-pico", "rp2040", "rpipico"],
        display: "Raspberry Pi Pico",
        mcu: "RP2040",
        target: target::arm("pico", Family::Rp2040, 264 * 1024, 133_000_000, 25, false),
        analog_base: 26,
        analog_count: 3,
        define: "PICO",
        fqbn: Some("rp2040:rp2040:rpipico"),
        package: "rp2040:rp2040",
        usb: &[(0x2E8A, 0x000A)],
        cc: "arm-none-eabi-gcc",
        cc_flags: "-mcpu=cortex-m0plus -mthumb -Os",
        flash: "picotool load firmware.uf2  (or copy to the RPI-RP2 drive)",
    },
    Board {
        name: "pico-w",
        aliases: &["raspberry-pi-pico-w", "rpipicow"],
        display: "Raspberry Pi Pico W",
        mcu: "RP2040 + CYW43439",
        target: target::arm("pico-w", Family::Rp2040, 264 * 1024, 133_000_000, 0, false),
        analog_base: 26,
        analog_count: 3,
        define: "PICO",
        fqbn: Some("rp2040:rp2040:rpipicow"),
        package: "rp2040:rp2040",
        usb: &[(0x2E8A, 0x000D)],
        cc: "arm-none-eabi-gcc",
        cc_flags: "-mcpu=cortex-m0plus -mthumb -Os",
        flash: "picotool load firmware.uf2  (LED_BUILTIN is GP0; onboard LED is CYW43439)",
    },
    Board {
        name: "bluepill",
        aliases: &["stm32f103", "stm32f103c8", "blue-pill"],
        display: "STM32 Blue Pill",
        mcu: "STM32F103C8",
        target: target::arm("bluepill", Family::Stm32F1, 20 * 1024, 72_000_000, 45, false),
        analog_base: 0,
        analog_count: 10,
        define: "BLUEPILL",
        fqbn: Some("STMicroelectronics:stm32:GenF1"),
        package: "STMicroelectronics:stm32",
        usb: &[],
        cc: "arm-none-eabi-gcc",
        cc_flags: "-mcpu=cortex-m3 -mthumb -Os",
        flash: "st-flash write firmware.bin 0x8000000  (LED_BUILTIN is PC13 = pin 45)",
    },
    Board {
        name: "nucleo-f401re",
        aliases: &["nucleo", "nucleo-f401", "stm32f401"],
        display: "STM32 Nucleo-F401RE",
        mcu: "STM32F401RE",
        target: target::arm(
            "nucleo-f401re",
            Family::Stm32F4,
            96 * 1024,
            84_000_000,
            5,
            true,
        ),
        analog_base: 0,
        analog_count: 16,
        define: "NUCLEO_F401RE",
        fqbn: Some("STMicroelectronics:stm32:Nucleo_64"),
        package: "STMicroelectronics:stm32",
        usb: &[(0x0483, 0x374B)],
        cc: "arm-none-eabi-gcc",
        cc_flags: "-mcpu=cortex-m4 -mthumb -mfpu=fpv4-sp-d16 -mfloat-abi=hard -Os",
        flash: "st-flash write firmware.bin 0x8000000  (LED_BUILTIN is PA5 / D13)",
    },
    Board {
        name: "esp32",
        aliases: &["esp32-devkit", "esp32-devkitc"],
        display: "ESP32 DevKit",
        mcu: "ESP32",
        target: target::esp("esp32", 520 * 1024, 240_000_000, 2),
        analog_base: 32,
        analog_count: 8,
        define: "ESP32",
        fqbn: Some("esp32:esp32:esp32"),
        package: "esp32:esp32",
        usb: &[(0x10C4, 0xEA60), (0x1A86, 0x7523)],
        cc: "xtensa-esp32-elf-gcc",
        cc_flags: "-Os",
        flash: "esptool.py --chip esp32 write_flash 0x10000 firmware.bin",
    },
    Board {
        name: "nano-esp32",
        aliases: &[
            "nano_esp32",
            "nano-nora",
            "arduino-nano-esp32",
            "arduino:esp32:nano_nora",
        ],
        display: "Arduino Nano ESP32",
        mcu: "ESP32-S3",
        target: target::esp("nano-esp32", 512 * 1024, 240_000_000, 13),
        analog_base: 14,
        analog_count: 8,
        define: "NANO_ESP32",
        fqbn: Some("arduino:esp32:nano_nora"),
        package: "arduino:esp32",
        usb: &[(0x2341, 0x0070), (0x2341, 0x0071)],
        cc: "xtensa-esp32s3-elf-gcc",
        cc_flags: "-Os",
        flash: "esptool.py --chip esp32s3 -p <PORT> write_flash 0x0 firmware.bin",
    },
    Board {
        name: "esp8000",
        aliases: &[
            "esp-8000",
            "esp8285",
            "esp8266",
            "nodemcu",
            "nodemcuv2",
            "esp-12e",
            "esp-12f",
            "esp8266:esp8266:generic",
            "esp8266:esp8266:nodemcuv2",
        ],
        display: "ESP8000 (ESP8266)",
        mcu: "ESP8266EX",
        target: target::esp8266("esp8000", 80 * 1024, 80_000_000, 2),
        analog_base: 17,
        analog_count: 1,
        define: "ESP8000",
        fqbn: Some("esp8266:esp8266:generic"),
        package: "esp8266:esp8266",
        usb: &[(0x10C4, 0xEA60), (0x1A86, 0x7523), (0x1A86, 0x55D4)],
        cc: "xtensa-lx106-elf-gcc",
        cc_flags: "-Os -DICACHE_FLASH",
        flash: "esptool.py --chip esp8266 -p <PORT> write_flash -fs 4MB 0x0 firmware.bin",
    },
    Board {
        name: "cortex-m0",
        aliases: &["m0", "arm-m0"],
        display: "Generic Cortex-M0",
        mcu: "Cortex-M0",
        target: target::CORTEX_M0,
        analog_base: 14,
        analog_count: 6,
        define: "ARM",
        fqbn: None,
        package: "volt",
        usb: &[],
        cc: "arm-none-eabi-gcc",
        cc_flags: "-mcpu=cortex-m0 -mthumb -Os",
        flash: "(link with your MCU's startup and ld script)",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arduino_uno_fqbn() {
        let b = find_board("arduino:avr:uno").expect("uno");
        assert_eq!(b.display, "Arduino UNO");
        assert_eq!(b.target.led, 13);
        assert_eq!(board_for_usb(0x2341, 0x0043).unwrap().name, "uno");
    }

    #[test]
    fn nano_cpu_option_stripped() {
        let b = find_board("arduino:avr:nano:cpu=atmega328old").expect("nano-old");
        assert_eq!(b.name, "nano-old");
        assert!(b.flash.contains("57600"));
        let n = find_board("arduino:avr:nano").expect("nano");
        assert_eq!(n.name, "nano");
        assert!(n.flash.contains("115200"));
        assert_eq!(board_for_usb(0x1A86, 0x7523).unwrap().name, "nano");
    }

    #[test]
    fn micro_is_not_leonardo() {
        assert_eq!(find_board("micro").unwrap().name, "micro");
        assert_eq!(find_board("arduino:avr:micro").unwrap().fqbn, Some("arduino:avr:micro"));
    }

    #[test]
    fn esp8000_is_esp8266() {
        let b = find_board("esp8000").expect("esp8000");
        assert_eq!(b.mcu, "ESP8266EX");
        assert_eq!(b.target.led, 2);
        assert_eq!(b.target.family, Family::Esp8266);
        assert_eq!(find_board("nodemcu").unwrap().name, "esp8000");
        assert_eq!(find_board("esp8266:esp8266:generic").unwrap().name, "esp8000");
    }

    #[test]
    fn nano_esp32_fqbn() {
        let b = find_board("nano-esp32").expect("nano-esp32");
        assert_eq!(b.fqbn, Some("arduino:esp32:nano_nora"));
        assert_eq!(b.target.led, 13);
        assert_eq!(find_board("arduino:esp32:nano_nora").unwrap().name, "nano-esp32");
    }

    #[test]
    fn search_finds_nano_and_esp8000() {
        let nanos: Vec<_> = search_boards("nano").iter().map(|b| b.name).collect();
        assert!(nanos.contains(&"nano"));
        assert!(nanos.contains(&"nano-old"));
        assert!(nanos.contains(&"nano-esp32"));
        assert_eq!(search_boards("esp8000")[0].name, "esp8000");
        assert!(search_boards("nodemcu").iter().any(|b| b.name == "esp8000"));
    }

    #[test]
    fn unknown_board_suggests_matches() {
        let msg = unknown_board_message("esp");
        assert!(msg.contains("unknown board `esp`"));
        assert!(msg.contains("matching boards:"));
        assert!(msg.contains("esp8000"));
        let typo = unknown_board_message("unoo");
        assert!(typo.contains("did you mean:"));
        assert!(typo.contains("uno"));
        let nonsense = unknown_board_message("zzzz-not-a-board");
        assert!(nonsense.contains("unknown board"));
        assert!(nonsense.contains("voltc boards"));
    }
}
