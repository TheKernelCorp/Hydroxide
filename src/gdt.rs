use lazy_static::lazy_static;
use x86_64::{
    instructions::{segmentation::set_cs, tables::load_tss},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss
    };
}

lazy_static! {
    static ref STATIC_GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

/// Global Descriptor Table
pub struct GDT;
impl GDT {
    // Initialize the GDT
    pub fn init() {
        // Load the GDT
        STATIC_GDT.0.load();

        unsafe {
            // Reload the kernel code segment register
            set_cs(STATIC_GDT.1.code_selector);

            // Load the task state register
            load_tss(STATIC_GDT.1.tss_selector); // ltr
        }
    }
}
