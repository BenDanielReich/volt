module registers

/// ATmega328P PORTB / DDRB — Arduino Uno pin 13 is PB5.
reg u8 PORTB @ 0x25
reg u8 DDRB  @ 0x24

#define LED_BIT 5

function setup() {
    DDRB |= 1 << LED_BIT
}

function loop() {
    PORTB ^= 1 << LED_BIT
}
