#![no_main]
#![no_std]

use core::{ptr, sync::atomic::compiler_fence, sync::atomic::Ordering::SeqCst};

use cortex_m;
use nrf52840_hal::{
    gpio::{p0, p1, DriveConfig, Floating, Input, Level, OpenDrainConfig, Output, Pin, PushPull},
    pac::{self, pdm::SAMPLE},
};
use pdm_bare_metal as _; // global logger + panicking-behavior + memory layout

const SAMPLE_RATE: u32 = 16000; // Sample rate in Hz
const TIMER_FREQUENCY: u32 = 1_032_000; // Timer frequency in Hz
const BUFFER_LENGTH: usize = (TIMER_FREQUENCY / SAMPLE_RATE) as usize;
// const BUFFER_LENGTH: usize = 1_000;

static DATA: [u16; BUFFER_LENGTH] = [0; BUFFER_LENGTH];

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("DATA: {}", ptr::addr_of!(DATA));
    let mut data_ptr: u32 = 0x20000000;
    data_ptr += ptr::addr_of!(DATA) as u32;
    // let data: [u16; BUFFER_LENGTH] = [0; BUFFER_LENGTH];

    defmt::println!("pointer: {}", data_ptr);

    let dp = pac::Peripherals::take().unwrap();

    let port0 = p0::Parts::new(dp.P0);
    let port1 = p1::Parts::new(dp.P1);

    port1
        .p1_09
        .into_push_pull_output_drive(Level::Low, DriveConfig::Standard0Standard1);

    port0.p0_08.into_floating_input();

    let pdm = dp.PDM;

    // configure PDM clock
    pdm.psel
        .clk
        .write(|w| unsafe { w.port().bit(true).pin().bits(0x09).connect().bit(true) });

    // configure PDM data pin
    pdm.psel
        .din
        .write(|w| unsafe { w.port().bit(false).pin().bits(0x08).connect().bit(true) });

    pdm.pdmclkctrl.write(|w| w.freq().default());

    // set mode to stereo
    pdm.mode
        .write(|w| w.operation().bit(false).edge().left_rising());

    // set gain
    pdm.gainl.write(|w| unsafe { w.gainl().bits(0x28) });
    pdm.gainr.write(|w| unsafe { w.gainr().bits(0x28) });

    // set ratio

    pdm.ratio.write(|w| w.ratio().bit(false));

    compiler_fence(SeqCst);

    // enable PDM peripheral
    pdm.enable.write(|w| w.enable().bit(true));

    compiler_fence(SeqCst);

    pdm.sample
        .ptr
        .write(|w| unsafe { w.sampleptr().bits(data_ptr as u32) });

    pdm.sample
        .maxcnt
        .write(|w| unsafe { w.buffsize().bits((BUFFER_LENGTH).try_into().unwrap()) });

    pdm.tasks_start.write(|w| w.tasks_start().bit(true));

    loop {
        pdm.sample
            .ptr
            .write(|w| unsafe { w.sampleptr().bits(data_ptr as u32) });

        pdm.sample
            .maxcnt
            .write(|w| unsafe { w.buffsize().bits((BUFFER_LENGTH).try_into().unwrap()) });

        compiler_fence(SeqCst);

        pdm.events_end.write(|w| w.events_end().bit(false));
        defmt::println! {"clear end bit"};

        compiler_fence(SeqCst);

        while pdm.events_end.read().bits() == 0 {
            cortex_m::asm::delay(100);
        }

        compiler_fence(SeqCst);

        unsafe {
            defmt::println!(
                "data {:x}: {}",
                data_ptr,
                ptr::read_volatile(data_ptr as *const [u16; BUFFER_LENGTH])
            );
        }

        defmt::println!("data: {}", data_ptr);

        defmt::println!(
            "address reg {:x}, address var: {:x}",
            pdm.sample.ptr.read().bits(),
            data_ptr
        );

        compiler_fence(SeqCst);
    }

    // pdm_bare_metal::exit()
}
