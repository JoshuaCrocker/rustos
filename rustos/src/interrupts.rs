// CPU Exceptions may occur at various points during the execution of a program,
// such as when accessing an invalid memory address or attempting to divide by
// zero. An Interrupt Descriptor Table (IDT) is used to define where the handler
// functions are located within the program, in order to allow the processor to
// call the correct functions when an exception occurs.
//
// Whenever an exception occurs, this is an indication that something was wrong
// with the current instruction. As an example, if you attempt to divide by zero
// the CPU will issue an exception. When an interrupt occurs, the CPU interrupts
// the current instruction, and called a specific exception handler function
// immediately.
//
// On the x86 architecture, there are about 20 different types of CPI exception,
// the with following being the most important:
// - Page Fault: Occurs on illegal memory accesses. (e.g. if the current
//               instruction tries to read from an unmapped page, or to write to
//               a read-only page)
// - Invalid Opcode: Occurs when the current instruction is invalid. 
//               (e.g. making a call to newer instructions on an older CPU that
//               doesn't support them)
// - General Protection Fault: Has a multitude of causes, including trying to
//               execute a priviledged instruction in user-level code or writing
//               to reserved field in configuration registers.
// - Double Fault: Occurs when an exception occurs while calling or executing
//               another exception handler. This also occurs when there is no
//               exception handler defined for a given exception.
// - Triple Fault: Occurs when another exception occurs during the Double Fault
//               handler. We aren't able to handle a Triple Fault as it is
//               handled directly by the processor, typically by resetting
//               itself and rebooting the operating system.
// A full list of the exceptions which can occur is written up on the OSDev Wiki
// (https://wiki.osdev.org/Exceptions)
//
// When an exception occurs, the CPU does the following (approximately):
// 1. Push some registers on the stack, including the instruction pointer and
//    the RFLAGS register.
// 2. Read the corresponding entry from the IDT.
// 3. Check if the entry is present, and raise a Double Fault if it doesn't.
// 4. Disable hardware interrupts if the entry is an interrupt gate (i.e. bit 40
//    is not set).
// 5. Load the specified GDT selector into the Code Segment segment.
// 6. Jump to the specified handler function.
// ---
// Interrupt Calling Convention
// Exception calls are similar to function calls in that, when called, the CPU
// jumps to and executes the first instruction of the exception handler, and
// afterwards jumps to the return address and continues program execution. The
// major difference between regular function calls and exception calls are that
// functions are invoked voluntary, while exceptions can occur at any point
// during the program execution.
//
// On x86_64 linux, the following rules apply for C functions (in Rust this only
// applies to functions marked as 'extern "C" fn'):
// - First six integer arguments are passed in the following registers:
//          rdi, rsi, rdx, rcd, r8, r9
// - Additional arguments are passed on the stack.
// - Results are returned in the 'rax' and 'rdx' registers.
//
// Within the calling conventions, registers are split into two groups:
// preserved and scratch registers.
//
// Preserved registers must remain unchanged between function calls, with the
// current function (the callee) being responsible for restoring the original
// values before returning from the function. Typically these are saved to the
// stack at the function's beginning, and are then removed and restored just
// before returning.
//
// Contrastingly, scratch registers can be overwritten without any restrictions.
// If the caller requires the values be preserved, they are required to save the
// values before a function call, and restore them afterwards. Again, this is
// typically done by saving the register values to the stack.
//
// +--------------------------------------|------------------------------------+
// |         Preserved Registers          |         Scratch Registers          |
// |   rbp, rbx, rsp, r12, r13, r14, r15  |   rax, rcx, rdx, rsi, rdi, r8, r9  |
// |                                      |              r10, r11              |
// |            callee-saved              |            caller-saved            |
// +--------------------------------------|------------------------------------+
//
// The compiler is aware of these rules, so during normal execution of the code
// they're enforced. However, we can't know during compile time that an
// exception will occur, which means we are unable to backup any registers
// prior to calling the exception handler. The x-86-interrupt calling convention
// is a calling convention which preserves all registers by storing them prior
// to executing the function, and restoring them afterwards. This is the calling
// convention we will have to use for our exception handlers.
//
// While normally calling another function would simply push the return address
// to the stack, we can't do this when calling exceptions. This is becuase
// interrupt handlers typically run in a different context, so the CPU must do
// the following when an exception is called:
// 1. Align the Stack Pointer: As an interrupt can occur at any instriction, the
//              stack pointer can have any value too. Some CPU instructions
//              require that the stack pointer is alighed on a 16 byte boundary.
// 2. Switch Stacks: This often occurs when the CPU privilege level changes 
//              (e.g. when a CPU exception occurs in a user mode program, the
//              CPU will change typically elevate to a higher prililege level).
//              Stack switching can also be configured to occur on the execution
//              of specific interrupts.
// 3. Pushing the old stack pointer: The CPU pushes the value of the stack 
//              pointer (rsp) and the stack segment (ss) registers at the time
//              of the interrupt. This enables the restoration of the orginal
//              stack pointer when returning from an interrupt handler.
// 4. Pushing and updating the RFLAGS register: The RFLAGS register is home to
//              many control and status bits. The old value is pushed to the
//              stack, and then some bits are changed as necessary.
// 5. Pushing the instruction pointer: The instruction point (rip) and code
//              segment (cs) are stored in the stack, allowing us to return to
//              the previous execution point after completing the interrupt
//              handler's execution.
// 6. Pushing an error code: For some exceptions, we have an error code which
//              provides more detail about the exception which has occurred.
// 7. Invoke the interrupt handler: Get the address and segment descriptor of
//              the interrupt handler and invoke it, by loading the values into
//              the rip and cs registers.

use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;
use lazy_static::lazy_static;
use crate::println;
use crate::gdt;

// Initialise the Interrupt Descriptor Table. The IDT is a table which contains
// a pointer to each of the handler functions for each exception which can
// occur.
// ---
// This has to be a static property as it needs to exist for the entire runtime
// of the application. By default static properties are immutable, so we coule 
// define this as a mutable static in order to allow us to set the interrupt 
// handlers. Unfortunately mutable statics are prone to data rases, so would 
// require an unsafe block.
// ---
// We can get around these problems by making use of the lazy_static library
// to set up the Interrupt Descriptor Table the first time it is called, and
// requires no unsafe blocks within our code, as all unsafe functionality is
// abstracted away into the lazy_static library.
lazy_static! {
    // Initialise the Interrupt Descriptor Table
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        
        // Set the handler functions for the exceptions we currently handle.
        idt.breakpoint.set_handler_fn(breakpoint_handler);

        // This is an unsafe operation becuase we need to ensure the given stack
        // is valid and not used by any other exception.
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        
        // Return the IDT
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

// Breakpoint handler, typically used in debuggers to pause execution.
extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut InterruptStackFrame) {
        println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// Double Fault Handler. This handler is typically called when an exception
// occurs within another interrupt handler, for example, if a Page Fault occurs
// and there is no Page Fault Handler, a Double Fault interrupt will be
// triggered.
// ---
// It is important to always have at least a Double Fault Handler, as without it
// a Triple Fault interrupt will be thrown, which typically results in the host
// system resetting and rebooting.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame, _error_code: u64) -> ! {
        panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}


// Testing

// Test the Breakpoint Exception Handler. We know this test passes if it
// sees execution the whole way through to the end.
#[test_case]
fn test_breakpoint_exception() {
    // Invoke a Breakpoint Exception
    x86_64::instructions::interrupts::int3();
}
