use crate::bootinfo;
use core::arch::x86::__cpuid;
use x86::addr::PhysAddr;
use x86::registers::control::{Cr0, Cr0Flags};

pub static mut BOOT_INFO: bootinfo::BootInfo = bootinfo::BootInfo::new();

extern "C" {
    pub fn undefined_instruction();
    fn switch_to_long_mode(
        boot_info: &'static bootinfo::BootInfo,
        entry_point: u32,
        stack_addr: u32,
    ) -> !;
}

pub fn num_cores() -> u32 {
    unsafe {
        let res = __cpuid(0x8000_0008);
        (res.ecx & 0xFF) + 1
    }
}

pub fn apic_id() -> u8 {
    unsafe {
        let res = __cpuid(0x0000_0001);
        (res.ebx >> 24) as u8
    }
}

#[no_mangle]
unsafe extern "C" fn smp_main() {
    // Load interrupt handlers for x86 mode
    log::info!("Core {} says hello", apic_id());

    // Load exception handler in case of an error
    crate::interrupts::init();

    // Enable all media extensions
    crate::media_extensions::enable_all();

    // Get stack address for this core
    let stack_addr: u32 = BOOT_INFO.cores[apic_id() as usize]
        .get_stack_start()
        .expect("Forgot to instantiate kernel stack");
    log::debug!("Stack addr: {:#x}", stack_addr);

    // Enable mmu features
    // and set cr3 register with memory map
    crate::mmu::setup_mmu(PhysAddr::new(BOOT_INFO.page_table_addr));

    // Enable write through
    // Enable caches
    let mut cr0 = Cr0::read();
    cr0.remove(Cr0Flags::NOT_WRITE_THROUGH);
    cr0.remove(Cr0Flags::CACHE_DISABLE);
    Cr0::write(cr0);

    // Switch to long mode
    let entry_addr = BOOT_INFO.kernel_entry_addr;
    log::debug!("Switching to long mode...");
    switch_to_long_mode(&BOOT_INFO, entry_addr, stack_addr);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    use x86::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }

    panic!("Failed to exit Qemu");
}
