module isr;

reg u8 PORTB @ 0x25;
reg u8 DDRB  @ 0x24;

static u32 ticks = 0;

function setup() {
    DDRB |= 1 << 5;
}

function loop() {
    if ((ticks % 1000) == 0) {
        PORTB ^= 1 << 5;
    }
}

#[interrupt("TIMER0_OVF")]
function on_tick() {
    ticks += 1;
}
