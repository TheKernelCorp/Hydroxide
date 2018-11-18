use lazy_static::lazy_static;
use x86_64::{
    VirtAddr,
    structures::{
        tss::TaskStateSegment,
        gdt::{
            SegmentSelector,
            GlobalDescriptorTable,
            Descriptor,
        },
    },
    instructions::{
        segmentation::{set_cs, load_ds},
        tables::load_tss,
    },
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            stack_start + STACK_SIZE // stack_end
        };
        tss
    };
}

fn kernel_data_segment() -> Descriptor {
    use x86_64::structures::gdt::DescriptorFlags as Flags;

    let flags = Flags::USER_SEGMENT | Flags::PRESENT | Flags::LONG_MODE;
    Descriptor::UserSegment(flags.bits())
}

lazy_static! {
    static ref STATIC_GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(kernel_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, data_selector, tss_selector })
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
            load_ds(STATIC_GDT.1.data_selector);

            // Load the task state register
            load_tss(STATIC_GDT.1.tss_selector); // ltr
        }
    }
}