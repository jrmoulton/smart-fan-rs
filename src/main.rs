#![no_std]
#![no_main]

use core::cell::RefCell;
use critical_section::Mutex;

use esp32c3_hal::{
    clock::ClockControl,
    gpio::{Gpio9, IO},
    gpio_types::{Event, Input, Pin, PullDown},
    interrupt,
    pac::{self, Peripherals, TIMG0},
    prelude::*,
    timer::{Timer, Timer0, TimerGroup},
    Rtc,
};
use esp_backtrace as _;
use riscv_rt::entry;

static ZERO_CROSS: Mutex<RefCell<Option<Gpio9<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
static TIMER0: Mutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> = Mutex::new(RefCell::new(None));

#[entry]
#[allow(non_snake_case, clippy::empty_loop)]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let mut timer0 = timer_group0.timer0;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut button = io.pins.gpio9.into_pull_down_input();
    button.listen(Event::FallingEdge); // TODO: This might need to be updated

    interrupt::enable(pac::Interrupt::GPIO, interrupt::Priority::Priority3).unwrap();
    interrupt::enable(pac::Interrupt::TG0_T0_LEVEL, interrupt::Priority::Priority1).unwrap();

    timer0.listen();

    critical_section::with(|cs| {
        ZERO_CROSS.borrow_ref_mut(cs).replace(button);
        TIMER0.borrow_ref_mut(cs).replace(timer0);
    });

    unsafe {
        riscv::interrupt::enable();
    }

    loop {}
}

#[interrupt]
#[allow(non_snake_case)]
fn GPIO() {
    critical_section::with(|cs| {
        TIMER0
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .start(500u32.millis());
        ZERO_CROSS
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt();
    });
}
#[interrupt]
#[allow(non_snake_case)]
fn TG0_T0_LEVEL() {
    critical_section::with(|cs| {
        // TODO: Toggle the output of a pin
        TIMER0
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt();
    });
}
