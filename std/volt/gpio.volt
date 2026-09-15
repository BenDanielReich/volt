module volt.gpio;

enum PinMode {
    Input = 0,
    Output = 1,
    InputPullup = 2,
}

enum Level {
    Low = 0,
    High = 1,
}

extern function pin_mode(u8 pin, u8 mode);
extern function digital_write(u8 pin, u8 level);
extern u8 digital_read(u8 pin);
