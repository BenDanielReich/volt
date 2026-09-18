module volt.gpio

// PinMode: Input or Output — pass this to pinmode.
enum PinMode {
    Input = 0,
    Output = 1,
    InputPullup = 2,
}

enum Level {
    Low = 0,
    High = 1,
}

extern function pin_mode(u8 pin, u8 mode)
extern function digital_write(u8 pin, u8 level)
extern u8 digital_read(u8 pin)
extern u16 analog_read(u8 pin)

function pinmode(u8 p, u8 mode) {
    pin_mode(p, mode)
}
