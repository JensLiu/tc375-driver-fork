use tc375_pac::asclin0::Asclin0;

use self::{
    configs::{OutputIdx, PinsConfig, Rx, RxSel, Tx},
    module::{Disabled, Module},
    node::{Configured, Node},
    ports::{InputMode, OutputMode, PadDriver, PortNumber},
};

pub mod configs;
pub mod module;
pub mod node;
pub mod ports;

static mut UART_IO_NODE: Option<Node<Asclin0, Configured>> = None;

// dirty code to get things started
pub fn init_uart_io() {
    let module = Module::<Asclin0, Disabled>::new().enable();
    let node = module.take_node(Default::default());
    node.set_pins(PinsConfig {
        rx: Some(Rx {
            port: PortNumber::_14,
            pin_index: 1,
            select: RxSel::_A,
            input_mode: InputMode::PULL_UP,
        }),
        tx: Some(Tx {
            port: PortNumber::_14,
            pin_index: 0,
            select: OutputIdx::ALT2,
            output_mode: OutputMode::PUSH_PULL,
        }),
        pad_driver: PadDriver::CmosAutomotiveSpeed1,
    });

    unsafe {
        UART_IO_NODE = Some(node.lock_configuration()).into();
    }
}

pub fn print(s: &str) {
    unsafe {
        UART_IO_NODE.as_ref().unwrap().send_blocking(s.as_bytes());
    }
}
