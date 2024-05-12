use tc375_pac::gpt120::Gpt120;

use crate::scu;

pub fn init_gpt12_timer() {
    let gpt120 = tc375_pac::GPT120;
    gpt12_init(&gpt120);
    timer_t3_init(&gpt120);
    timer_t2_init(&gpt120);
    timer_start(&gpt120);
    interrupt_init();
}

fn timer_start(gpt120: &Gpt120) {
    unsafe {
        gpt120.t3con().modify(|r| r.t3r().set(true));
    }
}

fn interrupt_init() {
    let src = tc375_pac::SRC;
    let srcr = src.gpt12().gpt12().gpt120t3();
    unsafe {
        // initialise service request
        srcr.modify(|r| {
            r.srpn() // set priority
                .set(6)
                .tos() // type of service
                .set(0) // CPU 0
                .clrr() // clear request
                .set(true)
                .sre()  // enable interrupt
                .set(true)
        })
    }
}

fn timer_t2_init(gpt120: &Gpt120) {
    unsafe {
        gpt120.t2con().modify(|r| {
            r.t2m() // T2 timer mode
                .set(4) // reload mode
                .t2i() // reload input mode
                .set(7) // both edges TxOTL
        });
        gpt120.t2().modify(|r| r.t2().set(48828)); // timer value
    }
}

fn timer_t3_init(gpt120: &Gpt120) {
    // set mode
    unsafe {
        gpt120.t3con().modify(|r| {
            r.t3m() // Timer Mode <- 0
                .set(0)
                .t3ud() // direction, down
                .set(true)
                .t3i() // timer prescalar
                .set(6) // 64 = 2^6
        });
        gpt120.t3().modify(|r| r.t3().set(48828)) // set T3 timer value
    }
}

fn gpt12_init(gpt120: &Gpt120) {
    // Enable the GPT120 Module
    scu::wdt::clear_cpu_endinit_inline();
    unsafe { gpt120.clc().modify(|r| r.disr().set(false)) }
    scu::wdt::set_cpu_endinit_inline();

    // Set GPT120 block prescalar
    unsafe {
        gpt120.t3con().modify(|r| r.bps1().set(3)) /* 2^3 = 16 */
    }
}
