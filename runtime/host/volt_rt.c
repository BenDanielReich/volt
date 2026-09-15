#include "volt_rt.h"

#include <stdio.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <unistd.h>
#endif

void delay_ms(uint32_t ms) {
#if defined(_WIN32)
    Sleep(ms);
#else
    usleep(ms * 1000u);
#endif
}

void delay_us(uint32_t us) {
#if defined(_WIN32)
    Sleep((us + 999u) / 1000u);
#else
    usleep(us);
#endif
}

void pin_mode(uint8_t pin, uint8_t mode) {
    printf("pin_mode(%u, %u)\n", (unsigned)pin, (unsigned)mode);
}

void digital_write(uint8_t pin, uint8_t level) {
    printf("digital_write(%u, %u)\n", (unsigned)pin, (unsigned)level);
}

uint8_t digital_read(uint8_t pin) {
    (void)pin;
    return 0;
}
