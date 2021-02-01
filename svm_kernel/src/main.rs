#![no_std]
#![no_main]
#![feature(custom_test_frameworks)] // https://github.com/rust-lang/rfcs/blob/master/text/2318-custom-test-frameworks.md
#![test_runner(svm_kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(asm)]
/*
 * Followed the tutorial here: https://os.phil-opp.com
 * TODO: Replace builtin memcpy, memset with optimized one
 */

/* TODO:
 * Write bootloader myself to be able to enable
 * mmx,sse & float features!
 * Should also solve the lto linktime warning
 */

/*
 * This kernel has been tested on an AMD x64 processor
 * family: 0x17h, model: 0x18h
 */

use log::{error, info, LevelFilter};
use svm_kernel::mylog::LOGGER;

use bootloader::{entry_point, BootInfo};

extern crate alloc;

/*
 * KERNEL MAIN
 * The macro entry_point creates the nomangle _start func for us and checks that
 * the given function has the correct signature
 */
entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Init & set logger level
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Info);
    // panic!("Reached kernel main");

    // Initialize routine for kernel
    svm_kernel::init(boot_info);

    // This func gets generated by cargo test
  //  #[cfg(test)]
 //   test_main();

    // Busy loop don't crash
    info!("Kernel going to loop now xoxo");
    svm_kernel::hlt_loop();
}

/*
 * KERNEL PANIC HANDLER
 * Not used in cargo test
 */
//TODO: Implement a bare metal debugger
// https://lib.rs/crates/gdbstub
// https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    svm_kernel::hlt_loop();
}
