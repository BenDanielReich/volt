/* ESP8266 / ESP8285 GPIO. ESP8000, NodeMCU, and ESP-12x LED is GPIO2. */

#include <stdint.h>

#define GPIO_OUT (*(volatile uint32_t *)0x60000300u)
#define GPIO_OUT_W1TS (*(volatile uint32_t *)0x60000304u)
#define GPIO_OUT_W1TC (*(volatile uint32_t *)0x60000308u)
#define GPIO_ENABLE_W1TS (*(volatile uint32_t *)0x60000310u)
#define GPIO_ENABLE_W1TC (*(volatile uint32_t *)0x60000314u)
#define GPIO_IN (*(volatile uint32_t *)0x60000318u)

void delay_ms(uint32_t ms) {
    for (uint32_t i = 0; i < ms; i++) {
        for (volatile uint32_t n = 0; n < 8000; n++) {
        }
    }
}

void delay_us(uint32_t us) {
    for (uint32_t i = 0; i < us; i++) {
        for (volatile uint32_t n = 0; n < 8; n++) {
        }
    }
}

void pin_mode(uint8_t pin, uint8_t mode) {
    if (pin > 16) {
        return;
    }
    uint32_t mask = 1u << pin;
    if (mode == 1) {
        GPIO_ENABLE_W1TS = mask;
    } else {
        GPIO_ENABLE_W1TC = mask;
    }
}

void digital_write(uint8_t pin, uint8_t level) {
    if (pin > 16) {
        return;
    }
    uint32_t mask = 1u << pin;
    if (level) {
        GPIO_OUT_W1TS = mask;
    } else {
        GPIO_OUT_W1TC = mask;
    }
}

uint8_t digital_read(uint8_t pin) {
    if (pin > 16) {
        return 0;
    }
    return (GPIO_IN & (1u << pin)) ? 1 : 0;
}

uint16_t analog_read(uint8_t pin) {
    (void)pin;
    return 0;
}
