#![feature(allocator_api)]
#![feature(never_type)]
#![feature(abi_x86_interrupt)]
#![feature(sync_unsafe_cell, ptr_as_ref_unchecked)]
#![no_std]
#![no_main]

extern crate alloc;

mod apic;
mod framebuffer;
mod interrupt;
mod keyboard;
mod limine_requests;
mod loader;
mod memory;
mod prelude;
mod util;

use alloc::string::String;
use core::{arch::naked_asm, cell::SyncUnsafeCell, slice};
use embedded_term::ConsoleOnGraphic;
use framebuffer::Framebuffer;
use ld_so_impl::{resolver::Resolver, safe_addr_of};
use limine::memory_map::EntryType;
use limine_requests::{BASE_REVISION, FRAMEBUFFER_REQUEST, MEMORY_MAP_REQUEST};
use los_api::{hcf, println};
use talc::*;

use crate::{limine_requests::MODULE_REQUEST, loader::RawPageLoader};

const ARENA_SIZE: usize = 0x200000;
static mut ARENA: [u8; ARENA_SIZE] = [0; ARENA_SIZE];

const STACK_SIZE: usize = 0x10000;
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

#[global_allocator]
static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    // if we're in a hosted environment, the Rust runtime may allocate before
    // main() is called, so we need to initialize the arena automatically
    ClaimOnOom::new(Span::from_array(
        core::ptr::addr_of!(ARENA) as *mut [u8; ARENA_SIZE]
    ))
})
.lock();

static CONSOLE: spin::Once<spin::Mutex<ConsoleOnGraphic<Framebuffer<'static>>>> = spin::Once::new();

#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn kmain() -> ! {
    naked_asm!(
        "lea rsp, [{STACK} + {STACK_SIZE} + rip]",
        "call {kmain_real}",
        "jmp {hcf}",
        STACK = sym STACK,
        STACK_SIZE = const STACK_SIZE,
        kmain_real = sym kmain_real,
        hcf = sym hcf,
    );
}

#[unsafe(no_mangle)]
fn kmain_real() -> ! {
    assert!(BASE_REVISION.is_supported());

    let base_addr = ld_so_impl::load_addr();

    let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() else {
        hcf();
    };
    let Some(framebuffer) = framebuffer_response.framebuffers().next() else {
        hcf();
    };
    let framebuffer = Framebuffer::from(framebuffer);
    let console = ConsoleOnGraphic::on_frame_buffer(framebuffer);
    CONSOLE.call_once(|| spin::Mutex::new(console));
    println!("Hello, world!");

    println!("Base Address: {base_addr:p}");

    let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response() else {
        hcf();
    };
    // for entry in memory_map_response.entries() {
    //     let entry_type = entry.entry_type;
    //     let base = entry.base;
    //     let length = entry.length;
    //     let entry_type_str = match entry_type {
    //         EntryType::USABLE => "Usable",
    //         EntryType::RESERVED => "Reserved",
    //         EntryType::ACPI_RECLAIMABLE => "ACPI (Reclaimable)",
    //         EntryType::ACPI_NVS => "ACPI (NVS)",
    //         EntryType::BAD_MEMORY => "Bad memory",
    //         EntryType::BOOTLOADER_RECLAIMABLE => "Bootloader (Reclaimable)",
    //         EntryType::EXECUTABLE_AND_MODULES => "Executable and Modules",
    //         EntryType::FRAMEBUFFER => "Framebuffer",
    //         _ => unreachable!(),
    //     };
    //     let color_code = if entry_type == EntryType::USABLE {
    //         "\x1b[32m"
    //     } else {
    //         "\x1b[0m"
    //     };
    //     println!(
    //         "{color_code}{length:#018X} @ [{base:#018X} - {:#018X}]: {entry_type_str}",
    //         base + length
    //     );
    // }

    apic::init();

    // unsafe {
    //     RESOLVER
    //         .get()
    //         .as_mut_unchecked()
    //         .set_loader_backend(&RawPageLoader);
    // }

    let dyn_ent = ld_so_impl::dynamic_section();

    unsafe {
        RESOLVER.get().as_ref_unchecked().resolve_object(
            base_addr,
            dyn_ent,
            Some(c"lilium-loader.so"),
            core::ptr::null_mut(),
            !0,
            None,
        );
    }

    println!("Dynloader loaded");

    hcf();
}

static RESOLVER: SyncUnsafeCell<Resolver> = SyncUnsafeCell::new(Resolver::ZERO);

#[unsafe(no_mangle)]
extern "C" fn hcf_real() -> ! {
    loop {
        // core::hint::spin_loop();
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn print_bytes(data: *const u8, len: usize) {
    CONSOLE
        .wait()
        .lock()
        .write_bytes(unsafe { core::slice::from_raw_parts(data, len) });
}
