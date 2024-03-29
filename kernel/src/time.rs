use core::arch::x86_64::_rdtsc;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;

/// The TSC tick rate in MHz
/// We "default" to a 3 GHz tick rate, which is likely within a ballpark of
/// actual tick rates if you happen to use the time routines prior to
/// calibrating the TSC.
static RDTSC_MHZ: AtomicU64 = AtomicU64::new(3_000);

/// TSC at the time of boot of the system
static RDTSC_START: AtomicU64 = AtomicU64::new(0);

/// Get the TSC rate in MHz
#[inline]
pub fn tsc_mhz() -> u64 {
    RDTSC_MHZ.load(Ordering::Relaxed)
}

/// Returns the TSC value upon a future time in microseconds
#[inline]
pub fn future(microseconds: u64) -> u64 {
    rdtsc() + (microseconds * tsc_mhz())
}

/// Returns system uptime in seconds as a float
#[inline]
pub fn uptime() -> f64 {
    elapsed(RDTSC_START.load(Ordering::Relaxed))
}

/// Return number of seconds elapsed since a prior TSC value
#[inline]
pub fn elapsed(start_time: u64) -> f64 {
    (rdtsc() - start_time) as f64 / RDTSC_MHZ.load(Ordering::Relaxed) as f64 / 1_000_000.0
}

/// Busy sleep for a given number of microseconds
#[inline]
pub fn sleep(microseconds: u64) {
    let waitval = future(microseconds);
    while rdtsc() < waitval {
        core::hint::spin_loop();
    }
}

#[inline]
pub fn rdtsc() -> u64 {
    // let mut x: u32 = 0;
    // unsafe { __rdtscp(&mut x as *mut u32) }
    unsafe { _rdtsc() }
}

/// Using the PIT, determine the frequency of rdtsc. Round this frequency to
/// the nearest 100MHz and return it.
pub unsafe fn calibrate() {
    // Store off the current rdtsc value
    let start = rdtsc();

    if RDTSC_START.load(Ordering::SeqCst) != 0 {
        return;
    }

    RDTSC_START.store(start, Ordering::Relaxed);

    // Start a timer
    let mut c0_data: Port<u8> = Port::new(0x40);
    let mut command: Port<u8> = Port::new(0x43);

    // Program the PIT to use mode 0 (interrupt after countdown) to
    // count down from 65535. This causes an interrupt to occur after
    // about 54.92 milliseconds (65535 / 1193182). We mask interrupts
    // from the PIT, thus we poll by sending the read back command
    // to check whether the output pin is set to 1, indicating the
    // countdown completed.
    command.write(0x30);
    c0_data.write(0xff);
    c0_data.write(0xff);

    loop {
        // Send the read back command to latch status on channel 0
        command.write(0xe2);

        // If the output pin is high, then we know the countdown is
        // done. Break from the loop.
        if (c0_data.read() & 0x80) != 0 {
            break;
        }
    }

    // Stop the timer
    let end = rdtsc();

    // Compute the time, in seconds, that the countdown was supposed to
    // take
    let elapsed = 65535f64 / 1193182f64;

    // Compute MHz for the rdtsc
    let computed_rate = ((end - start) as f64) / elapsed / 1000000.0;

    // Round to the nearest 100MHz value
    let rounded_rate = (((computed_rate / 100.0) + 0.5) as u64) * 100;

    // Stock the TSC rate
    RDTSC_MHZ.store(rounded_rate, Ordering::Relaxed);
}
