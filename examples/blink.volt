module blink

#include <volt/board>

function setup() {
    pinmode(LED_BUILTIN, PinMode.Output)
}

function loop() {
    digital_write(LED_BUILTIN, Level.High)
    delay_ms(500)
    digital_write(LED_BUILTIN, Level.Low)
    delay_ms(500)
}
