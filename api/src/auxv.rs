pub use lilium_sys::sys::auxv::*;

/// [`AuxEnt::a_value`] poinrs to
pub const AT_LILIUM_OS_BOOT_PART_GPT: usize = 96;
pub const AT_LILIUM_OS_BOOT_PART_MBR: usize = 97;
