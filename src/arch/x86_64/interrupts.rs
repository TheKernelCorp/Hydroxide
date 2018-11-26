#[inline(always)]
pub unsafe fn disable() {
    asm!("cli" : : : : "intel", "volatile");
}

#[inline(always)]
pub unsafe fn enable() {
    asm!("sti
          nop
    " : : : : "intel", "volatile");
}

#[inline(always)]
pub unsafe fn halt() {
    asm!("hlt" : : : : "intel", "volatile");
}

pub fn pause() {
    unsafe {
        asm!("pause" : : : : "intel", "volatile");
    }
}
