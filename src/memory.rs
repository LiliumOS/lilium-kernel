use core::ptr::NonNull;

use acpi::{AcpiHandler, PhysicalMapping};
use limine::request::ExecutableAddressRequest;
use spin::Lazy;
use x86_64::{
    VirtAddr,
    structures::paging::{OffsetPageTable, PageTable},
};

#[used]
#[unsafe(link_section = ".requests")]
static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

const HHDM_START: usize = 0xffffffff80000000;

#[derive(Clone)]
pub struct BasicAcpiHandler;

impl BasicAcpiHandler {
    pub fn new() -> Self {
        Self
    }
}

impl AcpiHandler for BasicAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        // TODO: THIS IS NOT CORRECT.
        // SAFETY: idk i'm kinda just throwing stuff at the wall right now
        // unsafe {
        //     PhysicalMapping::new(
        //         physical_address,
        //         NonNull::new((HHDM_START + physical_address) as *mut T).unwrap(),
        //         size,
        //         size,
        //         self.clone(),
        //     )
        // }
        todo!("we're just gonna stop right now");
    }

    fn unmap_physical_region<T>(_: &PhysicalMapping<Self, T>) {}
}
