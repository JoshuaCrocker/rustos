use spin::Mutex;
use lazy_static::lazy_static;

// We are going to use the 16550 UART Serial Port in order to communicate with
// the outsite world. We are doing this to enable communication to the console
// which is running our unit tests. The uart_16550 crate contains a SerialPort
// struct which reprects the UART registers.
use uart_16550::SerialPort;

// Similar to the VGA Buffer, lazy_static and a spinlock have been used to help
// create a static reference to the Serial Port. This has been done to ensure
// the init method is only called once, on the first use of the Serial Port.
// ---
// Similar to the isa-debug-exit device, the UART is programmed using Port I/O.
// As this is more complex than the isa-debug-exit device, it uses multiple I/O
// ports for programming different device registers. The SerialPort::new method
// required the address of the first I/O port of the UART, from which it then
// calculates the addresses of the other ports.
// ---
// 0x3F8 has been used as the first I/O port address. This is the standard port
// number for the first serial interface.
lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

// Helper method
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
