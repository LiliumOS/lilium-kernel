pub use lilium_sys::sys::auxv::*;

/// [`AuxEnt::a_value`] poinrs to
pub const AT_LILIUM_OS_BOOT_PART_GPT: usize = 96;
pub const AT_LILIUM_OS_BOOT_PART_MBR: usize = 97;

#[cfg(target_arch = "x86_64")]
pub const AT_LILIUM_OS_GDT_BASE: usize = 108;
#[cfg(target_arch = "x86_64")]
pub const AT_LILIUM_OS_IDT_BASE: usize = 109;
