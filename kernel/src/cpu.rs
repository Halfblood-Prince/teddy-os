use core::arch::asm;

#[inline]
pub fn load_idt(base: u64, limit: u16) {
    let descriptor = DescriptorTablePointer { limit, base };
    unsafe {
        asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));
    }
}

#[inline]
pub fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn halt() {
    unsafe {
        asm!("hlt", options(nomem, nostack));
    }
}

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}
