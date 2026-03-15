use core::mem::MaybeUninit;

use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
const DOUBLE_FAULT_STACK_SIZE: usize = 4096 * 5;

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

static mut DOUBLE_FAULT_STACK: [u8; DOUBLE_FAULT_STACK_SIZE] = [0; DOUBLE_FAULT_STACK_SIZE];
static mut TSS: MaybeUninit<TaskStateSegment> = MaybeUninit::uninit();
static mut GDT: MaybeUninit<GlobalDescriptorTable> = MaybeUninit::uninit();
static mut SELECTORS: MaybeUninit<Selectors> = MaybeUninit::uninit();
static mut INITIALIZED: bool = false;

pub fn init() {
    unsafe {
        if INITIALIZED {
            return;
        }

        let mut tss = TaskStateSegment::new();
        let stack_start = VirtAddr::from_ptr(DOUBLE_FAULT_STACK.as_ptr());
        let stack_end = stack_start + DOUBLE_FAULT_STACK_SIZE as u64;
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
        TSS.write(tss);

        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(TSS.assume_init_ref()));
        GDT.write(gdt);
        SELECTORS.write(Selectors {
            code_selector,
            data_selector,
            tss_selector,
        });

        GDT.assume_init_ref().load();
        let selectors = SELECTORS.assume_init_ref();

        CS::set_reg(selectors.code_selector);
        SS::set_reg(selectors.data_selector);
        DS::set_reg(selectors.data_selector);
        ES::set_reg(selectors.data_selector);
        load_tss(selectors.tss_selector);

        INITIALIZED = true;
    }
}
