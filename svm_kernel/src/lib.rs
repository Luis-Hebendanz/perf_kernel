#![feature(result_contains_err)]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_mut_refs)]
#![feature(asm)]
#![feature(test)]
#![no_std]

pub mod acpi;
pub mod acpi_regs;
pub mod allocator;
pub mod apic;
pub mod apic_regs;
pub mod bench;
pub mod default_interrupt;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod mylog;
pub mod print;
pub mod serial;
pub mod smp;
pub mod time;
pub mod vga;
pub mod networking;
pub mod pci;
pub mod rtl8139;

extern crate alloc;

use x86_64::structures::paging::OffsetPageTable;
/*
 * Use an exit code different from 0 and 1 to
 * differentiate between qemu error or kernel quit
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

// Write to port 0xf4 to exit qemu
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }

    panic!("Failed to exit Qemu");
}

// All kernel inits summed up
pub fn init(boot_info: &'static bootloader::bootinfo::BootInfo) {
    use x86_64::VirtAddr;

    // Check support of hardware features needed for benchmarking
    bench::check_support();

    // Load gdt into current cpu with lgdt
    // Also set code and tss segment selector registers
    gdt::init();

    // Load idt into the current cpu with lidt
    interrupts::load_idt();

    // Measure speed of rtsc once
    unsafe {
        time::calibrate();
    }

    // Create OffsetPageTable instance by
    // calculating address with: Cr3::read() + offset from bootloader
    let mut mapper: OffsetPageTable =
        unsafe { memory::init(VirtAddr::new(boot_info.physical_memory_offset)) };

    // Create FrameAllocator instance
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::new(&boot_info.memory_map) };

    // Initialize the heap allocator
    // by mapping the heap pages
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap init failed");

    // Parse acpi tables once
    unsafe {
        acpi::init_acpi_table(&mut mapper, &mut frame_allocator);
    }
    let acpi = acpi::get_acpi_table();

    // TODO: Map in bootloader the apic page
    // TODO: Read the apic id as soon as possible
    // TODO: Make core local storage globally available

    log::info!("Init apic controller");
    // Initialize apic controller
    unsafe {
        interrupts::APIC
            .lock()
            .init(&mut mapper, &mut frame_allocator, &acpi);
    }

    // Enable interrupts
    log::info!("Enabling interrupts");
    x86_64::instructions::interrupts::enable();


    // Search for pci devices
    unsafe {
        pci::init();
    };
    // unsafe {
    //     let apic = interrupts::APIC.lock();
    //     smp::init(&apic, &acpi);
    //     if apic.id.unwrap() < acpi.apics.as_ref().unwrap().last().unwrap().id {
    //         apic.mp_init(apic.id.unwrap() + 1, boot_info.smp_trampoline);
    //     }
    // }

    // let mut mem_mb = boot_info.max_phys_memory / 1024 / 1024;

    // if mem_mb % 1024 == 1023 {
    //     mem_mb += 1;
    // }
    // log::info!("Max physical memory: {} Gb", mem_mb / 1024);
    for device in pci::DEVICES.lock().iter() {
        unsafe {
            device.init(&mut mapper, &mut frame_allocator);
        };
    }
    exit_qemu(QemuExitCode::Success);
}

/*
 * TESTING CODE
 */
#[cfg(test)]
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
// Entry point for `cargo test`
#[cfg(test)]
entry_point!(kernel_test_main);
#[cfg(test)]
fn kernel_test_main(_boot_info: &'static BootInfo) -> ! {
    init();

    // Function not visible because gets generated by cargo test
    // automatically
    test_main();
    loop {}
}

// panic hanlder called only in cargo test
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

// Gets array of functions annotated with #[test_case]
pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

// Prints panic error and quits qemu
#[allow(unreachable_code)]
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    println!("[failed]\n");
    println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/* Creates the `Testable` trait
 * which helps printing the test function name
 * in the logs when executing cargo test
 */
pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        print!("[ok]\n");
    }
}
