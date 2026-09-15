#ifndef VOLT_RT_H
#define VOLT_RT_H

#include <stdint.h>
#include <stdbool.h>

void delay_ms(uint32_t ms);
void delay_us(uint32_t us);
void pin_mode(uint8_t pin, uint8_t mode);
void digital_write(uint8_t pin, uint8_t level);
uint8_t digital_read(uint8_t pin);

#endif
