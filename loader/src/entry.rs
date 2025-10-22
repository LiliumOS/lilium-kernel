use core::cell::SyncUnsafeCell;

use embedded_term::ConsoleOnGraphic;
use limine::memory_map::EntryType;
use los_api::{hcf, println};

use crate::{
    CONSOLE, RESOLVER,
    framebuffer::Framebuffer,
    limine_requests::{FRAMEBUFFER_REQUEST, MEMORY_MAP_REQUEST},
};

#[cfg(target_arch = "x86_64")]
mod x86_64;

const STACK_SIZE: usize = 0x10000;

#[repr(C, align(4096))]
pub struct PageAlign<T>(T);

#[unsafe(link_section = ".bss.stack")]
static mut STACK: PageAlign<[u8; STACK_SIZE]> = PageAlign([0; STACK_SIZE]);

const INTR_STACK_SIZE: usize = 0x1000;

#[inline]
pub fn portable_entry(postinit_cb: impl FnOnce()) -> ! {
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

    postinit_cb();

    let dyn_ent = ld_so_impl::dynamic_section();

    unsafe {
        (*RESOLVER.get()).resolve_object(
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
