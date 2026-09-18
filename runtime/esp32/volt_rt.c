/* ESP32 GPIO. DevKit LED is GPIO2. Arduino Nano ESP32 LED is D13 → GPIO48. */

#include <stdint.h>

#if defined(VOLT_BOARD_NANO_ESP32)
#define GPIO_OUT_W1TS (*(volatile uint32_t *)0x60004008u)
#define GPIO_OUT_W1TC (*(volatile uint32_t *)0x6000400Cu)
#define GPIO_OUT1_W1TS (*(volatile uint32_t *)0x60004014u)
#define GPIO_OUT1_W1TC (*(volatile uint32_t *)0x60004018u)
#define GPIO_ENABLE_W1TS (*(volatile uint32_t *)0x60004024u)
#define GPIO_ENABLE_W1TC (*(volatile uint32_t *)0x60004028u)
#define GPIO_ENABLE1_W1TS (*(volatile uint32_t *)0x60004030u)
#define GPIO_ENABLE1_W1TC (*(volatile uint32_t *)0x60004034u)
#define GPIO_IN (*(volatile uint32_t *)0x6000403Cu)
#define GPIO_IN1 (*(volatile uint32_t *)0x60004040u)
/* Arduino D0–D13 → ESP32-S3 GPIO (By Arduino pin numbering). */
static const uint8_t volt_dmap[] = {
    44, 43, 5, 6, 7, 8, 9, 10, 17, 18, 21, 38, 47, 48
};
static uint8_t volt_gpio(uint8_t pin) {
    if (pin < (uint8_t)sizeof(volt_dmap)) {
        return volt_dmap[pin];
    }
    return pin;
}
#else
#define GPIO_OUT_W1TS (*(volatile uint32_t *)0x3FF44008u)
#define GPIO_OUT_W1TC (*(volatile uint32_t *)0x3FF4400Cu)
#define GPIO_OUT1_W1TS (*(volatile uint32_t *)0x3FF44014u)
#define GPIO_OUT1_W1TC (*(volatile uint32_t *)0x3FF44018u)
#define GPIO_ENABLE_W1TS (*(volatile uint32_t *)0x3FF44024u)
#define GPIO_ENABLE_W1TC (*(volatile uint32_t *)0x3FF44028u)
#define GPIO_ENABLE1_W1TS (*(volatile uint32_t *)0x3FF44030u)
#define GPIO_ENABLE1_W1TC (*(volatile uint32_t *)0x3FF44034u)
#define GPIO_IN (*(volatile uint32_t *)0x3FF4403Cu)
#define GPIO_IN1 (*(volatile uint32_t *)0x3FF44040u)
static uint8_t volt_gpio(uint8_t pin) {
    return pin;
}
#endif

void delay_ms(uint32_t ms) {
    for (uint32_t i = 0; i < ms; i++) {
        for (volatile uint32_t n = 0; n < 20000; n++) {
        }
    }
}

void delay_us(uint32_t us) {
    for (uint32_t i = 0; i < us; i++) {
        for (volatile uint32_t n = 0; n < 20; n++) {
        }
    }
}

static void volt_enable(uint8_t gpio, int output) {
    if (gpio < 32) {
        uint32_t mask = 1u << gpio;
        if (output) {
            GPIO_ENABLE_W1TS = mask;
        } else {
            GPIO_ENABLE_W1TC = mask;
        }
    } else {
        uint32_t mask = 1u << (gpio - 32);
        if (output) {
            GPIO_ENABLE1_W1TS = mask;
        } else {
            GPIO_ENABLE1_W1TC = mask;
        }
    }
}

void pin_mode(uint8_t pin, uint8_t mode) {
    volt_enable(volt_gpio(pin), mode == 1);
}

void digital_write(uint8_t pin, uint8_t level) {
    uint8_t gpio = volt_gpio(pin);
    if (gpio < 32) {
        uint32_t mask = 1u << gpio;
        if (level) {
            GPIO_OUT_W1TS = mask;
        } else {
            GPIO_OUT_W1TC = mask;
        }
    } else {
        uint32_t mask = 1u << (gpio - 32);
        if (level) {
            GPIO_OUT1_W1TS = mask;
        } else {
            GPIO_OUT1_W1TC = mask;
        }
    }
}

uint8_t digital_read(uint8_t pin) {
    uint8_t gpio = volt_gpio(pin);
    if (gpio < 32) {
        return (GPIO_IN & (1u << gpio)) ? 1 : 0;
    }
    return (GPIO_IN1 & (1u << (gpio - 32))) ? 1 : 0;
}

uint16_t analog_read(uint8_t pin) {
    (void)pin;
    return 0;
}
