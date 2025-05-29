use crate::{interrupt::InterruptIndex, prelude::*};

use acpi::{AcpiTables, InterruptModel, PlatformInfo, platform::interrupt::Apic};
use alloc::alloc::Global;
use x2apic::{
    ioapic::{IoApic, IrqMode, RedirectionTableEntry},
    lapic::{LocalApic, LocalApicBuilder},
};
use x86_64::instructions::interrupts;

use crate::{
    interrupt::IDT,
    limine_requests::RSDP_REQUEST,
    memory::{BasicAcpiHandler, map_physical_region},
    util::UnsafeSync,
};

#[derive(Clone, Copy)]
#[repr(u8)]
enum IrqVector {
    Keyboard = 1,
    Mouse = 12,
}

static ACPI: UnsafeSync<spin::Lazy<AcpiTables<BasicAcpiHandler>>> = unsafe {
    UnsafeSync::new(spin::Lazy::new(|| {
        let Some(rsdp_response) = RSDP_REQUEST.get_response() else {
            panic!("couldn't locate RSDP table");
        };
        AcpiTables::from_rsdp(BasicAcpiHandler, rsdp_response.address()).unwrap()
    }))
};

static APIC: UnsafeSync<spin::Lazy<Apic<'static, Global>>> = unsafe {
    UnsafeSync::new(spin::Lazy::new(|| {
        let platform_info = PlatformInfo::new(&ACPI).unwrap();
        let InterruptModel::Apic(apic) = platform_info.interrupt_model else {
            panic!("My AMD64 system doesn't support APIC? What???")
        };
        apic
    }))
};

pub fn lapic() -> LocalApic {
    LocalApicBuilder::new()
        .set_xapic_base(
            unsafe { map_physical_region(APIC.local_apic_address as usize, 1024) } as u64,
        )
        .timer_vector(InterruptIndex::Timer as usize)
        .error_vector(InterruptIndex::ApicError as usize)
        .spurious_vector(InterruptIndex::ApicSpurious as usize)
        .build()
        .unwrap()
}

pub fn init() {
    let mut lapic = lapic();

    println!("{lapic:?}");

    for ioapic in &*APIC.io_apics {
        unsafe {
            let mut ioapic = IoApic::new(map_physical_region(ioapic.address as usize, 1024) as u64);
            println!("{ioapic:?}");

            ioapic.init(32);
            add_ioapic_entry(&mut ioapic, IrqVector::Keyboard, InterruptIndex::Keyboard);
            add_ioapic_entry(&mut ioapic, IrqVector::Mouse, InterruptIndex::Mouse);
        }
    }

    IDT.load();

    unsafe {
        lapic.enable();
    }

    interrupts::enable();
}

unsafe fn add_ioapic_entry(ioapic: &mut IoApic, irq: IrqVector, vector: InterruptIndex) {
    let lapic = lapic();
    let mut entry = RedirectionTableEntry::default();
    entry.set_mode(IrqMode::Fixed);
    entry.set_dest(unsafe { lapic.id() } as u8);
    entry.set_vector(vector as u8);
    unsafe {
        ioapic.set_table_entry(irq as u8, entry);
        ioapic.enable_irq(irq as u8);
    }
}
