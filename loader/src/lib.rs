#![feature(allocator_api)]
#![feature(never_type)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]
#![feature(
    sync_unsafe_cell,
    ptr_as_ref_unchecked,
    cstr_display,
    adt_const_params,
    unsized_const_params,
    mem_conjure_zst
)]
#![no_std]
#![no_main]

extern crate alloc;

#[cfg(target_arch = "x86_64")]
mod apic;
mod framebuffer;
mod helpers;
mod interrupt;
mod keyboard;
mod limine_requests;
mod loader;
mod memory;
mod prelude;
mod util;

mod entry;

use alloc::string::String;
use core::{arch::naked_asm, cell::SyncUnsafeCell, ffi::CStr, fmt::Debug, slice};
use embedded_term::ConsoleOnGraphic;
use framebuffer::Framebuffer;
use ld_so_impl::{
    loader::Error,
    resolver::{ResolveError, Resolver},
    safe_addr_of,
};
use limine::memory_map::EntryType;
use limine_requests::{BASE_REVISION, FRAMEBUFFER_REQUEST, MEMORY_MAP_REQUEST};
use los_api::{hcf, println};
use talc::*;
use x86_64::{
    VirtAddr,
    registers::segmentation::{CS, SS, Segment},
    structures::{
        gdt::{Descriptor, DescriptorFlags, Entry, GlobalDescriptorTable, SegmentSelector},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
};

use crate::{limine_requests::MODULE_REQUEST, loader::RawPageLoader};

const ARENA_SIZE: usize = 0x200000;
static mut ARENA: [u8; ARENA_SIZE] = [0; ARENA_SIZE];

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

fn resolve_error(msg: &CStr, e: Error) -> ! {
    println!("{e:?}: {}", msg.display());
    hcf()
}

static RESOLVER: SyncUnsafeCell<Resolver> = SyncUnsafeCell::new(Resolver::ZERO);

#[unsafe(no_mangle)]
extern "C" fn print_bytes(data: *const u8, len: usize) {
    CONSOLE
        .wait()
        .lock()
        .write_bytes(unsafe { core::slice::from_raw_parts(data, len) });
}
