// for software interrupts
use tc375_pac::{
    src::gpsr::{
        gpsr_gpsr::{GpsRxy, GpsRxy_SPEC},
        GpsrGpsr,
    },
    Reg, RW,
};

#[derive(Clone, Copy, Debug)]
pub struct SoftwareInterruptNode {
    group_nr: usize,
    req_nr: usize,
    prio: u8,
    tos: u8,
}

impl SoftwareInterruptNode {
    pub fn new(group_nr: usize, req_nr: usize, prio: u8, tos: u8) -> Self {
       Self {
            group_nr,
            req_nr,
            prio,
            tos,
        }
    }

    fn modify_reg(&self, f: impl FnOnce(GpsRxy) -> GpsRxy) {
        let src = tc375_pac::SRC;
        let groups = src.gpsr().gpsr();
        let group: GpsrGpsr = groups[self.group_nr];
        let reqs = group.gpsrxy();
        let req: Reg<GpsRxy_SPEC, RW> = reqs[self.req_nr];
        unsafe {
            req.modify(|r: GpsRxy| f(r));
        }
    }

    pub fn init(&self) {
        self.modify_reg(|r: GpsRxy| {
            r.srpn() // priority number
                .set(self.prio)
                .tos() // type of service
                .set(self.tos)
        })
    }

    pub fn set_request(&self) {
        self.modify_reg(|r: GpsRxy| r.setr().set(true));
    }

    pub fn clear_request(&self) {
        self.modify_reg(|r: GpsRxy| r.clrr().set(true));
    }

    pub fn enable(&self) {
        self.modify_reg(|r: GpsRxy| r.sre().set(true));
    }

    pub fn disable(&self) {
        self.modify_reg(|r: GpsRxy| r.sre().set(false));
    }

    pub fn get_prio(&self) -> u8 {
        self.prio
    }
}
