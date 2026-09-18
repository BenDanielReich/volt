/* STM32F1 GPIO. Pin packing: (port << 4) | pin. Blue Pill LED PC13 = 45. */

#include <stdint.h>

#define RCC_APB2ENR (*(volatile uint32_t *)0x40021018u)

static volatile uint32_t *gpio_base(uint8_t port) {
    return (volatile uint32_t *)(0x40010800u + (uint32_t)port * 0x400u);
}

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
    uint8_t port = (uint8_t)(pin >> 4);
    uint8_t bit = (uint8_t)(pin & 0x0Fu);
    RCC_APB2ENR |= (1u << (2 + port));
    volatile uint32_t *gpio = gpio_base(port);
    uint32_t shift = (bit < 8) ? (uint32_t)bit * 4u : (uint32_t)(bit - 8) * 4u;
    volatile uint32_t *cr = (bit < 8) ? &gpio[0] : &gpio[1];
    uint32_t v = *cr;
    v &= ~(0xFu << shift);
    v |= ((mode == 1) ? 0x1u : 0x4u) << shift;
    *cr = v;
}

void digital_write(uint8_t pin, uint8_t level) {
    uint8_t port = (uint8_t)(pin >> 4);
    uint8_t bit = (uint8_t)(pin & 0x0Fu);
    volatile uint32_t *bsrr = gpio_base(port) + 4;
    *bsrr = 1u << (level ? bit : (bit + 16));
}

uint8_t digital_read(uint8_t pin) {
    uint8_t port = (uint8_t)(pin >> 4);
    uint8_t bit = (uint8_t)(pin & 0x0Fu);
    return (gpio_base(port)[2] & (1u << bit)) ? 1 : 0;
}

uint16_t analog_read(uint8_t pin) {
    (void)pin;
    return 0;
}
