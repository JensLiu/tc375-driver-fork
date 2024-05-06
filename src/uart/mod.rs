use core::cmp::max;

use tc375_pac::{asclin0::Asclin0, RegisterValue, SRC};

use crate::{scu, uart::uart_node::{ClockSource, FrameMode, Tos}};

use self::uart_node::NodeConfig;

pub mod ports;
pub mod uart_node; // TODO: move?

// dirty code to get things started
pub fn print(s: &str) {
    let asclin0 = tc375_pac::ASCLIN0;
    for ch in s.chars() {
        // wait for the TFL flag
        while unsafe { !asclin0.flags().read().tfl().get() } {}
        unsafe {
            // clear the TFL flag
            asclin0
                .flagsclear()
                .modify_atomic(|val| val.tflc().set(true));
            // write one byte to the TXDATA register
            asclin0
                .txdata()
                .modify_atomic(|val| val.set_raw(ch as u32));
        }
    }
}

// only supports P14 and ASCLIN0
pub fn init_uart(config: NodeConfig) {
    let asclin0 = tc375_pac::ASCLIN0;

    enable_uart_module(&asclin0);

    // disable clock source
    set_clock_source(&asclin0, ClockSource::NoClock);
    // set the module in initialise mode
    unsafe {
        asclin0
            .framecon()
            .modify(|val| val.mode().set(FrameMode::Initialise.into()));
    }

    // set prescaler
    unsafe {
        asclin0
            .bitcon()
            .modify(|val| val.prescaler().set(config.baud_rate.prescaler - 1));
    }

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
                .set(config.frame.parity_bit) // parity enable
                .odd()
                .set(config.frame.parity_type) //parity type (odd / even)
                .stop()
                .set(config.frame.stop_bit) // stop bit
                .msb()
                .set(config.frame.shift_dir) // shift direction
                .idle() // idle delay
                .set(config.frame.idle_delay)
                .mode() // frame mode
                .set(config.frame.frame_mode.into())
        });
        asclin0
            .datcon() // data length
            .modify(|val| val.datlen().set(config.frame.data_length))
    }

    // set fifo
    unsafe {
        asclin0.txfifocon().modify(|val| {
            val.inw() // tx fifo inlet width
                .set(config.fifo.in_width)
                .intlevel() // tx fifo interrupt level
                .set(config.fifo.tx_fifo_interrupt_level)
                .fm() // tx fifo interrupt mode
                .set(config.fifo.tx_fifo_interrupt_mode)
        });
        asclin0.rxfifocon().modify(|val| {
            val.outw()
                .set(config.fifo.out_width) // rx fifo outlet width
                .intlevel()
                .set(config.fifo.rx_fifo_interrupt_level) // rx fifo interrupt level
                .fm() // rx fifo interrupt mode
                .set(config.fifo.rx_fifo_interrupt_mode)
        });
    }

    // pin mapping
    ports::port_mapping(&asclin0, &config.pins);

    // select the clock source
    set_clock_source(&asclin0, config.clock_source);

    unsafe {
        // disable all flags
        asclin0.flagsenable().modify(|val| val.set_raw(0x00000000));
        // clear all flags
        asclin0
            .flagsclear()
            .modify_atomic(|val| val.set_raw(0xFFFFFFFF));
    }

    // error flag setting
    unsafe {
        asclin0.flagsenable().modify(|val| {
            val.pee() // parity error flag enable
                .set(true)
                .fee() // frame error flag enable
                .set(true)
                .rfoe() // rx fifo overflow flag enable
                .set(true)
                .rfue() // rx fifo underflow enable
                .set(true)
                .tfoe() // tx fifo overflow flag enable
                .set(true)
        });
    }
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
        asclin0.rxfifocon().modify(|val| {
            val.eni()
                .set(true) // enable fifo inlet
                .flush()
                .set(true) // flush rx fifo
        });
        // enable fifo outlet
        asclin0.txfifocon().modify(|val| {
            val.eno() // enable fifo inlet
                .set(true)
                .flush() // flush tx fifo
                .set(true)
        });
    }

    // start transmission flag
    unsafe {
        asclin0.flagsset().modify_atomic(|val| val.tfls().set(true));
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
    while unsafe { asclin0.clc().read().disr().get() } {}

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
    // let (d, n) = find_best_fraction(0 as f32, 0 as f32, 0 as u32); // hacked for now
    let over_sampling_factor = max(config.baud_rate.over_sampling_factor + 1, 4);
    let sample_point_position = max(config.bit_timing.sample_point_position, 1);
    // calculation.... hacked
    let d_best = 434;
    let n_best = 1;
    // disable clock source
    let original_clock_source: ClockSource = unsafe { asclin0.csr().read().clksel().get() }
        .try_into()
        .unwrap();
    set_clock_source(asclin0, ClockSource::NoClock.into());

    unsafe {
        asclin0.brg().modify(|val| {
            val.denominator()
                .set(d_best as u16)
                .numerator()
                .set(n_best as u16)
        });
    }

    unsafe {
        asclin0.bitcon().modify(|val| {
            val
                // set shift frequency
                .oversampling()
                .set(over_sampling_factor - 1)
                // set sampling point
                .samplepoint()
                .set(sample_point_position)
                // set median filter
                .sm()
                .set(config.bit_timing.median_filter)
        });
    }

    // restore clock source
    set_clock_source(asclin0, original_clock_source);
}

// what does this algorithm do???
#[allow(unused)]
pub fn find_best_fraction(fpd: f32, baudrate: f32, oversampling: u32) -> (u32, u32) {
    let fovs: f32 = baudrate * oversampling as f32;
    let limit: f32 = 0.001 * fovs;
    // numerator, denominator
    let d: u32 = (fpd / fovs) as u32;
    let n: u32 = 1;

    // algorithm...
    let d_best: u32 = d;
    let d_best: u32 = n;

    (434, 1) // hacked for now
}
