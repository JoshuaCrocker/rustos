// TODO information about lib.rs

// As the library is a separate compilation unit, we need to specify the no_std
// attribute again.
#![no_std]

#![cfg_attr(test, no_main)]

// Enable the Custom Test Frameworks feature to allow for unit testing of the
// OS code. This has to be done becuase the default test libary relies on the
// standard library to function correctly. By implementing our own test running
// we are still able to unit test our code, though we will not have the more
// advanced features of Rust's default test framework, such as should_panic
// tests.
#![feature(custom_test_frameworks)]

//
#![feature(abi_x86_interrupt)]

// Point to the custom test runner method.
#![test_runner(crate::test_runner)]

// Change the name of the main function generated by the cargo test command, so
// that we are able to refer to it in our _start method. We need to do this
// becuase we are operating in a no_main environment, so by default the main
// test method will not be executed.
#![reexport_test_harness_main = "test_main"]

// While the majority of the built-in functions, which Rust assumes are 
// available on all systems, are provided by the 'compiler_builtins' crate, 
// there are some which are not enabled by default as they are normally provided
// by the C library on the system (memset, memcpy and memcmp). At present there
// is no way to enable the 'compiler_builtins' impementations of these methods,
// so the workout we have is to include rlibc as a dependency.
// ---
// Since we aren't directly using the functions from rlibc we need to instruct
// the Rust compiler to link the crate.
extern crate rlibc;

use core::panic::PanicInfo;

pub mod serial;
pub mod vga_buffer;
pub mod interrupts;
pub mod gdt;

// Create a new trait 'Testable' which enables us to automatically print out the
// names of the test methods prior to execution, as well as the '[ok]' status
// message afterwards.
pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where T: Fn(), {
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());

        // We are able to get the method to run itself becuase we have told the
        // compiler that we require the type to have the 'Fn' trait.
        self();
        
        serial_println!("[ok]");
    }
}

// QEMU Exit Code Enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

// Indicate to the QEMU emulator what we want to exit. We do this by opening the
// 0xf4 port, and writing the given exit code to the attached device.
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

// Implement the custom test runner method. the cfg(test) attribute tells the
// compiler that it should only be included within the test environment.
pub fn test_runner(tests: &[&dyn Testable]) {
    // Print the number of tests run
    serial_println!("Running {} tests", tests.len());
    
    // Iterate through the list of tests...
    for test in tests {
        // ... and run each one.
        test.run();
    }

    // Exit QEMU
    exit_qemu(QemuExitCode::Success);
}

// Test-mode panic handler, which prints output to the serial interface, and
// then exits QEMU with the fail status code.
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);

    exit_qemu(QemuExitCode::Failed);
    loop {}
}

// General init method to initialise any modules which we have imported. In this
// instance the only thing we're setting up is the Interrupt Descriptor Table.
pub fn init() {
    gdt::init();
    interrupts::init_idt();
}

// 'cargo test' entrypoint
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init();
    test_main();
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
