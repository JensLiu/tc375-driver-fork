// borrowed from Port: can/can_node/mod.rs
// For GPUI Port configuration. Should be extracted into a common crate 

use core::mem::transmute;

use tc375_pac::asclin0::Asclin0;
use tc375_pac::RegisterValue;

use crate::scu::wdt_call;

use super::configs::{OutputIdx, PinsConfig};

#[derive(Clone, Copy)]
pub enum PadDriver {
    CmosAutomotiveSpeed1 = 0,
    CmosAutomotiveSpeed2 = 1,
    CmosAutomotiveSpeed3 = 2,
    CmosAutomotiveSpeed4 = 3,
    TtlSpeed1 = 8,
    TtlSpeed2 = 9,
    TtlSpeed3 = 10,
    TtlSpeed4 = 11,
    Ttl3v3speed1 = 12,
    Ttl3v3speed2 = 13,
    Ttl3v3speed3 = 14,
    Ttl3v3speed4 = 15,
}

#[derive(Clone, Copy)]
pub enum PortNumber {
    _00,
    _01,
    _02,
    _10,
    _11,
    _12,
    _13,
    _14,
    _15,
    _20,
    _21,
    _22,
    _23,
    _32,
    _33,
    _34,
    _40,
}

#[allow(unused)]
#[derive(Debug, PartialEq)]
enum State {
    NotChanged = 0,
    High = 1,
    Low = 1 << 16,
    Toggled = (1 << 16) | 1,
}

pub struct Port {
    inner: crate::pac::p14::P14,
}

#[allow(unused)]
impl Port {
    fn new(port: PortNumber) -> Self {
        use crate::pac::p14::P14;
        use crate::pac::*;

        let inner: P14 = match port {
            PortNumber::_00 => unsafe { transmute(P00) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_01 => unsafe { transmute(P01) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_02 => unsafe { transmute(P02) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_10 => unsafe { transmute(P10) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_11 => unsafe { transmute(P11) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_12 => unsafe { transmute(P12) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_13 => unsafe { transmute(P13) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_14 => P14,
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_15 => unsafe { transmute(P15) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_20 => unsafe { transmute(P20) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_21 => unsafe { transmute(P21) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_22 => unsafe { transmute(P22) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_23 => unsafe { transmute(P23) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_32 => unsafe { transmute(P32) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_33 => unsafe { transmute(P33) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_34 => unsafe { transmute(P34) },
            // SAFETY: The following transmutes are safe because the underlying registers have the same layout
            PortNumber::_40 => unsafe { transmute(P40) },
        };
        Self { inner }
    }

    fn set_pin_state(&self, index: u8, action: State) {
        // SAFETY: Each bit of OMR is W0, TODO: index should be in range [0, 32)?
        unsafe {
            self.inner.omr().init(|r| {
                let v = (action as u32) << index;
                r.set_raw(v)
            })
        };
    }

    fn toogle_pin(&self, index: u8) {
        self.set_pin_state(index, State::Toggled)
    }

    fn set_pin_low(&self, index: u8) {
        self.set_pin_state(index, State::Low)
    }

    fn set_pin_mode_output(&self, pin_index: u8, mode: OutputMode, index: OutputIdx) {
        self.set_pin_mode(pin_index, (mode, index).into());
    }

    fn set_pin_mode_input(&self, pin_index: u8, mode: InputMode) {
        self.set_pin_mode(pin_index, mode.into())
    }

    fn set_pin_mode(&self, index: u8, mode: Mode) {
        // TODO: index should be in range [0, 16)
        let ioc_index = index / 4;
        let shift = (index & 0x3) * 8;

        // TODO This unsafe code could be made safe by comparing the address (usize) of the port if only self.inner.0 was public
        let is_supervisor =
            // SAFETY: The following transmute is safe because the underlying registers have the same layout
            unsafe { transmute::<_, usize>(self.inner) } == unsafe { transmute(crate::pac::P40) };

        if is_supervisor {
            // SAFETY: Bits 0:16 of PDISC are RW, TODO: index should be in range [0, 16)?
            wdt_call::call_without_cpu_endinit(|| unsafe {
                self.inner.pdisc().modify(|r| {
                    let mut v = r.get_raw();
                    v &= !(1 << index);
                    r.set_raw(v)
                })
            });
        }

        // TODO Can we do this without transmute?
        // TODO Use change_pin_mode_port_pin from gpio module instead?
        let iocr: crate::pac::Reg<crate::pac::p00::Iocr0_SPEC, crate::pac::RW> = {
            let iocr0 = self.inner.iocr0();
            // SAFETY: The following transmute is safe, IOCR0 is a 32 bit register
            let addr: *mut u32 = unsafe { transmute(iocr0) };
            // SAFETY: The following operation is safe since ioc_index is in range [0, 4) TODO: see line 918
            let addr = unsafe { addr.add(ioc_index as usize) };
            // SAFETY: The following transmute is safe because IOCR0, IOCR4, IOCR8 and IOCR12 have the same layout
            unsafe { transmute(addr) }
        };

        let v: u32 = (mode.0) << shift;
        let m: u32 = 0xFFu32 << shift;

        // SAFETY: Writing PCx (RW) for ioc_index
        unsafe {
            crate::intrinsics::load_modify_store(iocr.ptr(), v, m);
        }
    }

    fn set_pin_pad_driver(&self, index: u8, driver: PadDriver) {
        // TODO: index should be in range [0, 16)
        let pdr_index = index / 8;
        let shift = (index & 0x7) * 4;
        let pdr: crate::pac::Reg<crate::pac::p00::Pdr0_SPEC, crate::pac::RW> = {
            let pdr0 = self.inner.pdr0();
            // SAFETY: The following transmute is safe, PDR0 is a 32 bit register
            let addr: *mut u32 = unsafe { transmute(pdr0) };
            // SAFETY: The following operation is safe since pdr_index is in range [0, 1] TODO: see line 957
            let addr = unsafe { addr.add(pdr_index as usize) };
            // SAFETY: The following transmute is safe because PDR0 and PDR1 have the same layout
            unsafe { transmute(addr) }
        };

        wdt_call::call_without_cpu_endinit(|| {
            let v: u32 = (driver as u32) << shift;
            let m: u32 = 0xF << shift;
            // SAFETY: Writing PDx and PLx (RW) for pdr_index
            unsafe {
                crate::intrinsics::load_modify_store(pdr.ptr(), v, m);
            }
        });
    }
}

struct Mode(u32);
#[allow(unused)]
impl Mode {
    const INPUT_NO_PULL_DEVICE: Mode = Self(0);
    const INPUT_PULL_DOWN: Mode = Self(8);
    const INPUT_PULL_UP: Mode = Self(0x10);
    const OUTPUT_PUSH_PULL_GENERAL: Mode = Self(0x80);
    const OUTPUT_PUSH_PULL_ALT1: Mode = Self(0x88);
    const OUTPUT_PUSH_PULL_ALT2: Mode = Self(0x90);
    const OUTPUT_PUSH_PULL_ALT3: Mode = Self(0x98);
    const OUTPUT_PUSH_PULL_ALT4: Mode = Self(0xA0);
    const OUTPUT_PUSH_PULL_ALT5: Mode = Self(0xA8);
    const OUTPUT_PUSH_PULL_ALT6: Mode = Self(0xB0);
    const OUTPUT_PUSH_PULL_ALT7: Mode = Self(0xB8);
    const OUTPUT_OPEN_DRAIN_GENERAL: Mode = Self(0xC0);
    const OUTPUT_OPEN_DRAIN_ALT1: Mode = Self(0xC8);
    const OUTPUT_OPEN_DRAIN_ALT2: Mode = Self(0xD0);
    const OUTPUT_OPEN_DRAIN_ALT3: Mode = Self(0xD8);
    const OUTPUT_OPEN_DRAIN_ALT4: Mode = Self(0xE0);
    const OUTPUT_OPEN_DRAIN_ALT5: Mode = Self(0xE8);
    const OUTPUT_OPEN_DRAIN_ALT6: Mode = Self(0xF0);
    const OUTPUT_OPEN_DRAIN_ALT7: Mode = Self(0xF8);
}

impl From<InputMode> for Mode {
    fn from(value: InputMode) -> Self {
        Mode(value.0)
    }
}

impl From<(OutputMode, OutputIdx)> for Mode {
    fn from(value: (OutputMode, OutputIdx)) -> Self {
        Mode(value.0 .0 | value.1 .0)
    }
}

#[derive(Clone, Copy)]
pub struct InputMode(u32);
impl InputMode {
    pub const NO_PULL_DEVICE: Self = Self(0 << 3);
    pub const PULL_DOWN: Self = Self(1 << 3);
    pub const PULL_UP: Self = Self(2 << 3);
}

#[derive(Clone, Copy)]
pub struct OutputMode(u32);
impl OutputMode {
    pub const PUSH_PULL: OutputMode = Self(0x10 << 3);
    pub const OPEN_DRAIN: OutputMode = Self(0x18 << 3);
    pub const NONE: OutputMode = Self(0);
}

// port configuration for uart node
pub fn port_mapping(asclin0: &Asclin0, pins: &PinsConfig) {
    // rx
    if let Some(rx) = &pins.rx {
        let rx_port = Port::new(rx.port);
        rx_port.set_pin_mode_input(rx.pin_index, rx.input_mode);
        rx_port.set_pin_pad_driver(rx.pin_index, pins.pad_driver);
        // set rx input
        unsafe {
            asclin0
                .iocr()
                .modify(|val| val.alti().set(rx.select.into()));
        }
    }

    // tx
    if let Some(tx) = &pins.tx {
        let tx_port = Port::new(tx.port);
        tx_port.set_pin_mode_output(tx.pin_index, tx.output_mode, tx.select);
        tx_port.set_pin_pad_driver(tx.pin_index, pins.pad_driver);
    }
}
