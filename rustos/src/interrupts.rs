// TODO insert theory?

use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;
use lazy_static::lazy_static;
use crate::println;

// Initialise the Interrupt Descriptor Table. This has to be a static property
// as it needs to exist for the entire runtime of the application. By default
// static properties are immutable, so we coule define this as a mutable
// static in order to allow us to set the interrupt handlers. Unfortunately
// mutable statics are prone to data rases, so would require an unsafe block.
// ---
// We can get around these problems by making use of the lazy_static library
// to set up the Interrupt Descriptor Table the first time it is called, and
// requires no unsafe blocks within our code, as all unsafe functionality is
// abstracted away into the lazy_static library.
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        
        idt
    };
}

pub fn init_idt() {
    

    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut InterruptStackFrame) {
        println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}


// Testing

// Test the Breakpoint Exception Handler. We know this test passes if it
// sees execution the whole way through to the end.
#[test_case]
fn test_breakpoint_exception() {
    // Invoke a Breakpoint Exception
    x86_64::instructions::interrupts::int3();
}
