#![no_main]
#![no_std]

use core::ptr;

use cortex_m::asm::wfe;
use nrf52840_hal::{
    gpio::{p0, p1, Floating, Input, Level, Output, Pin, PushPull},
    pac,
};
use pdm_bare_metal as _; // global logger + panicking-behavior + memory layout

const SAMPLE_RATE: u32 = 16000; // Sample rate in Hz
const TIMER_FREQUENCY: u32 = 1_032_000; // Timer frequency in Hz
const BUFFER_LENGTH: usize = (TIMER_FREQUENCY / SAMPLE_RATE) as usize;

#[cortex_m_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    // let p = 0x20000000 as *const u32;

    let port0 = p0::Parts::new(dp.P0);
    let port1 = p1::Parts::new(dp.P1);

    let mut data: [u16; BUFFER_LENGTH] = [0; BUFFER_LENGTH];
    let data2: [u16; BUFFER_LENGTH] = [0; BUFFER_LENGTH];
    let mut counter = 0;
    static DUMMY: [u16; 1] = [0; 1];

    // intialize PDM
    let _clk: Pin<Output<PushPull>> = port1.p1_09.into_push_pull_output(Level::Low).degrade();
    let _dat: Pin<Input<Floating>> = port0.p0_08.into_floating_input().degrade();

    let pdm = dp.PDM;

    // enable PDM peripheral
    pdm.enable.write(|w| w.enable().bit(true));

    // configure PDM clock
    pdm.psel
        .clk
        .write(|w| unsafe { w.port().bit(true).pin().bits(0x09).connect().bit(true) });

    // configure PDM data pin
    pdm.psel
        .din
        .write(|w| unsafe { w.port().bit(false).pin().bits(0x08).connect().bit(true) });

    pdm.pdmclkctrl.write(|w| w.freq().default());

    // set mode to mono
    pdm.mode
        .write(|w| w.operation().bit(true).edge().left_rising());

    // set gain
    pdm.gainl.write(|w| unsafe { w.gainl().bits(0x28) });
    pdm.gainr.write(|w| unsafe { w.gainr().bits(0x28) });

    // set ratio

    pdm.ratio.write(|w| w.ratio().bit(false));

    // pdm.sample
    //     .ptr
    //     .write(|w| unsafe { w.sampleptr().bits(data.as_ptr() as u32)});

    pdm.sample
        .ptr
        .write(|w| unsafe { w.sampleptr().bits(data.as_ptr() as u32) });

    pdm.sample
        .maxcnt
        .write(|w| unsafe { w.buffsize().bits((BUFFER_LENGTH).try_into().unwrap()) });

    pdm.tasks_start.write(|w| w.tasks_start().bit(true));

    loop {
        pdm.sample
            .ptr
            .write(|w| unsafe { w.sampleptr().bits(data.as_ptr() as u32) });

        pdm.sample
            .maxcnt
            .write(|w| unsafe { w.buffsize().bits((BUFFER_LENGTH).try_into().unwrap()) });

        defmt::println!("started bit: {}", pdm.events_started.read().bits());

        defmt::println!("data at: {}", data.as_ptr());

        // while pdm.events_started.read().bits() == 0 {
        //     // wfe();
        // }

        cortex_m::asm::delay(50_000);

        pdm.events_end.write(|w| w.events_end().bit(false));

        pdm.sample
            .ptr
            .write(|w| unsafe { w.sampleptr().bits(DUMMY.as_ptr() as u32) });

        pdm.sample.maxcnt.write(|w| unsafe { w.buffsize().bits(1) });

        defmt::println!("successfully started heh");
        // pdm.events_started.write(|w| w.events_started().clear_bit());

        while pdm.events_end.read().bits() == 0 {
            // wfe();
        }

        defmt::println!("ended!");

        defmt::println!("data: {}", data);
        // pdm.events_started.write(|w| w.events_started().clear_bit());

        zero_me(&mut data);

        cortex_m::asm::delay(50_000_000);
    }

    // pdm_bare_metal::exit()
}

fn zero_me(array: &mut [u16]) {
    unsafe {
        let p = array.as_mut_ptr();
        ptr::write_bytes(p, 0, array.len());
    }
}
