// Disable the implicit linking of the standard library.
// We have to do this becuase Rust, by default, links to the Rust standard
// library. As our plant is to produce an operating system, we do not want to do
// we need a standalone executable. The standard library closely interacts with
// OS services, which means we cannot use it.
#![no_std]

// Without access to the standard Rust runtime, we don't have Rust setting up
// the execution environemnt for us. This means we need to tell the compiler
// that we don't want to use the normal entry point, and then need to define our
// own start method for the freestanding executable.
#![no_main]

use core::panic::PanicInfo;

// As we are operating in a no_std environment we need to define our own
// panic_handler method. This is usually implemented by the standard library.
// ---
// This function should never return a value. As such this is defined as a
// diverging function by returning the never type, as shown by the exclaimation
// mark in the return type field.

// TODO what is a diverging function?
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // The PanicInfo parameter contains information relating to the position
    // within the code where the panic occurred, as well as the optional panic
    // message.

    // For now there's not much we can do in this function, so we'll loop
    // indefinitely.
    loop {}
}

// We no longer need the main method, as it was the underlying Rust runtime
// which called it. Instead we define the _start method, which overwrites the
// standard entry point.
// ---
// The no_mangle attribute disables name mangiling to ensure the Rust compiler
// keeps the _start name on this method. This is necessary as we need to tell
// the linker which method is the entry point to the executable.
// ---
// The method is marked at 'extern "C"' to indicate to the compiler that we want
// to use the C calling convention for this function, instead of the standard
// Rust calling convention.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // The _start method is also a diverging function which is not allowed to
    // return. This is becuase this method is invoked directly by the host OS
    // or bootloader. Instead of returning this method would, within the context
    // of producing an OS, invoke the exit system call, or shut down the
    // machine.
    loop {}
}

