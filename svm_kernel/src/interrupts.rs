use crate::gdt;
use crate::print;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

// Offset the PICs to avoid index collision with
// exceptions in the IDT
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

use pic8259_simple::ChainedPics;

// Initialize the Process Interrupt Controller
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

// IDT index numbers
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard, // 33
    Reserved0,
    COM2,
    COM1,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// Global static IDT
lazy_static::lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.invalid_opcode.set_handler_fn(invalid_op_handler);

        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
            // Use a different stack in case of kernel stack overflow
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt[InterruptIndex::Timer.as_usize()]
                .set_handler_fn(timer_interrupt_handler);
            idt[InterruptIndex::Keyboard.as_usize()]
                .set_handler_fn(keyboard_interrupt_handler);
            idt[InterruptIndex::COM2.as_usize()]
                .set_handler_fn(serial_handler);
            idt[InterruptIndex::COM1.as_usize()]
                .set_handler_fn(serial_handler);
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}


use x86_64::structures::idt::PageFaultErrorCode;
use crate::hlt_loop;
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    log::error!("EXCEPTION: PAGE FAULT");
    log::error!("Accessed Address: {:?}", Cr2::read());
    log::error!("Error Code: {:?}", error_code);
    log::error!("{:#?}", stack_frame);
    hlt_loop();
}

// Keyboard handler
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    // Initialize pc_keyboard crate
    // This will only be called on kernel start don't worry
    lazy_static::lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = {
            // Use US layout & ignore ctrl+key characters
            Mutex::new(
                Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
            )
        };
    }

    // Lock keyboard parser
    let mut keyboard = KEYBOARD.lock();
    // Create port on mem addr 0x60 to read keycode
    let mut port = Port::new(0x60);
    // Read keycode from mem
    let scancode: u8 = unsafe { port.read() };

    // Parse keycode with pc_keyboard crate
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:#?}", key),
            }
        }
    }

    // Renable interrupts again
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// Breakpoint handler
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    log::error!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// Double fault handler
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_op_handler(stack_frame: &mut InterruptStackFrame) {
    panic!("INVALID OP HANDLER\n{:#?}", stack_frame);
}

// Serial handler
extern "x86-interrupt" fn serial_handler(_stack_frame: &mut InterruptStackFrame) {
    log::info!("SERIAL HANDLER\n");

    // Renable interrupts again
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

// timer interrupt handler
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    print!(".");

    // Renable interrupts again
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

// Executed on cargo test
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}