use los_api::{hcf, println};
use x86_64::{
    VirtAddr, instructions,
    registers::{
        self,
        segmentation::{CS, SS, Segment},
    },
    structures::{
        gdt::{Descriptor, DescriptorFlags, GlobalDescriptorTable, SegmentSelector},
        idt::{InterruptDescriptorTable, InterruptStackFrame},
        tss::TaskStateSegment,
    },
};

use crate::{
    apic,
    entry::{INTR_STACK_SIZE, PageAlign, STACK, STACK_SIZE},
    interrupt::IDT,
    limine_requests::BASE_REVISION,
};

#[unsafe(link_section = ".bss.stack")]
static mut DF_STACK: PageAlign<[u8; INTR_STACK_SIZE]> = PageAlign([0; INTR_STACK_SIZE]);

#[unsafe(link_section = ".bss.stack")]
static mut INTR_STACK: PageAlign<[u8; INTR_STACK_SIZE * 4]> = PageAlign([0; INTR_STACK_SIZE * 4]);

#[unsafe(link_section = ".bss.stack")]
static mut PF_STACK: PageAlign<[u8; INTR_STACK_SIZE]> = PageAlign([0; INTR_STACK_SIZE]);

#[unsafe(link_section = ".bss.stack")]
static mut DB_STACK: PageAlign<[u8; INTR_STACK_SIZE]> = PageAlign([0; INTR_STACK_SIZE]);

#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn kmain() -> ! {
    core::arch::naked_asm!(
        "lea rsp, [{STACK} + {STACK_SIZE} + rip]",
        "mov rax, cr0",
        "and rax, {NOT_EM_TS}",
        "or rax, {NE}",
        "mov cr0, rax",
        "mov rax, cr4",
        "or rax, {CR4}",
        "mov cr4, rax",
        "call {kmain_real}",
        "jmp {hcf}",
        STACK = sym STACK,
        STACK_SIZE = const STACK_SIZE,
        kmain_real = sym kmain_real,
        hcf = sym hcf_real,
        NOT_EM_TS = const !(x86_64::registers::control::Cr0Flags::EMULATE_COPROCESSOR.bits() | x86_64::registers::control::Cr0Flags::TASK_SWITCHED.bits()),
        NE = const x86_64::registers::control::Cr0Flags::NUMERIC_ERROR.bits(),
        CR4 = const (x86_64::registers::control::Cr4Flags::OSFXSR.bits() | x86_64::registers::control::Cr4Flags::OSXMMEXCPT_ENABLE.bits()),
    );
}

#[unsafe(no_mangle)]
extern "C" fn kmain_real() -> ! {
    assert!(BASE_REVISION.is_supported());

    let mut tss = TaskStateSegment::new();
    tss.interrupt_stack_table[0] = VirtAddr::from_ptr(unsafe { (&raw mut DF_STACK).add(1) });
    tss.interrupt_stack_table[1] = VirtAddr::from_ptr(unsafe { (&raw mut PF_STACK).add(1) });
    tss.interrupt_stack_table[2] = VirtAddr::from_ptr(unsafe { (&raw mut DB_STACK).add(1) });
    tss.privilege_stack_table[0] = VirtAddr::from_ptr(unsafe { (&raw mut INTR_STACK).add(1) });

    // GDT with following indecies:
    // 0: Null Segment
    // 1: 16-bit Code Segment (base=0, limit=0xFFFF, Executable, Readable, DPL=0)
    // 2: 16-bit Data Segment (base=0, limit=0xFFFF, Writable, DPL=0)
    // 3: 32-bit Code Segment (base=0, limit=0xFFFFFF Pages, Executable, Readable, DPL=0)
    // 4: 32-bit Data Segment (base=0, limit=0xFFFFFF Pages, Writable, DPL=0)
    // 5: 64-bit Code Segment (base=ignored, limit=ignored, Executable, Readable, Long-mode, DPL=0)
    // 6: 64-bit Data Segment (base=ignored, limit=ignored, Writable, DPL=0)
    // 7: Absent User Segment
    // 8: TSS (IST)
    // 9: Entry 8 Continued
    // 10: 64-bit Code Segment (base=ignored, limit=ignored, Executable, Readable, Long-mode, DPL=3)
    // 11: 64-bit Data Segment (base=ignored, limit=ignored, Writable, Long-mode, DPL=3)
    // 12: Absent User Segment - Used for a 32-bit
    // 13: Absent User Segment
    // 14: Absent (For LDT)
    // 15: Entry 14 Continued
    let mut gdt = GlobalDescriptorTable::<16>::empty();

    gdt.append(Descriptor::UserSegment(
        (DescriptorFlags::EXECUTABLE
            | DescriptorFlags::WRITABLE
            | DescriptorFlags::LIMIT_0_15
            | DescriptorFlags::USER_SEGMENT)
            .bits(),
    ));
    gdt.append(Descriptor::UserSegment(
        (DescriptorFlags::WRITABLE | DescriptorFlags::LIMIT_0_15 | DescriptorFlags::USER_SEGMENT)
            .bits(),
    ));
    gdt.append(Descriptor::UserSegment(
        DescriptorFlags::KERNEL_CODE32.bits(),
    ));
    gdt.append(Descriptor::UserSegment(
        (DescriptorFlags::KERNEL_DATA).bits(),
    ));
    gdt.append(Descriptor::kernel_code_segment());
    gdt.append(Descriptor::kernel_data_segment());
    gdt.append(Descriptor::UserSegment(
        DescriptorFlags::USER_SEGMENT.bits(),
    ));
    gdt.append(unsafe { Descriptor::tss_segment_unchecked(&tss) });
    gdt.append(Descriptor::user_code_segment());
    gdt.append(Descriptor::user_data_segment());

    unsafe {
        gdt.load_unsafe();
        SS::set_reg(SegmentSelector::new(6, x86_64::PrivilegeLevel::Ring0));
        CS::set_reg(SegmentSelector::new(5, x86_64::PrivilegeLevel::Ring0));
        instructions::tables::load_tss(SegmentSelector::new(8, x86_64::PrivilegeLevel::Ring0));
    }

    IDT.load();

    super::portable_entry(|| {
        apic::init();
    })
}

#[unsafe(no_mangle)]
extern "C" fn hcf_real() -> ! {
    loop {
        // core::hint::spin_loop();
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
