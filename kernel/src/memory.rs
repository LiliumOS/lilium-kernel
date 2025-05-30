use acpi::{AcpiHandler, PhysicalMapping};
use limine::memory_map::{self, EntryType};
use spin::{Lazy, Mutex};
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, MappedPageTable, Mapper, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB, mapper::PageTableFrameMapping,
    },
};

use crate::{MEMORY_MAP_REQUEST, limine_requests::HHDM_REQUEST, println};

struct FrameMappping;

unsafe impl PageTableFrameMapping for FrameMappping {
    fn frame_to_pointer(&self, frame: PhysFrame) -> *mut PageTable {
        let phys = frame.start_address().as_u64();
        let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();
        let virt = phys + hhdm_offset;
        virt as *mut PageTable
    }
}

static PAGE_TABLE: Lazy<PhysAddr> = Lazy::new(|| Cr3::read().0.start_address());
static PAGE_TABLE_MAPPING: Lazy<Mutex<MappedPageTable<FrameMappping>>> = Lazy::new(|| unsafe {
    println!(
        "HHDM offset: {:#X}",
        HHDM_REQUEST.get_response().unwrap().offset()
    );
    Mutex::new(MappedPageTable::new(
        &mut *(VirtAddr::new(PAGE_TABLE.as_u64() + HHDM_REQUEST.get_response().unwrap().offset())
            .as_mut_ptr()),
        FrameMappping,
    ))
});

struct BootInfoFrameAllocator {
    entries: &'static [&'static memory_map::Entry],
    next: usize,
}

impl BootInfoFrameAllocator {
    pub fn new(memory_map: &'static [&'static memory_map::Entry]) -> Self {
        Self {
            entries: memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        self.entries
            .iter()
            .filter(|e| e.entry_type == EntryType::USABLE)
            .flat_map(|entry| {
                let start = entry.base;
                let end = entry.base + entry.length;
                (start..end)
                    .step_by(4096)
                    .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
            })
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // TODO: This is a stupid solution. It works, but it is in fact stupid. Let's not do this.
        let frame = self.usable_frames().nth(self.next)?;
        self.next += 1;
        Some(frame)
    }
}

static FRAME_ALLOCATOR: Lazy<Mutex<BootInfoFrameAllocator>> = Lazy::new(|| {
    Mutex::new(BootInfoFrameAllocator::new(
        MEMORY_MAP_REQUEST.get_response().unwrap().entries(),
    ))
});

#[derive(Clone, Copy)]
pub struct BasicAcpiHandler;

pub unsafe fn map_physical_region(physical_address: usize, size: usize) -> usize {
    let offset = HHDM_REQUEST.get_response().unwrap().offset();
    let virt = physical_address + offset as usize;

    let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(virt as u64));
    let frame = PhysFrame::containing_address(PhysAddr::new(physical_address as u64));

    let mut mapper = PAGE_TABLE_MAPPING.lock();
    if mapper.translate_page(page).is_err() {
        println!("allocating a page at {physical_address:#X} with a size of {size:#X}");
        unsafe {
            mapper
                .map_to(
                    page,
                    frame,
                    PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                    &mut *FRAME_ALLOCATOR.lock(),
                )
                .unwrap()
                .flush();
        }
    }

    virt
}

impl AcpiHandler for BasicAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let virt = unsafe { map_physical_region(physical_address, size) };

        unsafe {
            PhysicalMapping::new(
                physical_address,
                core::ptr::NonNull::new(virt as *mut T).unwrap(),
                size,
                size,
                *self,
            )
        }
    }

    fn unmap_physical_region<T>(_: &PhysicalMapping<Self, T>) {}
}
