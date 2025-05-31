use crate::prelude::*;

use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts::Us104Key};
use spin::{Lazy, Mutex};
use x86_64::{
    instructions::port::Port, registers::control::Cr2, structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}
};

use crate::{CONSOLE, apic::lapic};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,
    ApicError,
    ApicSpurious,
    Keyboard,
    Mouse,
}

pub static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();

    idt.double_fault.set_handler_fn(double_fault);
    idt.general_protection_fault.set_handler_fn(general_protection_fault);
    idt.page_fault.set_handler_fn(page_fault);
    idt[InterruptIndex::Timer as u8].set_handler_fn(timer_interrupt);
    idt[InterruptIndex::Keyboard as u8].set_handler_fn(keyboard_interrupt);

    return idt;
});

static KEYBOARD: Mutex<Keyboard<Us104Key, ScancodeSet1>> =
    Mutex::new(Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore));

extern "x86-interrupt" fn double_fault(frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!("double fault detected, stopping. stack frame: {frame:?}");
}

extern "x86-interrupt" fn general_protection_fault(frame: InterruptStackFrame, error_code: u64) {
    panic!("GP fault detected (error code: {error_code}), stopping. stack frame: {frame:?}");
}

extern "x86-interrupt" fn page_fault(frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    panic!("page fault detected (error code: {error_code:?}) at address {:?}, stopping. stack frame: {frame:?}", Cr2::read().unwrap());
}

extern "x86-interrupt" fn timer_interrupt(_frame: InterruptStackFrame) {
    // No-op for now
    unsafe {
        lapic().end_of_interrupt();
    }
}

extern "x86-interrupt" fn keyboard_interrupt(_frame: InterruptStackFrame) {
    let scancode = unsafe { Port::new(0x60).read() };
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
