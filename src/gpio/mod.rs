//! General Purpose Input / Output
//!
//! The GPIO pins are organised into groups of 16 pins which can be accessed through the
//! `gpioa`, `gpiob`... modules. To get access to the pins, you first need to convert them into a
//! HAL designed struct from the `pac` struct using the [split](trait.GpioExt.html#tymethod.split) function.
//! ```rust
//! use tc37x_pac::PORT_00;
//! use tc37x_hal::gpio::GpioExt;
//! let mut gpio00 = PORT_00.split();
//! ```
//!
//! This gives you a struct containing all the pins `px0..px15`.
//! By default pins are in floating input mode. You can change their modes.
//! For example, to set `pa5` high, you would call
//!
//! ```rust
//! use tc37x_pac::PORT_00;
//! use tc37x_hal::gpio::GpioExt;
//! let mut gpio00 = PORT_00.split();
//! let mut output = gpio00.p00_5.into_push_pull_output();
//! output.set_high();
//! ```
//!
//! ## Modes
//!
//! Each GPIO pin can be set to various modes:
//!
//! - **Alternate**: Pin mode required when the pin is driven by other peripherals
//! - **Analog**: Analog input to be used with ADC.
//! - **Dynamic**: Pin mode is selected at runtime. See changing configurations for more details
//! - Input
//!     - **PullUp**: Input connected to high with a weak pull up resistor. Will be high when nothing
//!     is connected
//!     - **PullDown**: Input connected to high with a weak pull up resistor. Will be low when nothing
//!     is connected
//!     - **Floating**: Input not pulled to high or low. Will be undefined when nothing is connected
//! - Output
//!     - **PushPull**: Output which either drives the pin high or low
//!     - **OpenDrain**: Output which leaves the gate floating, or pulls it do ground in drain
//!     mode. Can be used as an input in the `open` configuration
//!
//! ## Changing modes
//! The simplest way to change the pin mode is to use the `into_<mode>` functions. These return a
//! new struct with the correct mode that you can use the input or output functions on.
//!
//! If you need a more temporary mode change, and can not use the `into_<mode>` functions for
//! ownership reasons, you can use the closure based `with_<mode>` functions to temporarily change the pin type, do
//! some output or input, and then have it change back once done.
//!
//! ### Dynamic Mode Change
//! The above mode change methods guarantee that you can only call input functions when the pin is
//! in input mode, and output when in output modes, but can lead to some issues. Therefore, there
//! is also a mode where the state is kept track of at runtime, allowing you to change the mode
//! often, and without problems with ownership, or references, at the cost of some performance and
//! the risk of runtime errors.
//!
//! To make a pin dynamic, use the `into_dynamic` function, and then use the `make_<mode>` functions to
//! change the mode

use core::fmt;
use core::marker::PhantomData;

pub use embedded_hal::digital::v2::PinState;
use tc37x_pac::RegisterValue;

pub use convert::PinMode;
pub use dynamic::{Dynamic, DynamicPin};
pub use erased::{EPin, ErasedPin};
// TODO (alepez) Added because it was previously imported by use f4::*
pub use Input as DefaultMode;
pub use partially_erased::{PartiallyErasedPin, PEPin};

pub mod alt;
mod convert;

mod partially_erased;

mod erased;

// TODO (alepez) mod exti;
// TODO (alepez) pub use exti::ExtiPin;
mod dynamic;

mod hal_02;
mod hal_1;
pub mod outport;

/// A filler pin type
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NoPin<Otype = PushPull>(PhantomData<Otype>);

impl<Otype> NoPin<Otype> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self) -> Self::Parts;
}

/// Id, port and mode for any pin
pub trait PinExt {
    /// Current pin mode
    type Mode;
    /// Pin number
    fn pin_id(&self) -> u8;
    /// Port number starting from 0
    fn port_id(&self) -> u8;
}

/// Some alternate mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Alternate<const A: u8, Otype = PushPull>(PhantomData<Otype>);

/// Input mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Input;

/// Pull setting for an input.
#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Pull {
    /// Floating
    None = 0,
    /// Pulled up
    Up = 1,
    /// Pulled down
    Down = 2,
}

/// Open drain input or output (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OpenDrain;

/// Output mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Output<MODE = PushPull> {
    _mode: PhantomData<MODE>,
}

/// Push pull output (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PushPull;

/// Analog mode (type state)
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Analog;

/// JTAG/SWD mote (type state)
pub type Debugger = Alternate<0, PushPull>;

pub(crate) mod marker {
    /// Marker trait that show if `ExtiPin` can be implemented
    pub trait Interruptible {}

    /// Marker trait for readable pin modes
    pub trait Readable {}

    /// Marker trait for slew rate configurable pin modes
    pub trait OutputSpeed {}

    /// Marker trait for active pin modes
    pub trait Active {}

    /// Marker trait for all pin modes except alternate
    pub trait NotAlt {}

    /// Marker trait for pins with alternate function `A` mapping
    pub trait IntoAf<const A: u8> {}
}

impl<MODE> marker::Interruptible for Output<MODE> {}

impl marker::Interruptible for Input {}

impl marker::Readable for Input {}

impl marker::Readable for Output<OpenDrain> {}

impl<const A: u8, Otype> marker::Interruptible for Alternate<A, Otype> {}

impl<const A: u8, Otype> marker::Readable for Alternate<A, Otype> {}

impl marker::Active for Input {}

impl<Otype> marker::OutputSpeed for Output<Otype> {}

impl<const A: u8, Otype> marker::OutputSpeed for Alternate<A, Otype> {}

impl<Otype> marker::Active for Output<Otype> {}

impl<const A: u8, Otype> marker::Active for Alternate<A, Otype> {}

impl marker::NotAlt for Input {}

impl<Otype> marker::NotAlt for Output<Otype> {}

impl marker::NotAlt for Analog {}

/// GPIO Pin speed selection
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Speed {
    /// Low speed
    Low = 0,
    /// Medium speed
    Medium = 1,
    /// High speed
    High = 2,
    /// Very high speed
    VeryHigh = 3,
}

/// GPIO interrupt trigger edge selection
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Edge {
    /// Rising edge of voltage
    Rising,
    /// Falling edge of voltage
    Falling,
    /// Rising and falling edge of voltage
    RisingFalling,
}

macro_rules! af {
    ($($i:literal: $AFi:ident),+) => {
        $(
            #[doc = concat!("Alternate function ", $i, " (type state)" )]
            pub type $AFi<Otype = PushPull> = Alternate<$i, Otype>;
        )+
    };
}

af!(
    0: AF0,
    1: AF1,
    2: AF2,
    3: AF3,
    4: AF4,
    5: AF5,
    6: AF6,
    7: AF7,
    8: AF8,
    9: AF9,
    10: AF10,
    11: AF11,
    12: AF12,
    13: AF13,
    14: AF14,
    15: AF15
);

/// Generic pin type
///
/// - `MODE` is one of the pin modes (see [Modes](crate::gpio#modes) section).
/// - `P` is port id
/// - `N` is pin number: from `0` to `15`.
pub struct Pin<const P: usize, const N: u8, MODE = DefaultMode> {
    _mode: PhantomData<MODE>,
}

impl<const P: usize, const N: u8, MODE> Pin<P, N, MODE> {
    const fn new() -> Self {
        Self { _mode: PhantomData }
    }
}

impl<const P: usize, const N: u8, MODE> fmt::Debug for Pin<P, N, MODE> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!(
            "P{}{}<{}>",
            P,
            N,
            crate::stripped_type_name::<MODE>()
        ))
    }
}

#[cfg(feature = "defmt")]
impl<const P: usize, const N: u8, MODE> defmt::Format for Pin<P, N, MODE> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "P{}{}<{}>", P, N, crate::stripped_type_name::<MODE>());
    }
}

impl<const P: usize, const N: u8, MODE> PinExt for Pin<P, N, MODE> {
    type Mode = MODE;

    #[inline(always)]
    fn pin_id(&self) -> u8 {
        N
    }
    #[inline(always)]
    fn port_id(&self) -> u8 {
        P as u8 - b'A'
    }
}

pub trait PinSpeed: Sized {
    /// Set pin speed
    fn set_speed(&mut self, speed: Speed);

    #[inline(always)]
    fn speed(mut self, speed: Speed) -> Self {
        self.set_speed(speed);
        self
    }
}

pub trait PinPull: Sized {
    /// Set the internal pull-up and pull-down resistor
    fn set_internal_resistor(&mut self, resistor: Pull);

    #[inline(always)]
    fn internal_resistor(mut self, resistor: Pull) -> Self {
        self.set_internal_resistor(resistor);
        self
    }
}

impl<const P: usize, const N: u8, MODE> PinSpeed for Pin<P, N, MODE>
    where
        MODE: marker::OutputSpeed,
{
    #[inline(always)]
    fn set_speed(&mut self, speed: Speed) {
        self.set_speed(speed)
    }
}

impl<const P: usize, const N: u8, MODE> Pin<P, N, MODE>
    where
        MODE: marker::OutputSpeed,
{
    /// Set pin speed
    pub fn set_speed(&mut self, speed: Speed) {
        let offset = 2 * { N };

        // TODO (alepez)
        // unsafe {
        //     (*Gpio::<P>::ptr())
        //         .ospeedr
        //         .modify(|r, w| w.bits((r.bits() & !(0b11 << offset)) | ((speed as u32) << offset)));
        // }
    }

    /// Set pin speed
    pub fn speed(mut self, speed: Speed) -> Self {
        self.set_speed(speed);
        self
    }
}

impl<const P: usize, const N: u8, MODE> PinPull for Pin<P, N, MODE>
    where
        MODE: marker::Active,
{
    #[inline(always)]
    fn set_internal_resistor(&mut self, resistor: Pull) {
        self.set_internal_resistor(resistor)
    }
}

impl<const P: usize, const N: u8, MODE> Pin<P, N, MODE>
    where
        MODE: marker::Active,
{
    /// Set the internal pull-up and pull-down resistor
    pub fn set_internal_resistor(&mut self, resistor: Pull) {
        let offset = 2 * { N };
        let value = resistor as u32;
        // TODO (alepez)
        // unsafe {
        //     (*Gpio::<P>::ptr())
        //         .pupdr
        //         .modify(|r, w| w.bits((r.bits() & !(0b11 << offset)) | (value << offset)));
        // }
    }

    /// Set the internal pull-up and pull-down resistor
    pub fn internal_resistor(mut self, resistor: Pull) -> Self {
        self.set_internal_resistor(resistor);
        self
    }

    /// Enables / disables the internal pull up
    pub fn internal_pull_up(self, on: bool) -> Self {
        if on {
            self.internal_resistor(Pull::Up)
        } else {
            self.internal_resistor(Pull::None)
        }
    }

    /// Enables / disables the internal pull down
    pub fn internal_pull_down(self, on: bool) -> Self {
        if on {
            self.internal_resistor(Pull::Down)
        } else {
            self.internal_resistor(Pull::None)
        }
    }
}

impl<const P: usize, const N: u8, MODE> Pin<P, N, MODE> {
    /// Erases the pin number from the type
    ///
    /// This is useful when you want to collect the pins into an array where you
    /// need all the elements to have the same type
    pub fn erase_number(self) -> PartiallyErasedPin<P, MODE> {
        PartiallyErasedPin::new(N)
    }

    /// Erases the pin number and the port from the type
    ///
    /// This is useful when you want to collect the pins into an array where you
    /// need all the elements to have the same type
    pub fn erase(self) -> ErasedPin<MODE> {
        ErasedPin::new(P as u8 - b'A', N)
    }
}

impl<const P: usize, const N: u8, MODE> From<Pin<P, N, MODE>> for PartiallyErasedPin<P, MODE> {
    /// Pin-to-partially erased pin conversion using the [`From`] trait.
    ///
    /// Note that [`From`] is the reciprocal of [`Into`].
    fn from(p: Pin<P, N, MODE>) -> Self {
        p.erase_number()
    }
}

impl<const P: usize, const N: u8, MODE> From<Pin<P, N, MODE>> for ErasedPin<MODE> {
    /// Pin-to-erased pin conversion using the [`From`] trait.
    ///
    /// Note that [`From`] is the reciprocal of [`Into`].
    fn from(p: Pin<P, N, MODE>) -> Self {
        p.erase()
    }
}

impl<const P: usize, const N: u8, MODE> Pin<P, N, MODE> {
    /// Set the output of the pin regardless of its mode.
    /// Primarily used to set the output value of the pin
    /// before changing its mode to an output to avoid
    /// a short spike of an incorrect value
    #[inline(always)]
    fn _set_state(&mut self, state: PinState) {
        let state: u32 = match state {
            PinState::High => 1,
            PinState::Low => 1 << 16,
        };
        unsafe {
            (*Gpio::<P>::ptr())
                .omr()
                .init(|mut r| r.set_raw((state << N).into()));
        };
    }
    #[inline(always)]
    fn _set_high(&mut self) {
        self._set_state(PinState::High)
    }
    #[inline(always)]
    fn _set_low(&mut self) {
        self._set_state(PinState::Low)
    }
    #[inline(always)]
    fn _is_set_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        // TODO (alepez) unsafe { (*Gpio::<P>::ptr()).odr.read().bits() & (1 << N) == 0 }
        todo!()
    }
    #[inline(always)]
    fn _is_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        // TODO (alepez) unsafe { (*Gpio::<P>::ptr()).idr.read().bits() & (1 << N) == 0 }
        todo!()
    }
}

impl<const P: usize, const N: u8, MODE> Pin<P, N, Output<MODE>> {
    /// Drives the pin high
    #[inline(always)]
    pub fn set_high(&mut self) {
        self._set_high()
    }

    /// Drives the pin low
    #[inline(always)]
    pub fn set_low(&mut self) {
        self._set_low()
    }

    /// Is the pin in drive high or low mode?
    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        if self.is_set_low() {
            PinState::Low
        } else {
            PinState::High
        }
    }

    /// Drives the pin high or low depending on the provided value
    #[inline(always)]
    pub fn set_state(&mut self, state: PinState) {
        match state {
            PinState::Low => self.set_low(),
            PinState::High => self.set_high(),
        }
    }

    /// Is the pin in drive high mode?
    #[inline(always)]
    pub fn is_set_high(&self) -> bool {
        !self.is_set_low()
    }

    /// Is the pin in drive low mode?
    #[inline(always)]
    pub fn is_set_low(&self) -> bool {
        self._is_set_low()
    }

    /// Toggle pin output
    #[inline(always)]
    pub fn toggle(&mut self) {
        if self.is_set_low() {
            self.set_high()
        } else {
            self.set_low()
        }
    }
}

pub trait ReadPin {
    #[inline(always)]
    fn is_high(&self) -> bool {
        !self.is_low()
    }
    fn is_low(&self) -> bool;
}

impl<const P: usize, const N: u8, MODE> ReadPin for Pin<P, N, MODE>
    where
        MODE: marker::Readable,
{
    #[inline(always)]
    fn is_low(&self) -> bool {
        self.is_low()
    }
}

impl<const P: usize, const N: u8, MODE> Pin<P, N, MODE>
    where
        MODE: marker::Readable,
{
    /// Is the input pin high?
    #[inline(always)]
    pub fn is_high(&self) -> bool {
        !self.is_low()
    }

    /// Is the input pin low?
    #[inline(always)]
    pub fn is_low(&self) -> bool {
        self._is_low()
    }
}

macro_rules! gpio {
    ($gpiox:ident, $PORTX:ty, $port_id:expr, $PXn:ident, [
        $($PXi:ident: ($pxi:ident, $i:expr, [$($A:literal),*] $(, $MODE:ty)?),)+
    ]) => {
        /// GPIO
        pub mod $gpiox {
            /// GPIO parts
            pub struct Parts {
                $(
                    /// Pin
                    pub $pxi: $PXi $(<$MODE>)?,
                )+
            }

            impl super::GpioExt for $PORTX {
                type Parts = Parts;

                fn split(self) -> Parts {
                    unsafe {
                        // Enable clock.
                        // TODO (alepez) $PORTX::enable_unchecked();
                        // TODO (alepez) $PORTX::reset_unchecked();
                    }
                    Parts {
                        $(
                            $pxi: $PXi::new(),
                        )+
                    }
                }
            }

            #[doc="Common type for "]
            #[doc=stringify!($PORTX)]
            #[doc=" related pins"]
            pub type $PXn<MODE> = super::PartiallyErasedPin<$port_id, MODE>;

            $(
                #[doc=stringify!($PXi)]
                #[doc=" pin"]
                pub type $PXi<MODE = super::DefaultMode> = super::Pin<$port_id, $i, MODE>;

                $(
                    impl<MODE> super::marker::IntoAf<$A> for $PXi<MODE> { }
                )*
            )+

        }

        pub use $gpiox::{ $($PXi,)+ };
    }
}
use gpio;

mod tc37x;
pub use self::tc37x::*;

// TODO (alepez) Was crate::pac::gpioa::RegisterBlock in stm32f4xx-hal
type RegisterBlock = crate::pac::port_00::Port00;

struct Gpio<const P: usize>;

impl<const P: usize> Gpio<P> {
    const fn ptr() -> *const crate::pac::port_00::Port00 {
        // TODO (alepez) add ports
        // TODO (alepez) check if the assumptions are correct
        // The logic relies on the following assumptions:
        // - PORT_00 register are available on all chips
        // - all PORT register blocks have the same layout
        unsafe {
            match P {
                0 => core::mem::transmute(&crate::pac::PORT_00),
                1 => core::mem::transmute(&crate::pac::PORT_01),
                _ => panic!("Unknown GPIO port"),
            }
        }
    }
}
