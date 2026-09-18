module power

type adc12 = u12

reg u8 PORTB @ 0x25 {
    pb5: 5,
}

pin LED = PORTB.pb5

peripheral UART0

struct Pulse {
    u16 on_ms
}

trait Writer {
    function write(&mut self, u8 byte)
}

impl Pulse {
    function start(&mut self, u16 ms) {
        self.on_ms = ms
    }
}

impl Writer for Pulse {
    function write(&mut self, u8 byte) {
        self.on_ms = byte as u16
    }
}

enum Phase {
    Idle = 0,
    Run = 1,
}

internal function helper() {
    u8 a = 1
    u8 b = 2
    u8 wrapped = (a +% b) as u8
    u8 sat = (a +| b) as u8
    u12 sample = 4095u12
    adc12 x = sample
    u8 unused = (wrapped + sat + (x as u8)) as u8
}

#[blocking]
function wait() {
}

async function blink_once() {
    LED = 1
    await wait()
    LED = 0
}

function take_uart(UART0 uart) {
}

int main() {
    Pulse p
    p.start(10)
    p.write(3)

    match (Phase.Run) {
        Phase.Idle => { return 1 }
        Phase.Run => { }
        _ => { return 2 }
    }

    when (target.has_fpu) {
        f32 g = 1.0
        g = g + 0.0
    } else {
        int g = 1
        g = g + 0
    }

    u8 bit = LED
    LED = 1
    PORTB.pb5 = 1

    asm ("nop", in dummy = 0)

    helper()
    take_uart(UART0)

    u8* raw = &bit
    return bit as int
}
