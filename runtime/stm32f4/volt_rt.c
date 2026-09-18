/* STM32F4 GPIO. Pin packing: (port << 4) | pin. Nucleo-F401RE LED PA5 = 5. */

#include <stdint.h>

#define RCC_AHB1ENR (*(volatile uint32_t *)0x40023830u)

static volatile uint32_t *gpio_base(uint8_t port) {
    return (volatile uint32_t *)(0x40020000u + (uint32_t)port * 0x400u);
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
    RCC_AHB1ENR |= 1u << port;
    volatile uint32_t *moder = gpio_base(port);
    uint32_t v = *moder;
    v &= ~(3u << (bit * 2));
    if (mode == 1) {
        v |= 1u << (bit * 2);
    }
    *moder = v;
}

void digital_write(uint8_t pin, uint8_t level) {
    uint8_t port = (uint8_t)(pin >> 4);
    uint8_t bit = (uint8_t)(pin & 0x0Fu);
    volatile uint32_t *bsrr = gpio_base(port) + 6;
    *bsrr = 1u << (level ? bit : (bit + 16));
}

uint8_t digital_read(uint8_t pin) {
    uint8_t port = (uint8_t)(pin >> 4);
    uint8_t bit = (uint8_t)(pin & 0x0Fu);
    return (gpio_base(port)[4] & (1u << bit)) ? 1 : 0;
}

uint16_t analog_read(uint8_t pin) {
    (void)pin;
    return 0;
}
