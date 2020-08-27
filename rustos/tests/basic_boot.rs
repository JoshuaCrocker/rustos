// It is the convention for integration tests that they are placed within the
// 'tests' folder within the project root. Tests in this location are picked up
// by both the standard test framework and custom test frameworks.
// ---
// All integration tests are completely standalone to the main.rs program, which
// means each test needs to define its own entry point function. Since this is
// not ideal, we can create a special lib.rs file, within which we can set up
// all of the stuff common to our main program and our intergration tests.
// ---
// We don't want the standard library.
#![no_std]
#![no_main]

// Like in main, we need to import the main test components from the library and
// bring them into the integration test environment.
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use rustos::println;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // The _start method is also a diverging function which is not allowed to
    // return. This is becuase this method is invoked directly by the host OS
    // or bootloader. Instead of returning this method would, within the context
    // of producing an OS, invoke the exit system call, or shut down the
    // machine.
    // --- 

    // At this stage in development we will use the VGA text buffer to print
    // text to the screen. This typically consists of an area of 25 lines, each
    // 80 character cells long.
    // ---

    // We will produce a driver for the VGA buffer soon, but for now we just
    // need to know that the buffer is located at the address 0xb8000, and each
    // character cells consists of an ASCII byte and a colour byte.

    println!("Hello World{}", "!");

    // Within the test environment, we want to call the main test method.
    #[cfg(test)]
    test_main();

    loop {}
}

#[test_case]
fn test_println() {
    println!("test_println output");
}
