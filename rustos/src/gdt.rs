// The Global Descriptor Table contains segments of the program, and was used on
// older systems to isolate programs from eachother, prior to paging. While 
// segmentation is no longer supported in 64-bit mode, the GTD is still around
// for various  functions such as kernel or user mode configuration and TSS 
// loading.
// ---
// When an exception occcurs, the x86_64 architecture is capable of switching
// to another, known-good, stack. This is done at the hardware level, so is
// completed prior to the CPI pushing the exception stack frame. The Interrupt
// Stack Table (IST) implements the switching mechanism. The IST consists of a
// table of 7 points to known-good stacks.
// ---
// The IST is a contained within a legacy structure called the Task State
// Segment (TSS). The TSS has changed since it was used on 32-bit systems, now
// only containing the IST, the Privilege Stack Table and the pointer to the I/O
// port permission bitmap.
// ---
// The Privilege Stack Table is used when the privilege level changes. (e.g. an
// exception occurs while in user mode [level 3], the CPU will typically switch
// to kernel mode [level 0] before invoking the exception handler). In this
// example, the CPU would switch to the stack in the 0th index of the PST.

use x86_64::VirtAddr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::structures::gdt::SegmentSelector;
use lazy_static::lazy_static;

// The 0th IST entry will be the double fault stack.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

//
struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

// Lazy static to ensure that the TaskStateSegment is initialised on first call.
lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // Create the Double Fault stack at the desired entry within the IST.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            // Set up an array as the underlying stack data structure. The stack
            // consists of STACK_SIZE u8 integers.
            // ---
            // This stack has no guard page to prevent a stack overflow, which
            // means we shouldn't do anything too stack-heavy within the double
            // fault handler, becuase a stack overflow could corrupt the memory
            // below the stack.
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe {&STACK});
            let stack_end = stack_start + STACK_SIZE;

            stack_end
        };

        tss
    };

    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            }
        )
    };
}

pub fn init() {
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();

    // Use set_cs to reload the Code Segment register, and use load_tss to load
    // the TSS. These are considered unsafe operations as they may break memory
    // safety by loading invalid selectors.
    unsafe {
        set_cs(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
