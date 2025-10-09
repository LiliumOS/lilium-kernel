use core::ffi::c_char;

use los_api::{auxv::AuxvEnt, hcf};

#[cfg(target_arch = "x86_64")]
mod x86;

unsafe extern "C" fn begin_kernel(
    argc: isize,
    argv: *mut *mut c_char,
    envp: *mut *mut c_char,
    auxv: *mut AuxvEnt,
) -> ! {
    hcf()
}
