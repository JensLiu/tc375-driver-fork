use super::ports::{InputMode, OutputMode, PadDriver, PortNumber};

// a reimplementation of UART configuration in the iLLD library
#[derive(Default)]
pub struct NodeConfig {
    pub clock_source: ClockSource,
    pub baud_rate: BaudRateConfig,
    pub bit_timing: BitTimingConfig,
    pub frame: FrameConfig,
    pub fifo: FifoConfig,
    pub interrupt: InterruptConfig,
    pub error_flags: u8, // hacked for now
    pub look_back: bool,
    pub pins: PinsConfig,
}

// baud rate --------------------------------------------------------------
pub struct BaudRateConfig {
    // value of the required baudrate
    pub baud_rate: f32,
    // BITCON.PRESCALER, the division ratio of the predevider
    pub prescaler: u16,
    // BITCON.OVERSAMPLING, division ratio of the baudrate post devider
    pub over_sampling_factor: u8,
}

impl Default for BaudRateConfig {
    fn default() -> Self {
        Self {
            prescaler: 1,
            baud_rate: 115200f32,
            over_sampling_factor: 3, // factor_4: 3
        }
    }
}

// bit timing -------------------------------------------------------------
pub struct BitTimingConfig {
    // BITCON.SM, number of samples per bit (1 or 3), sample mode/median filter
    pub median_filter: bool,
    // BITCON.SAMPLEPOINT, sample point position
    pub sample_point_position: u8,
}

impl Default for BitTimingConfig {
    fn default() -> Self {
        Self {
            median_filter: false,     // one sample per bit
            sample_point_position: 3, // sample point position at 3
        }
    }
}

// frame -------------------------------------------------------------------
pub struct FrameConfig {
    pub idle_delay: u8,        // FRAMECON.IDLE, idle delay
    pub stop_bit: u8,          // FRAMECON.STOP, number of stop bits
    pub frame_mode: FrameMode, // FRAMECON.MODE, mode of operation of the module
    pub shift_dir: bool,       // FRAMECON.MSB, shift direction
    pub parity_type: bool,     // FRAMECON.ODD, parity type (even or odd)
    pub data_length: u8, // DATCON.DATALENGTH, data length, number of bits per transfer (bytes)
    pub parity_bit: bool, // FRAMECON.PEN, parity enable
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            idle_delay: 0,              // no idle delay
            stop_bit: 1,                // one stop bit
            frame_mode: FrameMode::Asc, // Asc Mode
            shift_dir: false,           // shift diection LSB first
            parity_type: false,         // even parity
            data_length: 7,             // number of bits per transfer 8 (7, 0-indexed)
            parity_bit: false,          // disable parity
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum FrameMode {
    Initialise,
    #[default]
    Asc,
    Spi,
    Lin,
}

impl From<FrameMode> for u8 {
    fn from(x: FrameMode) -> Self {
        match x {
            FrameMode::Initialise => 0,
            FrameMode::Asc => 1,
            FrameMode::Spi => 2,
            FrameMode::Lin => 3,
        }
    }
}

// clock source ---------------------------------------------------------------------
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ClockSource {
    NoClock,
    #[default]
    FastClock,
    SlowClock,
}

impl From<ClockSource> for u8 {
    fn from(x: ClockSource) -> Self {
        match x {
            ClockSource::NoClock => 0,
            ClockSource::FastClock => 2,
            ClockSource::SlowClock => 4,
        }
    }
}

impl TryInto<ClockSource> for u8 {
    type Error = ();
    fn try_into(self) -> Result<ClockSource, Self::Error> {
        match self {
            0 => Ok(ClockSource::NoClock),
            2 => Ok(ClockSource::FastClock),
            4 => Ok(ClockSource::SlowClock),
            _ => Err(()),
        }
    }
}

// FIFO Control --------------------------------------------------------------
pub struct FifoConfig {
    pub in_width: u8,                // TXFIFOCON.INW, transmit FIFO inlet width */
    pub out_width: u8,               // RXFIFOCON.OTW, receive FIFO oulet width */
    pub tx_fifo_interrupt_level: u8, // TXFIFOCON.INTLEVEL, Tx FIFO interrupt level */
    pub rx_fifo_interrupt_level: u8, // RXFIFOCON.INTLEVEL, Rx FIFO interrupt level */
    pub buff_mode: u32, // RXFIFOCON.BUFF, receive buffer mode (Rx FIFO or Rx buffer) */
    pub tx_fifo_interrupt_mode: u8, // TXFIFOCON.FM, Tx FIFO interrupt generation mode */
    pub rx_fifo_interrupt_mode: u8, // RXFIFOCON.FM, Rx FIFO interrupt generation mode */
}

impl Default for FifoConfig {
    fn default() -> Self {
        Self {
            in_width: 1,                // 1-byte (8-bit) write
            out_width: 1,               // 1-byte read
            tx_fifo_interrupt_level: 0, // txFifoInterruptLevel = 0. optimised to write upto 16 bytes at a time
            rx_fifo_interrupt_level: 0, // rxFifoInterruptLevel = 1. (1-indexed)
            buff_mode: 0,               // Rx FIFO Mode (1: Rx Buffer Mode)
            tx_fifo_interrupt_mode: 0,  // combined move mode
            rx_fifo_interrupt_mode: 0,  // combined move mode
        }
    }
}

// Interrupt Control -----------------------------------------------------------
pub struct InterruptConfig {
    pub tx_priority: u8,      // brief transmit interrupt priority */
    pub rx_priority: u8,      // brief receive interrupt priority */
    pub er_priority: u8,      // brief error interrupt priority */
    pub type_of_service: Tos, // brief type of interrupt service */
}

impl Default for InterruptConfig {
    fn default() -> Self {
        Self {
            tx_priority: 0,             // transmit interrupt priority 0
            rx_priority: 0,             // receive interrupt priority 0
            er_priority: 0,             // error interrupt priority 0
            type_of_service: Tos::CPU0, // CPU0 as the default TOS
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tos {
    CPU0,
    DMA,
    CPU1,
    CPU2,
}

impl From<Tos> for u8 {
    fn from(x: Tos) -> Self {
        match x {
            Tos::CPU0 => 0,
            Tos::DMA => 1,
            Tos::CPU1 => 2,
            Tos::CPU2 => 3,
        }
    }
}

#[derive(Clone, Copy)]
pub enum RxSel {
    _A,
    _B,
    _C,
    _D,
    _E,
    _F,
    _G,
    _H,
}

impl From<RxSel> for u8 {
    fn from(value: RxSel) -> Self {
        match value {
            RxSel::_A => 0,
            RxSel::_B => 1,
            RxSel::_C => 2,
            RxSel::_D => 3,
            RxSel::_E => 4,
            RxSel::_F => 5,
            RxSel::_G => 6,
            RxSel::_H => 7,
        }
    }
}

#[derive(Clone, Copy)]
pub struct OutputIdx(pub u32);
impl OutputIdx {
    pub const GENERAL: Self = Self(0x10 << 3);
    pub const ALT1: Self = Self(0x11 << 3);
    pub const ALT2: Self = Self(0x12 << 3);
    pub const ALT3: Self = Self(0x13 << 3);
    pub const ALT4: Self = Self(0x14 << 3);
    pub const ALT5: Self = Self(0x15 << 3);
    pub const ALT6: Self = Self(0x16 << 3);
    pub const ALT7: Self = Self(0x17 << 3);
}

// Pins
pub struct PinsConfig {
    pub rx: Option<Rx>,
    pub tx: Option<Tx>,
    pub pad_driver: PadDriver,
}

impl Default for PinsConfig {
    fn default() -> Self {
        Self {
            rx: None,
            tx: None,
            pad_driver: PadDriver::CmosAutomotiveSpeed1,
        }
    }
}

pub struct Rx {
    pub port: PortNumber,
    pub pin_index: u8,
    pub select: RxSel,
    pub input_mode: InputMode,
}

pub struct Tx {
    pub port: PortNumber,
    pub pin_index: u8,
    pub select: OutputIdx,
    pub output_mode: OutputMode,
}
