module branch

#include <volt/gpio>
#include <volt/delay>

const u8 LED = 13

function setup() {
    pin_mode(LED, PinMode.Output)
}

function loop() {
    const u8 on = Level.High
    u8 state = digital_read(LED)
    if (state == on) {
        digital_write(LED, Level.Low)
    } else if (state == Level.Low) {
        digital_write(LED, on)
    } else {
        digital_write(LED, Level.Low)
    }
    delay_ms(200)
}
