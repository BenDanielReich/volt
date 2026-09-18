/* Embedded by voltc. Arduino pin numbers for Uno/Nano/Leonardo/Mega. */

#include <stdint.h>
#if defined(__AVR__)
#include <avr/io.h>
#include <util/delay.h>
#ifndef F_CPU
#define F_CPU 16000000UL
#endif
#endif

#ifndef ARDUINO

void delay_ms(uint32_t ms) {
#if defined(__AVR__)
    while (ms--) {
        _delay_ms(1);
    }
#else
    (void)ms;
#endif
}

void delay_us(uint32_t us) {
#if defined(__AVR__)
    while (us--) {
        _delay_us(1);
    }
#else
    (void)us;
#endif
}

#if defined(__AVR__)

#define VP_A 0
#define VP_B 1
#define VP_C 2
#define VP_D 3
#define VP_E 4
#define VP_F 5
#define VP_G 6
#define VP_H 7
#define VP_J 8
#define VP_K 9
#define VP_L 10

#define VP(port, bit) ((uint8_t)(((port) << 4) | ((bit) & 0x0Fu)))

#if defined(VOLT_BOARD_MEGA)
static const uint8_t volt_map[] = {
    VP(VP_E, 0), VP(VP_E, 1), VP(VP_E, 4), VP(VP_E, 5), VP(VP_G, 5), VP(VP_E, 3),
    VP(VP_H, 3), VP(VP_H, 4), VP(VP_H, 5), VP(VP_H, 6), VP(VP_B, 4), VP(VP_B, 5),
    VP(VP_B, 6), VP(VP_B, 7), VP(VP_J, 1), VP(VP_J, 0), VP(VP_H, 1), VP(VP_H, 0),
    VP(VP_D, 3), VP(VP_D, 2), VP(VP_D, 1), VP(VP_D, 0), VP(VP_A, 0), VP(VP_A, 1),
    VP(VP_A, 2), VP(VP_A, 3), VP(VP_A, 4), VP(VP_A, 5), VP(VP_A, 6), VP(VP_A, 7),
    VP(VP_C, 7), VP(VP_C, 6), VP(VP_C, 5), VP(VP_C, 4), VP(VP_C, 3), VP(VP_C, 2),
    VP(VP_C, 1), VP(VP_C, 0), VP(VP_D, 7), VP(VP_G, 2), VP(VP_G, 1), VP(VP_G, 0),
    VP(VP_L, 7), VP(VP_L, 6), VP(VP_L, 5), VP(VP_L, 4), VP(VP_L, 3), VP(VP_L, 2),
    VP(VP_L, 1), VP(VP_L, 0), VP(VP_B, 3), VP(VP_B, 2), VP(VP_B, 1), VP(VP_B, 0),
    VP(VP_F, 0), VP(VP_F, 1), VP(VP_F, 2), VP(VP_F, 3), VP(VP_F, 4), VP(VP_F, 5),
    VP(VP_F, 6), VP(VP_F, 7), VP(VP_K, 0), VP(VP_K, 1), VP(VP_K, 2), VP(VP_K, 3),
    VP(VP_K, 4), VP(VP_K, 5), VP(VP_K, 6), VP(VP_K, 7),
};
#elif defined(VOLT_BOARD_GEMMA)
static const uint8_t volt_map[] = {
    VP(VP_B, 0), VP(VP_B, 1), VP(VP_B, 2), VP(VP_B, 3), VP(VP_B, 4), VP(VP_B, 5),
};
#elif defined(VOLT_BOARD_LEONARDO)
static const uint8_t volt_map[] = {
    VP(VP_D, 2), VP(VP_D, 3), VP(VP_D, 1), VP(VP_D, 0), VP(VP_D, 4), VP(VP_C, 6),
    VP(VP_D, 7), VP(VP_E, 6), VP(VP_B, 4), VP(VP_B, 5), VP(VP_B, 6), VP(VP_B, 7),
    VP(VP_D, 6), VP(VP_C, 7), VP(VP_B, 3), VP(VP_B, 1), VP(VP_B, 2), VP(VP_B, 0),
    VP(VP_F, 7), VP(VP_F, 6), VP(VP_F, 5), VP(VP_F, 4), VP(VP_F, 1), VP(VP_F, 0),
    VP(VP_D, 4), VP(VP_D, 7), VP(VP_B, 4), VP(VP_B, 5), VP(VP_B, 6), VP(VP_D, 6),
};
#else
/* Uno, Nano, Pro Mini — ATmega328P Arduino pin numbers 0–19. */
static const uint8_t volt_map[] = {
    VP(VP_D, 0), VP(VP_D, 1), VP(VP_D, 2), VP(VP_D, 3), VP(VP_D, 4), VP(VP_D, 5),
    VP(VP_D, 6), VP(VP_D, 7), VP(VP_B, 0), VP(VP_B, 1), VP(VP_B, 2), VP(VP_B, 3),
    VP(VP_B, 4), VP(VP_B, 5), VP(VP_C, 0), VP(VP_C, 1), VP(VP_C, 2), VP(VP_C, 3),
    VP(VP_C, 4), VP(VP_C, 5),
};
#endif

static volatile uint8_t *volt_ddr(uint8_t port) {
    switch (port) {
#ifdef DDRA
    case VP_A: return &DDRA;
#endif
    case VP_B: return &DDRB;
    case VP_C: return &DDRC;
    case VP_D: return &DDRD;
#ifdef DDRE
    case VP_E: return &DDRE;
#endif
#ifdef DDRF
    case VP_F: return &DDRF;
#endif
#ifdef DDRG
    case VP_G: return &DDRG;
#endif
#ifdef DDRH
    case VP_H: return &DDRH;
#endif
#ifdef DDRJ
    case VP_J: return &DDRJ;
#endif
#ifdef DDRK
    case VP_K: return &DDRK;
#endif
#ifdef DDRL
    case VP_L: return &DDRL;
#endif
    default: return &DDRB;
    }
}

static volatile uint8_t *volt_port(uint8_t port) {
    switch (port) {
#ifdef PORTA
    case VP_A: return &PORTA;
#endif
    case VP_B: return &PORTB;
    case VP_C: return &PORTC;
    case VP_D: return &PORTD;
#ifdef PORTE
    case VP_E: return &PORTE;
#endif
#ifdef PORTF
    case VP_F: return &PORTF;
#endif
#ifdef PORTG
    case VP_G: return &PORTG;
#endif
#ifdef PORTH
    case VP_H: return &PORTH;
#endif
#ifdef PORTJ
    case VP_J: return &PORTJ;
#endif
#ifdef PORTK
    case VP_K: return &PORTK;
#endif
#ifdef PORTL
    case VP_L: return &PORTL;
#endif
    default: return &PORTB;
    }
}

static volatile uint8_t *volt_pin(uint8_t port) {
    switch (port) {
#ifdef PINA
    case VP_A: return &PINA;
#endif
    case VP_B: return &PINB;
    case VP_C: return &PINC;
    case VP_D: return &PIND;
#ifdef PINE
    case VP_E: return &PINE;
#endif
#ifdef PINF
    case VP_F: return &PINF;
#endif
#ifdef PING
    case VP_G: return &PING;
#endif
#ifdef PINH
    case VP_H: return &PINH;
#endif
#ifdef PINJ
    case VP_J: return &PINJ;
#endif
#ifdef PINK
    case VP_K: return &PINK;
#endif
#ifdef PINL
    case VP_L: return &PINL;
#endif
    default: return &PINB;
    }
}

static int volt_decode(uint8_t pin, volatile uint8_t **ddr, volatile uint8_t **port,
                       volatile uint8_t **in, uint8_t *bit) {
    if (pin >= (uint8_t)sizeof(volt_map)) {
        return 0;
    }
    uint8_t enc = volt_map[pin];
    uint8_t p = (uint8_t)(enc >> 4);
    *bit = (uint8_t)(enc & 0x0Fu);
    *ddr = volt_ddr(p);
    *port = volt_port(p);
    *in = volt_pin(p);
    return 1;
}

void pin_mode(uint8_t pin, uint8_t mode) {
    volatile uint8_t *ddr, *port, *in;
    uint8_t bit;
    if (!volt_decode(pin, &ddr, &port, &in, &bit)) {
        return;
    }
    uint8_t mask = (uint8_t)(1u << bit);
    if (mode == 1) {
        *ddr |= mask;
    } else {
        *ddr &= (uint8_t)~mask;
        if (mode == 2) {
            *port |= mask;
        } else {
            *port &= (uint8_t)~mask;
        }
    }
}

void digital_write(uint8_t pin, uint8_t level) {
    volatile uint8_t *ddr, *port, *in;
    uint8_t bit;
    if (!volt_decode(pin, &ddr, &port, &in, &bit)) {
        return;
    }
    uint8_t mask = (uint8_t)(1u << bit);
    if (level) {
        *port |= mask;
    } else {
        *port &= (uint8_t)~mask;
    }
}

uint8_t digital_read(uint8_t pin) {
    volatile uint8_t *ddr, *port, *in;
    uint8_t bit;
    if (!volt_decode(pin, &ddr, &port, &in, &bit)) {
        return 0;
    }
    return (*in & (uint8_t)(1u << bit)) ? 1 : 0;
}

uint16_t analog_read(uint8_t pin) {
#ifdef ADCSRA
    uint8_t chan = pin;
    if (chan >= 14) {
        chan = (uint8_t)(chan - 14);
    }
    ADMUX = (uint8_t)((1u << REFS0) | (chan & 0x07u));
    ADCSRA = (uint8_t)((1u << ADEN) | (1u << ADSC) | (1u << ADPS2) | (1u << ADPS1) | (1u << ADPS0));
    while (ADCSRA & (1u << ADSC)) {
    }
    return ADC;
#else
    (void)pin;
    return 0;
#endif
}

#else /* !__AVR__ */

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

#endif /* __AVR__ */

#else /* ARDUINO */

void pin_mode(uint8_t pin, uint8_t mode) { pinMode(pin, mode); }
void digital_write(uint8_t pin, uint8_t level) { digitalWrite(pin, level); }
uint8_t digital_read(uint8_t pin) { return digitalRead(pin) ? 1 : 0; }
uint16_t analog_read(uint8_t pin) { return (uint16_t)analogRead(pin); }

#endif
