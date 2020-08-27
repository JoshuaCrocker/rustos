// Define the Buffer width and height constants. The VGA buffer consists of 25
// rows of 80 ASCII characters.
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// We need to imeplement the core::fmt::Write trait in order to support the
// write! and writeln! macros.
use core::fmt;

// Import the volatile library to allow us to inform the Rust compiler of 
// references which may have side-effects outside of its vision.
use volatile::Volatile;

// We need to use the Lazy Static crate in order to stop our static constants
// from being computed at compile time. Instead the Lazy Static crate lazily
// evaluates the constant for the first time when the constant is first
// accessed, which allows us to continue to initialise stuff like the ColourCode
// within the constant definition.
use lazy_static::lazy_static;

// Use a spinlock to ensure a lock can be held on the Writer constant.
use spin::Mutex;

// Use a C-like enum to specify the number for each colour, which is stored as a
// u8, thanks to the repr(u8) attribute.
// ---
// The allow(dead_code) attribute prevents the Rust compiler from complaining
// when values within the enum aren't used.
// ---
// By deriving the Copy, Clone, Debug, PartialEq and Eq traits, we enable copy
// semantics for the type and make it printable and comparable.
// TODO what are 'copy semantics'?
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Colour {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGrey = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15
}

// The ColourCode struct contains the full colour data byte, in u8 format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColourCode(u8);

impl ColourCode {
    fn new(foreground: Colour, background: Colour) -> ColourCode {
        ColourCode((background as u8) << 4 | (foreground as u8))
    }
}

// repr(C) guarantees the struct's fields are laid out exactly how they would be
// in C, therefore guaranteeing the correct field ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    colour_code: ColourCode,
}

// repr(transparent) allows us to guarantee the struct has the same memory
// layout as the single chars field contained within the struct.
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// The writer always writes to the last line and shifts lines up when a line is
// full, or the \n control character is received. 
pub struct Writer {
    // Stores the current position in the last row.
    column_position: usize,
    
    // Stores the current foeground and background colours.
    colour_code: ColourCode,

    // Reference to the buffer.
    // We make use of the 'static lifetime to specify that the reference to the
    // Buffer should be valid for the entire runtime of the program.
    buffer: &'static mut Buffer,
}

// Writer implementation
impl Writer {
    // Write a byte to the VGA Buffer
    pub fn write_byte(&mut self, byte: u8) {
        // Check the byte we've been given...
        match byte {
            // If the byte is a new line control code, we want to move to the
            // next line of the VGA Buffer.
            b'\n' => self.new_line(),

            // otherwise...
            byte => {
                // If we're at the end of the current row, we want tp move to
                // the next line of the VGA Buffer.
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                // Determine the current position in the VGA buffer.
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                
                // Set the character and colour code.
                // ---
                // As we are using the Volatile wrapper we don't have access to
                // the standard assignment operator. As such we have to use the
                // write method exposed by the Volatile library to write to
                // the given memory space.
                // ---
                // Using the Volatile library ensures the Rust compiler will
                // never optimise away this write, which is might do as it does
                // not have any side effects which are visible to the compiler.
                let colour_code = self.colour_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    colour_code,
                });

                // Move to the next position in the current row.
                self.column_position += 1;
            }
        }
    }

    // To print whole strings we will break them down into their constituent
    // bytes and then iterate through them, printing the valid bytes to the
    // screen.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII byte or a new line
                0x20..=0x7e | b'\n' => self.write_byte(byte),

                // Values not part of the printable ASCII range so we will
                // print a ■ characrer instead
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        // Move each character up one row.
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }

        // reset the row and column
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            colour_code: self.colour_code,
        };

        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// Load the contents of the constant when it is called for the first time.
// ---
// We can be sure this interface to the outside world is safe becuase the only
// unsafe block is called within this declaration, and the rest of the access to
// that memory space is handled by the underlying array data type. This data
// type is protected by out of bounds checks, which means it is now impossible
// to assign values to any parts of the system outside of the buffer.
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        colour_code: ColourCode::new(Colour::Cyan, Colour::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

// Implement the standard print! macro, passing it through to our VGA Buffer
// implementation.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

// Implement the standard println! macro, passing it through to our VGA Buffer
// implementation.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

// Hidden, helper method to pass input from the print! and println! macros
// through to our VGA Buffer.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}


// TESTING

// Simple test to ensure that the println (and consequently the print) macro
// have been set up and are functioning correctly. If we get through this
// without panicking, then the test passes.
#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

// Test printing many lines to ensure that no panic occurs when printing over
// the maximum number of rows available within the buffer. If we get through 
// this without panicking, then the test passes.
#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

// Test that the text output to the buffer is the same which is input into the
// buffer.
#[test_case]
fn test_println_output() {
    // Define and print a test string
    let s = "Test string";
    println!("{}", s);

    // Iterate over the test string
    for (i, c) in s.chars().enumerate() {
        // and retrieve the relevant character within the VGA Buffer
        // N.b. as we called the println macro, the text will be on the second
        // line, not the bottom, hence BUFFER_HEIGHT - 2.
        let screen_char = 
            WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();

        // Ensure the characters are the same.
        assert_eq!(char::from(screen_char.ascii_character), c);
    }
}

// TODO test printing long lines (shouldn't panic)
// TODO test line wrapping
// TODO test non-printable character handling
// TODO test non-unicode character handling
