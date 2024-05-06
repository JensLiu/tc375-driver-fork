// UART Module

use core::marker::PhantomData;

use tc375_pac::{asclin0::Asclin0, ASCLIN0};

use crate::scu;

use super::{
    configs::NodeConfig,
    node::{Configurable, Node},
};

pub trait ModuleId {
    fn get_module_id() -> u32;
}

// whether the module is enabled or not
pub struct Disabled;
pub struct Enabled;


pub struct Module<Reg, State> {
    // nodes_taken: [bool; N_NODES], // supports 1 node for now
    _phantom: PhantomData<(Reg, State)>,
}

impl<Reg> Module<Reg, Disabled> {
    // crate a disabled module
    pub fn new() -> Self {
        Self {
            // nodes_taken: [false; N_NODES],
            _phantom: PhantomData,
        }
    }
}

impl Module<Asclin0, Disabled> {
    pub fn enable(self) -> Module<Asclin0, Enabled> {
        scu::wdt::clear_cpu_endinit_inline();

        // Module Disable Request Bit: set 0 to enable
        unsafe {
            ASCLIN0.clc().modify_atomic(|val| val.disr().set(false));
        }

        // wait until it is enabled
        while unsafe { ASCLIN0.clc().read().disr().get() } {}

        scu::wdt::set_cpu_endinit_inline();

        Module::<Asclin0, Enabled> {
            // nodes_taken: [false; N_NODES],
            _phantom: PhantomData,
        }
    }
}

impl Module<Asclin0, Enabled> {
    // it keeps track of its nodes' allocation using the nodes_taken array
    pub fn take_node(&self, config: NodeConfig) -> Node<Asclin0, Configurable> {
        Node::new(config)
    }
}
