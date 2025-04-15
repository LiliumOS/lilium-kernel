#![feature(naked_functions)]
#![feature(never_type)]
#![no_std]
#![no_main]

mod framebuffer;
mod interrupt;
mod memory;

use core::{arch::naked_asm, fmt::Write};

use acpi::AcpiTables;
use embedded_term::ConsoleOnGraphic;
use framebuffer::Framebuffer;
use limine::{
    BaseRevision,
    memory_map::EntryType,
    request::{
        FramebufferRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker, RsdpRequest,
    },
};
use memory::BasicAcpiHandler;
use talc::*;

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

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

static CONSOLE: spin::Once<spin::Mutex<ConsoleOnGraphic<Framebuffer<'static>>>> = spin::Once::new();

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        write!(CONSOLE.wait().lock(), $($arg)*).unwrap();
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        CONSOLE.wait().lock().write_str("\n").unwrap();
    }};
    ($($arg:tt)*) => {{
        writeln!(CONSOLE.wait().lock(), $($arg)*).unwrap();
    }};
}

#[unsafe(no_mangle)]
#[naked]
unsafe extern "C" fn kmain() -> ! {
    unsafe {
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

    let Some(rsdp_response) = RSDP_REQUEST.get_response() else {
        hcf();
    };
    let acpi_tables =
        unsafe { AcpiTables::from_rsdp(BasicAcpiHandler::new(), rsdp_response.address()) };

    hcf();
}

#[cfg(not(test))]
#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    println!("\x1b[31;1merror: the OS encountered a panic. {info}");
    hcf();
}

fn hcf() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
