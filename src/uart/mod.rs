use tc375_pac::{
    asclin0::{self, Asclin0},
    cpu0::tr,
    src::{asclin, Asclin},
    RegisterValue, SRC,
};

use crate::{gpio::PushPull, uart::uart_node::Tos};

use crate::{
    gpio::{self, gpio01, gpio20},
    scu,
    uart::uart_node::ClockSource,
};

use self::{
    port::PortNumber,
    uart_node::{NodeConfig, OutputIdx, Pins, Rx, RxSel, Tx},
};

pub mod port;
pub mod uart_node; // TODO: move?

// dirty code to get things started
pub fn print(buf: &[u8]) {
    let asclin0 = tc375_pac::ASCLIN0;
    for byte in buf {
        unsafe {
            asclin0.txdata().modify_atomic(|val| {
                val.set_raw(*byte as u32)
            });
        }
    }
}

// only supports P14 and ASCLIN0
pub fn init_uart(config: NodeConfig) {
    let asclin0 = tc375_pac::ASCLIN0;
    let p14 = tc375_pac::P14;

    enable_uart_module(&asclin0);

    // bit timing config (enable clock source for baud rate configuration)
    set_clock_source(&asclin0, config.clock_source);
    set_bit_timing(&asclin0, &config);
    set_clock_source(&asclin0, ClockSource::NoClock);

    // set loop back
    unsafe {
        asclin0.iocr().modify(|val| val.lb().set(config.look_back));
    }

    // set frame
    unsafe {
        asclin0.framecon().modify(|val| {
            val.pen()
                .set(config.frame.parity_bit)
                .odd()
                .set(config.frame.parity_type)
                .stop()
                .set(config.frame.stop_bit)
                .msb()
                .set(config.frame.shift_dir)
                .idle()
                .set(config.frame.idle_delay) // out of order config (compared to iLLD)
                .mode()
                .set(config.frame.frame_mode.into()) // out of order config
        });
        asclin0
            .datcon()
            .modify(|val| val.datlen().set(config.frame.data_length))
    }

    // set fifo
    unsafe {
        asclin0.txfifocon().modify(|val| {
            val.inw()
                .set(config.fifo.in_width)
                .intlevel()
                .set(config.fifo.tx_fifo_interrupt_level)
                .fm()
                .set(config.fifo.tx_fifo_interrupt_mode)
        });
        asclin0.rxfifocon().modify(|val| {
            val.outw()
                .set(config.fifo.out_width)
                .intlevel()
                .set(config.fifo.rx_fifo_interrupt_level)
                .fm()
                .set(config.fifo.rx_fifo_interrupt_mode)
        });
    }

    // pin mapping
    let pins = Pins {
        rx: Rx {
            port: PortNumber::_14,
            pin_index: 1,
            select: RxSel::_A,
        },
        tx: Tx {
            port: PortNumber::_14,
            pin_index: 0,
            select: OutputIdx::ALT2,
        },
    };
    port::port_mapping(&asclin0, &pins);

    // select the clock source
    set_clock_source(&asclin0, config.clock_source);

    unsafe {
        // disable all flags
        asclin0.flagsenable().modify(|val| val.set_raw(0x00000000));
        // set all flags
        asclin0
            .flagsclear()
            .modify_atomic(|val| val.set_raw(0xFFFFFFFF));
    }

    // skipping error flag setting
    // skipping software fifos

    // initialise interrupts
    let tos = &config.interrupt.type_of_service;
    assert_eq!(*tos, Tos::CPU0); // only supports CPU0 for now
    let ASCLIN_INDEX = 0;
    if config.interrupt.rx_priority > 0 {
        unsafe {
            let src = SRC.asclin().asclin()[ASCLIN_INDEX]; // supports ASCLIN0 for now
            src.asclinxrx().modify(|val| {
                val.srpn()
                    .set(config.interrupt.rx_priority)
                    .tos()
                    .set(config.interrupt.type_of_service.into())
                    .clrr()
                    .set(true)
            });
            asclin0.flagsenable().modify(|val| val.rfle().set(true));
            src.asclinxrx().modify(|val| val.sre().set(true));
        };
    }

    if config.interrupt.tx_priority > 0 {
        unsafe {
            let src = SRC.asclin().asclin()[ASCLIN_INDEX]; // supports ASCLIN0 for now
            src.asclinxtx().modify(|val| {
                val.srpn()
                    .set(config.interrupt.tx_priority)
                    .tos()
                    .set(config.interrupt.type_of_service.into())
                    .clrr()
                    .set(true)
            });
            asclin0.flagsenable().modify(|val| val.tfle().set(true));
            src.asclinxtx().modify(|val| val.sre().set(true));
        }
    }

    if config.interrupt.er_priority > 0 {
        let src = SRC.asclin().asclin()[ASCLIN_INDEX]; // supports ASCLIN0 for now
        unsafe {
            src.asclinxerr().modify(|val| {
                val.srpn()
                    .set(config.interrupt.er_priority)
                    .tos()
                    .set(config.interrupt.type_of_service.into())
                    .clrr()
                    .set(true)
            });
            asclin0.flagsenable().modify(|val| val.pee().set(true));
            src.asclinxerr().modify(|val| val.sre().set(true));
        }
    }

    // enable transfers
    unsafe {
        // enable fifo inlet
        asclin0.rxfifocon().modify(|val| {
            val.eni().set(true)
        });
        // enable fifo outlet
        asclin0.txfifocon().modify(|val| {
            val.eno().set(true)
        });
        
        // flush rx fifo
        asclin0.rxfifocon().modify(|val| {
            val.flush().set(true)
        });

        // flush tx fifo
        asclin0.txfifocon().modify(|val| {
            val.flush().set(true)
        });
    }
}

pub fn enable_uart_module(asclin0: &Asclin0) {
    // enable module
    scu::wdt::clear_cpu_endinit_inline();
    unsafe {
        // Module Disable Request Bit: set 0 to enable
        asclin0.clc().modify_atomic(|val| val.disr().set(false));
    }

    // wait until it is enabled
    // while unsafe { asclin0.clc().read().disr().get() } {} // comment out if it blocks?

    scu::wdt::set_cpu_endinit_inline();
}

pub fn set_clock_source(asclin0: &Asclin0, source: ClockSource) {
    unsafe {
        asclin0.csr().modify(|val| val.clksel().set(source.into()));
    }

    if source == ClockSource::NoClock {
        while unsafe { asclin0.csr().read().con().get() != false } {}
    } else {
        while unsafe { asclin0.csr().read().con().get() != true } {}
    }
}

pub fn set_bit_timing(asclin0: &Asclin0, config: &NodeConfig) {
    let (d, n) = find_best_fraction(0 as f32, 0 as f32, 0 as u32); // hacked for now
    unsafe {
        asclin0
            .brg()
            .modify(|val| val.denominator().set(d as u16).numerator().set(n as u16));
    }

    unsafe {
        asclin0.bitcon().modify(|val| {
            val
                // set shift frequency
                .oversampling()
                .set(config.baud_rate.over_sampling_factor)
                // set sampling point
                .samplepoint()
                .set(config.bit_timing.sample_point_position)
                // set median filter
                .sm()
                .set(config.bit_timing.median_filter)
        });
    }
}

// pub fn get_fa_fraquency(asclin0: &Asclin0) -> f32 {
//     let clock_source = unsafe {
//         asclin0.csr().read().clksel().get()
//     };
//     match clock_source {
//         ClockSource::NoClock => 0.0,
//         ClockSource::FastClock =>
//         _ => panic!("Invalid clock source")
//     }
// }

// what does this algorithm do???
pub fn find_best_fraction(fpd: f32, baudrate: f32, oversampling: u32) -> (u32, u32) {
    let fOvs: f32 = baudrate * oversampling as f32;
    let limit: f32 = 0.001 * fOvs;
    // numerator, denominator
    let d: u32 = (fpd / fOvs) as u32;
    let n: u32 = 1;

    let d_best: u32 = d;
    let d_best: u32 = n;

    (434, 1) // hacked for now
}
