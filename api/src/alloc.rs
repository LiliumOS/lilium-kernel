use core::ffi::c_void;

#[repr(usize)]
pub enum PagePool {
    Global,
    CoreLocal,
    InterruptLocal,
}

pub unsafe fn raw_alloc_page(
    npages: usize,
    vaddr_hint: *mut c_void,
    pool: PagePool,
) -> *mut c_void {
    unsafe { super::raw_kalloc(npages, vaddr_hint, pool) }
}
