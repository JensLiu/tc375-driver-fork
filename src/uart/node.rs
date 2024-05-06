use core::{cmp::max, marker::PhantomData};

use tc375_pac::{asclin0::Asclin0, RegisterValue, SRC};

use crate::uart::configs::Tos;

use super::{
    configs::{
        BaudRateConfig, BitTimingConfig, ClockSource, FifoConfig, FrameConfig, FrameMode,
        InterruptConfig, NodeConfig, PinsConfig, RxSel,
    },
    ports,
};

pub struct Configurable;

pub struct Configured;

pub struct Node<NodeReg, State> {
    reg: NodeReg,
    _phantom: PhantomData<State>,
}

impl Node<Asclin0, Configurable> {
    pub fn new(config: NodeConfig) -> Self {
        // hacked for now
        let zelf = Self {
            reg: tc375_pac::ASCLIN0,
            _phantom: PhantomData,
        };

        zelf.disable_clock_source();
        zelf.set_initialisation_frame_mode();

        zelf.set_bit_timing_and_baud_rate(&config.baud_rate, &config.bit_timing);
        zelf.set_loop_back(config.loop_back);
        zelf.set_frame(&config.frame);
        zelf.set_fifo(&config.fifo);

        // skip pin mapping ...

        zelf.set_clock_source(config.clock_source);
        zelf.set_flags_default(); // TODO: make flags configurable

        // skip SW FIFO ...

        zelf.set_interrupts(&config.interrupt);
        zelf.enable_transfers();

        // kick off the transmission flag
        // without SW FIFO and not interrupt driven
        zelf.set_tfl_flag();

        zelf
    }

    pub fn set_pins(&self, pins: PinsConfig) {
        ports::port_mapping(&self.reg, &pins);
    }

    fn set_clock_source(&self, source: ClockSource) {
        unsafe {
            self.reg.csr().modify(|r| r.clksel().set(source.into()));
        }
        if source == ClockSource::NoClock {
            while unsafe { self.reg.csr().read().con().get() != false } {}
        } else {
            while unsafe { self.reg.csr().read().con().get() != true } {}
        }
    }

    fn disable_clock_source(&self) -> ClockSource {
        let old = self.get_clock_source();
        self.set_clock_source(ClockSource::NoClock);
        old
    }

    fn get_clock_source(&self) -> ClockSource {
        unsafe { self.reg.csr().read().clksel().get() }
            .try_into()
            .unwrap()
    }

    fn set_bit_timing_and_baud_rate(
        &self,
        baud_rate: &BaudRateConfig,
        bit_timing: &BitTimingConfig,
    ) {
        let over_sampling_factor = max(baud_rate.over_sampling_factor + 1, 4);
        let sample_point_position = max(bit_timing.sample_point_position, 1);

        // calculation: hacked for now ...
        let d_best = 434;
        let n_best = 1;

        // disable clock source for now
        let old_clock_source = self.disable_clock_source();

        unsafe {
            // set the Baud Rate Generation Register
            self.reg.brg().modify(|r| {
                r.denominator()
                    .set(d_best as u16)
                    .numerator()
                    .set(n_best as u16)
            });
            // set the Bit Configuration Register
            self.reg.bitcon().modify(|r| {
                r.prescaler() // set pre scaler
                    .set(baud_rate.prescaler - 1)
                    .oversampling() // set shift frequency
                    .set(over_sampling_factor - 1)
                    .samplepoint() // set sampling point
                    .set(sample_point_position)
                    .sm() // set median filter
                    .set(bit_timing.median_filter)
            });
        }

        // restore clock source
        self.set_clock_source(old_clock_source);
    }

    fn set_initialisation_frame_mode(&self) {
        unsafe {
            self.reg
                .framecon()
                .modify(|r| r.mode().set(FrameMode::Initialise.into()));
        }
    }

    fn set_loop_back(&self, set: bool) {
        unsafe {
            self.reg.iocr().modify(|r| r.lb().set(set));
        }
    }

    fn set_frame(&self, frame: &FrameConfig) {
        unsafe {
            self.reg.framecon().modify(|r| {
                r.pen()
                    .set(frame.parity_bit) // parity enable
                    .odd()
                    .set(frame.parity_type) //parity type (odd / even)
                    .stop()
                    .set(frame.stop_bit) // stop bit
                    .msb()
                    .set(frame.shift_dir) // shift direction
                    .idle() // idle delay
                    .set(frame.idle_delay)
                    .mode() // frame mode
                    .set(frame.frame_mode.into())
            });
            self.reg
                .datcon() // data length
                .modify(|r| r.datlen().set(frame.data_length))
        }
    }

    fn set_fifo(&self, fifo: &FifoConfig) {
        unsafe {
            self.reg.txfifocon().modify(|r| {
                r.inw() // tx fifo inlet width
                    .set(fifo.in_width)
                    .intlevel() // tx fifo interrupt level
                    .set(fifo.tx_fifo_interrupt_level)
                    .fm() // tx fifo interrupt mode
                    .set(fifo.tx_fifo_interrupt_mode)
            });
            self.reg.rxfifocon().modify(|r| {
                r.outw()
                    .set(fifo.out_width) // rx fifo outlet width
                    .intlevel()
                    .set(fifo.rx_fifo_interrupt_level) // rx fifo interrupt level
                    .fm() // rx fifo interrupt mode
                    .set(fifo.rx_fifo_interrupt_mode)
            });
        }
    }

    fn set_flags_default(&self) {
        unsafe {
            // disable all flags
            self.reg.flagsenable().modify(|r| r.set_raw(0x00000000));
            // clear all flags
            self.reg
                .flagsclear()
                .modify_atomic(|r| r.set_raw(0xFFFFFFFF));
            // error flag setting
            self.reg.flagsenable().modify(|r| {
                r.pee() // parity error flag enable
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
    }

    fn set_interrupts(&self, interrupt: &InterruptConfig) {
        let tos = &interrupt.type_of_service;
        assert_eq!(*tos, Tos::CPU0); // only supports CPU0 for now
        let asclin_index = 0;
        let src = SRC.asclin().asclin()[asclin_index]; // supports asclin for now
        if interrupt.rx_priority > 0 {
            unsafe {
                src.asclinxrx().modify(|r| {
                    r.srpn()
                        .set(interrupt.rx_priority)
                        .tos()
                        .set(interrupt.type_of_service.into())
                        .clrr()
                        .set(true)
                });
                self.reg.flagsenable().modify(|r| r.rfle().set(true));
                src.asclinxrx().modify(|r| r.sre().set(true));
            };
        }

        if interrupt.tx_priority > 0 {
            unsafe {
                src.asclinxtx().modify(|r| {
                    r.srpn()
                        .set(interrupt.tx_priority)
                        .tos()
                        .set(interrupt.type_of_service.into())
                        .clrr()
                        .set(true)
                });
                self.reg.flagsenable().modify(|r| r.tfle().set(true));
                src.asclinxtx().modify(|r| r.sre().set(true));
            }
        }

        if interrupt.er_priority > 0 {
            unsafe {
                src.asclinxerr().modify(|r| {
                    r.srpn()
                        .set(interrupt.er_priority)
                        .tos()
                        .set(interrupt.type_of_service.into())
                        .clrr()
                        .set(true)
                });
                self.reg.flagsenable().modify(|r| r.pee().set(true));
                src.asclinxerr().modify(|r| r.sre().set(true));
            }
        }
    }

    fn enable_transfers(&self) {
        unsafe {
            self.reg.rxfifocon().modify(|r| {
                r.eni()
                    .set(true) // enable fifo inlet
                    .flush()
                    .set(true) // flush rx fifo
            });
            // enable fifo outlet
            self.reg.txfifocon().modify(|r| {
                r.eno() // enable fifo inlet
                    .set(true)
                    .flush() // flush tx fifo
                    .set(true)
            });
        }
    }

    fn set_tfl_flag(&self) {
        unsafe {
            self.reg.flagsset().modify_atomic(|r| r.tfls().set(true));
        }
    }

    fn set_rx_input(&self, select: RxSel) {
        unsafe {
            self.reg.iocr().modify(|r| r.alti().set(select.into()));
        }
    }

    pub fn set_ports(&self, pins: PinsConfig) {
        ports::port_mapping(&self.reg, &pins)
    }

    pub fn lock_configuration(self) -> Node<Asclin0, Configured> {
        Node {
            reg: self.reg,
            _phantom: PhantomData,
        }
    }
}

impl Node<Asclin0, Configured> {
    pub fn send_blocking(&self, buf: &[u8]) {
        for byte in buf {
            while unsafe { !self.reg.flags().read().tfl().get() } {}
            unsafe {
                // clear the TFL flag
                self.reg.flagsclear().modify_atomic(|r| r.tflc().set(true));
                // write one byte to the TXDATA register
                self.reg.txdata().modify_atomic(|r| r.set_raw(*byte as u32));
            }
        }
    }
}
