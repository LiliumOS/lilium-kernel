use core::slice;

use crate::prelude::*;

use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts::Us104Key};
use spin::{Lazy, Mutex};
use x86_64::{
    instructions::port::Port,
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

use crate::{CONSOLE, apic::lapic};

use los_api::{arch::x86_64::*, rand::add_enthropy};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,
    ApicError,
    ApicSpurious,
    Keyboard,
    Mouse,
}

struct HandlerHelpers<const NAME: &'static str>;

impl<const NAME: &'static str> HandlerHelpers<NAME> {
    pub fn halt_interrupt<R: InterruptResult>(x: &mut InterruptStackFrame) -> R {
        let rip = x.instruction_pointer;
        let cs = x.code_segment;
        println!("Caught #{NAME} from {cs:?}:{rip:p}");
        hcf()
    }

    pub fn halt_exception<R: InterruptResult, E: InterruptErrorCode + core::fmt::Debug>(
        x: &mut InterruptStackFrame,
        errc: E,
    ) -> R {
        let rip = x.instruction_pointer;
        let cs = x.code_segment;
        println!("Caught #{NAME}({errc:?}) from {cs:?}:{rip:p}");
        hcf()
    }
}

pub static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();

    idt.invalid_opcode
        .set_handler_fn(interrupt_handler(HandlerHelpers::<"UD">::halt_interrupt));
    idt.page_fault
        .set_handler_fn(exception_handler(HandlerHelpers::<"PF">::halt_exception));

    idt.general_protection_fault
        .set_handler_fn(exception_handler(HandlerHelpers::<"GP">::halt_exception));
    idt.stack_segment_fault
        .set_handler_fn(exception_handler(HandlerHelpers::<"PF">::halt_exception));

    idt.breakpoint
        .set_handler_fn(interrupt_handler(HandlerHelpers::<"BP">::halt_interrupt));
    unsafe {
        idt.debug
            .set_handler_fn(interrupt_handler(HandlerHelpers::<"DB">::halt_interrupt))
            .set_stack_index(1)
    };
    unsafe {
        idt.double_fault
            .set_handler_fn(exception_handler(HandlerHelpers::<"DF">::halt_exception))
            .set_stack_index(0);
    }
    idt[InterruptIndex::Timer as u8].set_handler_fn(interrupt_handler(timer_interrupt));
    idt[InterruptIndex::Keyboard as u8].set_handler_fn(interrupt_handler(keyboard_interrupt));

    return idt;
});

static KEYBOARD: Mutex<Keyboard<Us104Key, ScancodeSet1>> = Mutex::new(Keyboard::new(
    ScancodeSet1::new(),
    Us104Key,
    HandleControl::Ignore,
));

fn timer_interrupt(_frame: &mut InterruptStackFrame) {
    let mut b: u32;
    unsafe {
        core::arch::asm!("rdtsc", out("eax") b, out("rdx") _, options(nomem, nostack, preserves_flags));
    }

    add_enthropy(&b.to_ne_bytes());

    unsafe {
        lapic().end_of_interrupt();
    }
}

fn keyboard_interrupt(_frame: &mut InterruptStackFrame) {
    let scancode = unsafe { Port::new(0x60).read() };
    add_enthropy(slice::from_ref(&scancode));
    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(decoded_key) = keyboard.process_keyevent(key_event) {
            match decoded_key {
                DecodedKey::RawKey(_) => (), // TODO: handle
                DecodedKey::Unicode(x) => {
                    write!(CONSOLE.wait().lock(), "{x}").unwrap();
                }
            }
        }
    }
    unsafe {
        lapic().end_of_interrupt();
    }
}
