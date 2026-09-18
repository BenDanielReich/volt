/* Generic Cortex-M stubs when the chip is not a named board. */

#include <stdint.h>

void delay_ms(uint32_t ms) {
    for (uint32_t i = 0; i < ms; i++) {
        for (volatile uint32_t n = 0; n < 1000; n++) {
        }
    }
}

void delay_us(uint32_t us) {
    for (uint32_t i = 0; i < us; i++) {
        for (volatile uint32_t n = 0; n < 1; n++) {
        }
    }
}

void pin_mode(uint8_t pin, uint8_t mode) {
    (void)pin;
    (void)mode;
}

void digital_write(uint8_t pin, uint8_t level) {
    (void)pin;
    (void)level;
}

uint8_t digital_read(uint8_t pin) {
    (void)pin;
    return 0;
}

uint16_t analog_read(uint8_t pin) {
    (void)pin;
    return 0;
}
