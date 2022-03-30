#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
fn main() {}
use fudge::primitives::{
    Chain as _hidden_Chain, ParaId as _hidden_ParaId, FudgeParaChain as _hidden_FudgeParaChain,
};
struct TestEnv {
    centrifuge: (),
    acala: (),
    polkadot: (),
}
impl TestEnv {
    pub fn new(polkadot: (), centrifuge: (), acala: ()) -> Result<Self, ()> {
        let companion = Self {
            polkadot,
            centrifuge,
            acala,
        };
        let para = _hidden_FudgeParaChain {
            id: _hidden_ParaId::from(2001u32),
            head: companion.centrifuge.head(),
            code: companion.centrifuge.code(),
        };
        companion
            .polkadot
            .onboard_para(para)
            .map_err(|_| ())
            .map(|_| ())?;
        let para = _hidden_FudgeParaChain {
            id: _hidden_ParaId::from(2002u32),
            head: companion.acala.head(),
            code: companion.acala.code(),
        };
        companion
            .polkadot
            .onboard_para(para)
            .map_err(|_| ())
            .map(|_| ())?;
        Ok(companion)
    }
    pub fn with_state<R>(&self, chain: _hidden_Chain, exec: impl FnOnce() -> R) -> Result<(), ()> {
        match chain {
            _hidden_Chain::Relay => self.polkadot.with_state(exec),
            _hidden_Chain::Para(id) => match id {
                2001u32 => self.centrifuge.with_state(exec).map_err(|_| ()).map(|_| ()),
                2002u32 => self.acala.with_state(exec).map_err(|_| ()).map(|_| ()),
                _ => Err(()),
            },
        }
    }
    pub fn with_mut_state<R>(
        &self,
        chain: _hidden_Chain,
        exec: impl FnOnce() -> R,
    ) -> Result<(), ()> {
        match chain {
            _hidden_Chain::Relay => self.polkadot.with_mut_state(exec),
            _hidden_Chain::Para(id) => match id {
                2001u32 => self
                    .centrifuge
                    .with_mut_state(exec)
                    .map_err(|_| ())
                    .map(|_| ()),
                2002u32 => self.acala.with_mut_state(exec).map_err(|_| ()).map(|_| ()),
                _ => Err(()),
            },
        }
    }
    pub fn evolve(&mut self) -> Result<(), ()> {
        self.polkadot.build_block().map_err(|_| ()).map(|_| ())?;
        self.polkadot.import_block().map_err(|_| ()).map(|_| ())?;
        self.centrifuge.build_block().map_err(|_| ()).map(|_| ())?;
        self.centrifuge.import_block().map_err(|_| ()).map(|_| ())?;
        self.acala.build_block().map_err(|_| ()).map(|_| ())?;
        self.acala.import_block().map_err(|_| ()).map(|_| ())?;
        self.polkadot.build_block().map_err(|_| ()).map(|_| ())?;
        self.polkadot.import_block().map_err(|_| ()).map(|_| ())?;
        let para = _hidden_FudgeParaChain {
            id: _hidden_ParaId::from(2001u32),
            head: self.centrifuge.head(),
            code: self.centrifuge.code(),
        };
        self.polkadot
            .onboard_para(para)
            .map_err(|_| ())
            .map(|_| ())?;
        let para = _hidden_FudgeParaChain {
            id: _hidden_ParaId::from(2002u32),
            head: self.acala.head(),
            code: self.acala.code(),
        };
        self.polkadot
            .onboard_para(para)
            .map_err(|_| ())
            .map(|_| ())?;
    }
}
