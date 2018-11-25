#[macro_export]
macro_rules! read_gs_offset64 {
    ($offset:expr) => {{
        let ret: u64;
        asm!("mov $0, gs:$1" : "=r"(ret) : "i"($offset) : "memory" : "intel", "volatile");
        ret
    }};
}

#[macro_export]
macro_rules! write_gs_offset64 {
    ($offset:expr, $val:expr) => {{
        asm!("mov $0, %gs:$1" : : "r"($val), "i"($offset) : "memory" : "volatile");
    }};
}

#[macro_export]
macro_rules! offset_of {
    ($ty:ty, $field:ident) => {
        #[allow(unused_unsafe)]
        unsafe {
            &(*(0 as *const $ty)).$field as *const _ as usize
        }
    };
}
