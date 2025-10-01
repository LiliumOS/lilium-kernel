use ld_so_impl::loader::LoaderImpl;

use crate::print_bytes;

pub struct RawPageLoader;

impl LoaderImpl for RawPageLoader {
    unsafe fn alloc_base_addr(
        &self,
        udata: *mut core::ffi::c_void,
        max_pma: ld_so_impl::elf::ElfAddr,
    ) -> Result<*mut core::ffi::c_void, ld_so_impl::loader::Error> {
        todo!()
    }

    unsafe fn find(
        &self,
        soname: &core::ffi::CStr,
        udata: *mut core::ffi::c_void,
    ) -> Result<*mut core::ffi::c_void, ld_so_impl::loader::Error> {
        todo!()
    }

    unsafe fn map_phdrs(
        &self,
        phdrs: &[ld_so_impl::elf::ElfPhdr],
        map_desc: *mut core::ffi::c_void,
        base_addr: *mut core::ffi::c_void,
    ) -> Result<*mut core::ffi::c_void, ld_so_impl::loader::Error> {
        todo!()
    }

    fn read_offset(
        &self,
        off: ld_so_impl::elf::ElfOffset,
        map_desc: *mut core::ffi::c_void,
        sl: &mut [u8],
    ) -> Result<(), ld_so_impl::loader::Error> {
        todo!()
    }

    fn write_str(&self, st: &str) -> core::fmt::Result {
        unsafe {
            print_bytes(st.as_ptr(), st.len());
        }
        Ok(())
    }
}
