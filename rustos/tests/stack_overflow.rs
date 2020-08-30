#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptStackFrame, InterruptDescriptorTable};
use rustos::{exit_qemu, QemuExitCode, serial_print, serial_println};

lazy_static! {
    // Set up the test IDT, to call a custom double-fault handler function, with
    // access to the stack we've set aside for the double fault handler.
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(rustos::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");

    rustos::gdt::init();

    // Don't call our standard init_idt method becuase we want to register a
    // custom double fault handler which exits the emulator with a success code
    // if the double fault handler gets called and doesn't fail.
    init_test_idt();

    // trigger a stack overflow
    stack_overflow();

    panic!("Execution continued after stack overflow");
}

// Allow unconditional recursion to prevent compiler warnings about the function
// recursing endlessly.
#[allow(unconditional_recursion)]
fn stack_overflow() {
    // Recurse
    stack_overflow();

    // Prevent the compiler optimising this recursive operation into a loop,
    // which would prevent a stack overflow from occuring.
    volatile::Volatile::new(0).read();
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

// Set up a double fault handler to exit QEMU with a success exit code, to mark
// the test as passed.
extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}
