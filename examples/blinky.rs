//! Blinks LED1 and LED2 on Aurix Lite Kit V2. Blinks faster when BUTTON1 is pressed.

#![cfg_attr(target_arch = "tricore", no_main)]
#![cfg_attr(target_arch = "tricore", no_std)]

#[cfg(target_arch = "tricore")]
tc37x_rt::entry!(main);

use embedded_hal::digital::StatefulOutputPin;
use core::time::Duration;
use tc37x_hal::gpio::GpioExt;
use tc37x_hal::log::info;
use tc37x_hal::pac;

pub enum State {
    NotChanged = 0,
    High = 1,
    Low = 1 << 16,
    Toggled = (1 << 16) | 1,
}

fn main() -> ! {
    #[cfg(not(target_arch = "tricore"))]
    let _report = tc37x_hal::tracing::print::Report::new();

    #[cfg(not(target_arch = "tricore"))]
    env_logger::init();

    let gpio00 = pac::PORT_00.split();

    let mut led1 = gpio00.p00_5.into_push_pull_output();
    let mut led2 = gpio00.p00_6.into_push_pull_output();
    let button1 = gpio00.p00_7.into_input();

    let mut was_pressed = false;

    loop {
        let is_pressed = button1.is_low();

        if is_pressed != was_pressed {
            was_pressed = is_pressed;
            if is_pressed {
                info!("Button pressed");
            } else {
                info!("Button released");
            }
        }

        let period = Duration::from_millis(if is_pressed { 50 } else { 500 });

        // Test set_low
        led1.set_low();

        // Test toggle
        led2.toggle();

        wait_nop(period);

        // Test high
        led1.set_high();

        // Test is_set_high
        if led1.is_set_high().unwrap_or_default() {
            led2.set_low();
        }

        // Test is_set_low
        if led1.is_set_low().unwrap_or_default() {
            led2.set_high();
        }

        wait_nop(period);
    }
}

fn wait_nop(period: Duration) {
    #[cfg(target_arch = "tricore")]
    {
        let n = period.as_micros() / 5;
        for _ in 0..n {
            unsafe { core::arch::asm!("nop") };
        }
    }

    #[cfg(not(target_arch = "tricore"))]
    std::thread::sleep(period);
}
