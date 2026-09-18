/* RP2040 SIO GPIO. LED_BUILTIN is GP25 on Pico. */

#include <stdint.h>

#define SIO_BASE 0xd0000000u
#define RESETS_BASE 0x4000c000u
#define IO_BANK0_BASE 0x40014000u
#define PADS_BANK0_BASE 0x4001c000u

static void rp2040_enable_gpio(uint8_t pin) {
    volatile uint32_t *reset = (volatile uint32_t *)(RESETS_BASE + 0x00);
    volatile uint32_t *reset_clr = (volatile uint32_t *)(RESETS_BASE + 0x00 + 0x3000);
    volatile uint32_t *reset_done = (volatile uint32_t *)(RESETS_BASE + 0x08);
    *reset_clr = (1u << 5) | (1u << 8);
    while ((*reset_done & ((1u << 5) | (1u << 8))) != ((1u << 5) | (1u << 8))) {
    }
    (void)reset;
    volatile uint32_t *ctrl = (volatile uint32_t *)(IO_BANK0_BASE + 0x04u + (uint32_t)pin * 8u);
    *ctrl = 5u;
    volatile uint32_t *pad = (volatile uint32_t *)(PADS_BANK0_BASE + 0x04u + (uint32_t)pin * 4u);
    *pad = (1u << 6);
}

void delay_ms(uint32_t ms) {
    for (uint32_t i = 0; i < ms; i++) {
        for (volatile uint32_t n = 0; n < 4000; n++) {
        }
    }
}

void delay_us(uint32_t us) {
    for (uint32_t i = 0; i < us; i++) {
        for (volatile uint32_t n = 0; n < 4; n++) {
        }
    }
}

void pin_mode(uint8_t pin, uint8_t mode) {
    rp2040_enable_gpio(pin);
    volatile uint32_t *oe_set = (volatile uint32_t *)(SIO_BASE + 0x024);
    volatile uint32_t *oe_clr = (volatile uint32_t *)(SIO_BASE + 0x028);
    if (mode == 1) {
        *oe_set = 1u << pin;
    } else {
        *oe_clr = 1u << pin;
    }
}

void digital_write(uint8_t pin, uint8_t level) {
    if (level) {
        *(volatile uint32_t *)(SIO_BASE + 0x014) = 1u << pin;
    } else {
        *(volatile uint32_t *)(SIO_BASE + 0x018) = 1u << pin;
    }
}

uint8_t digital_read(uint8_t pin) {
    return ((*(volatile uint32_t *)(SIO_BASE + 0x004)) & (1u << pin)) ? 1 : 0;
}

uint16_t analog_read(uint8_t pin) {
    (void)pin;
    return 0;
}
