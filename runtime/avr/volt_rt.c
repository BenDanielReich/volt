#include <avr/io.h>
#include <util/delay.h>
#include <stdint.h>

#ifndef F_CPU
#define F_CPU 16000000UL
#endif

void delay_ms(uint32_t ms) {
    while (ms--) {
        _delay_ms(1);
    }
}

void delay_us(uint32_t us) {
    while (us--) {
        _delay_us(1);
    }
}

void pin_mode(uint8_t pin, uint8_t mode) {
    /* Arduino Uno-style digital pins 0–13 on PORTD/PORTB. */
    if (pin < 8) {
        if (mode == 1) {
            DDRD |= (uint8_t)(1u << pin);
        } else {
            DDRD &= (uint8_t)~(1u << pin);
            if (mode == 2) {
                PORTD |= (uint8_t)(1u << pin);
            }
        }
    } else {
        uint8_t bit = (uint8_t)(pin - 8);
        if (mode == 1) {
            DDRB |= (uint8_t)(1u << bit);
        } else {
            DDRB &= (uint8_t)~(1u << bit);
            if (mode == 2) {
                PORTB |= (uint8_t)(1u << bit);
            }
        }
    }
}

void digital_write(uint8_t pin, uint8_t level) {
    if (pin < 8) {
        if (level) {
            PORTD |= (uint8_t)(1u << pin);
        } else {
            PORTD &= (uint8_t)~(1u << pin);
        }
    } else {
        uint8_t bit = (uint8_t)(pin - 8);
        if (level) {
            PORTB |= (uint8_t)(1u << bit);
        } else {
            PORTB &= (uint8_t)~(1u << bit);
        }
    }
}

uint8_t digital_read(uint8_t pin) {
    if (pin < 8) {
        return (PIND & (uint8_t)(1u << pin)) ? 1 : 0;
    }
    uint8_t bit = (uint8_t)(pin - 8);
    return (PINB & (uint8_t)(1u << bit)) ? 1 : 0;
}
