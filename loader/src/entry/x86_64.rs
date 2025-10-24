use los_api::{hcf, helpers::Align16, println};
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
    apic::{self, init},
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

pub fn fill_init_buffer<F: FnOnce(&[u8])>(f: F) {
    let mut init_buf = [0u8; 16];
    let range = init_buf.as_mut_ptr_range();
    if lc_crypto::is_x86_feature_detected!("rdseed") {
        unsafe {
            core::arch::asm!(
                "2:",
                "lea rdi, [{in}+8]",
                "rdseed rax",
                "cmovc qword ptr [{in}], rax",
                "cmovc {in}, rdi",
                "cmp {in}, {end}",
                "jne 2b",
                in = inout(reg) range.start => _,
                end = in(reg) range.end,
                out("rax") _,
                out("rdi") _,
            )
        }
    } else if lc_crypto::is_x86_feature_detected!("rdrand") {
        unsafe {
            core::arch::asm!(
                "2:",
                "lea rdi, [{in}+8]",
                "rdrand rax",
                "cmovc qword ptr [{in}], rax",
                "cmovc {in}, rdi",
                "cmp {in}, {end}",
                "jne 2b",
                in = inout(reg) range.start => _,
                end = in(reg) range.end,
                out("rax") _,
                out("rdi") _,
                options(nostack),
            )
        }
    } else {
        let mut real_buf = [0u16; 8];
        real_buf[0] = ((fill_init_buffer::<F> as usize) >> 10) as u16;

        unsafe {
            core::arch::asm!("mfence", "rdtsc", "shl eax, 3", out("ax") real_buf[1], out("rdx") _, options(nomem, nostack));
        }

        for x in 2..8 {
            let mut membuf = Align16([0u8; 512]);
            let mut scratch = 1u64;
            unsafe {
                core::arch::asm!(
                    "lfence",
                    "rdpmc",
                    "finit",
                    "fild [{scratch}]",
                    "fldpi",
                    "fadd",
                    "fsin",
                    "fld [{scratch}]",
                    "fdiv st1",
                    "fxsave [{membuf}]",
                    "mfence",
                    "mov esi, eax",
                    "rdpmc",
                    "fxrstor [{membuf}]",
                    "sub eax, esi",
                    out("ax") real_buf[x],
                    out("rsi") _,
                    in("rcx") 0,
                    membuf = in(reg) &raw mut membuf,
                    scratch = in(reg) &raw mut scratch,
                )
            }
        }

        init_buf = bytemuck::must_cast(real_buf);
    }

    f(&init_buf);
}
