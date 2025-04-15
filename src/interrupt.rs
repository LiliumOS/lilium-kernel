use spin::Lazy;
use x86_64::structures::idt::InterruptDescriptorTable;

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let idt = InterruptDescriptorTable::new();

    idt[1];

    return idt;
});
