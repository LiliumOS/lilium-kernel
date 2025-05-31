#![feature(allocator_api)]
#![feature(never_type)]
#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

extern crate alloc;

mod apic;
mod framebuffer;
mod interrupt;
mod keyboard;
mod limine_requests;
mod memory;
mod prelude;
mod util;

use alloc::string::String;
use core::{arch::naked_asm, slice};
use elf_loader::{Loader, mmap::Mmap, object::ElfBinary};
use embedded_term::ConsoleOnGraphic;
use framebuffer::Framebuffer;
use limine::memory_map::EntryType;
use limine_requests::{BASE_REVISION, FRAMEBUFFER_REQUEST, MEMORY_MAP_REQUEST};
use los_api::{hcf, println};
use talc::*;

use crate::limine_requests::MODULE_REQUEST;

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

    let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response() else {
        hcf();
    };
    for entry in memory_map_response.entries() {
        let entry_type = entry.entry_type;
        let base = entry.base;
        let length = entry.length;
        let entry_type_str = match entry_type {
            EntryType::USABLE => "Usable",
            EntryType::RESERVED => "Reserved",
            EntryType::ACPI_RECLAIMABLE => "ACPI (Reclaimable)",
            EntryType::ACPI_NVS => "ACPI (NVS)",
            EntryType::BAD_MEMORY => "Bad memory",
            EntryType::BOOTLOADER_RECLAIMABLE => "Bootloader (Reclaimable)",
            EntryType::EXECUTABLE_AND_MODULES => "Executable and Modules",
            EntryType::FRAMEBUFFER => "Framebuffer",
            _ => unreachable!(),
        };
        let color_code = if entry_type == EntryType::USABLE {
            "\x1b[32m"
        } else {
            "\x1b[0m"
        };
        println!(
            "{color_code}{length:#018X} @ [{base:#018X} - {:#018X}]: {entry_type_str}",
            base + length
        );
    }

    apic::init();

    let mut loader = Loader::<ElfMmapImpl>::new();

    for module in MODULE_REQUEST.get_response().unwrap().modules() {
        let lib = loader
            .easy_load_dylib(ElfBinary::new(
                &*String::from_utf8_lossy(module.path().to_bytes()),
                unsafe { slice::from_raw_parts(module.addr(), module.size() as usize) },
            ))
            .unwrap();
        let lib = lib.easy_relocate(core::iter::empty(), &|_| None).unwrap();
        let module_init = unsafe { lib.get::<fn()>("module_init").unwrap() };
        module_init();
    }

    hcf();
}

struct ElfMmapImpl;

impl Mmap for ElfMmapImpl {
    unsafe fn mmap(
        addr: Option<usize>,
        len: usize,
        prot: elf_loader::mmap::ProtFlags,
        flags: elf_loader::mmap::MapFlags,
        offset: usize,
        fd: Option<i32>,
        need_copy: &mut bool,
    ) -> elf_loader::Result<core::ptr::NonNull<core::ffi::c_void>> {
        todo!()
    }

    unsafe fn mmap_anonymous(
        addr: usize,
        len: usize,
        prot: elf_loader::mmap::ProtFlags,
        flags: elf_loader::mmap::MapFlags,
    ) -> elf_loader::Result<core::ptr::NonNull<core::ffi::c_void>> {
        todo!()
    }

    unsafe fn munmap(
        addr: core::ptr::NonNull<core::ffi::c_void>,
        len: usize,
    ) -> elf_loader::Result<()> {
        todo!()
    }

    unsafe fn mprotect(
        addr: core::ptr::NonNull<core::ffi::c_void>,
        len: usize,
        prot: elf_loader::mmap::ProtFlags,
    ) -> elf_loader::Result<()> {
        todo!()
    }
}

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
