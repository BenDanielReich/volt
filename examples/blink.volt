module blink;

#include <volt/gpio>
#include <volt/delay>

#define LED 13
#define DELAY 500

function setup() {
    pin_mode(LED, PinMode.Output);
}

function loop() {
    digital_write(LED, Level.High);
    delay_ms(DELAY);
    digital_write(LED, Level.Low);
    delay_ms(DELAY);
}
