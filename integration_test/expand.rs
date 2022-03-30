#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use centrifuge_runtime::{
    Block as PBlock, Runtime as PRuntime, RuntimeApi as PRtApi, WASM_BINARY as PCODE,
};
use fudge::{
    digest::DigestCreator,
    inherent::{
        CreateInherentDataProviders, FudgeDummyInherentRelayParachain, FudgeInherentParaParachain,
        FudgeInherentTimestamp,
    },
    EnvProvider, ParachainBuilder, RelaychainBuilder,
};
use polkadot_core_primitives::{Block as RBlock, Header as RHeader};
use polkadot_runtime::{Runtime as RRuntime, RuntimeApi as RRtApi, WASM_BINARY as RCODE};
type RCidp = Box<
    dyn CreateInherentDataProviders<
        RBlock,
        (),
        InherentDataProviders = (
            FudgeInherentTimestamp,
            sp_consensus_babe::inherents::InherentDataProvider,
            sp_authorship::InherentDataProvider<RHeader>,
            FudgeDummyInherentRelayParachain<RHeader>,
        ),
    >,
>;
type PCidp = Box<
    dyn CreateInherentDataProviders<
        RBlock,
        (),
        InherentDataProviders = (
            FudgeInherentTimestamp,
            sp_consensus_babe::inherents::InherentDataProvider,
            FudgeInherentParaParachain,
        ),
    >,
>;
type Dp = Box<dyn DigestCreator + Send + Sync>;
fn main() {}
use fudge::primitives::{
    Chain as _hidden_Chain, ParaId as _hidden_ParaId, FudgeParaChain as _hidden_FudgeParaChain,
};
struct TestEnv {
    centrifuge: ParachainBuilder<
        PBlock,
        PRtApi,
        Box<
            dyn CreateInherentDataProviders<
                RBlock,
                (),
                InherentDataProviders = (
                    FudgeInherentTimestamp,
                    sp_consensus_babe::inherents::InherentDataProvider,
                    sp_authorship::InherentDataProvider<RHeader>,
                    FudgeDummyInherentRelayParachain<RHeader>,
                ),
            >,
        >,
        Dp,
    >,
    polkadot: RelaychainBuilder<RBlock, RRtApi, RRuntime, RCidp, Dp>,
}
impl TestEnv {
    pub fn new(
        polkadot: RelaychainBuilder<RBlock, RRtApi, RRuntime, RCidp, Dp>,
        centrifuge: ParachainBuilder<
            PBlock,
            PRtApi,
            Box<
                dyn CreateInherentDataProviders<
                    RBlock,
                    (),
                    InherentDataProviders = (
                        FudgeInherentTimestamp,
                        sp_consensus_babe::inherents::InherentDataProvider,
                        sp_authorship::InherentDataProvider<RHeader>,
                        FudgeDummyInherentRelayParachain<RHeader>,
                    ),
                >,
            >,
            Dp,
        >,
    ) -> Result<Self, ()> {
        let companion = Self {
            polkadot,
            centrifuge,
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
        Ok(companion)
    }
    pub fn with_state<R>(&self, chain: _hidden_Chain, exec: impl FnOnce() -> R) -> Result<(), ()> {
        match chain {
            _hidden_Chain::Relay => self.polkadot.with_state(exec),
            _hidden_Chain::Para(id) => match id {
                2001u32 => self.centrifuge.with_state(exec).map_err(|_| ()).map(|_| ()),
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
                _ => Err(()),
            },
        }
    }
    pub fn evolve(&mut self) -> Result<(), ()> {
        self.polkadot.build_block().map_err(|_| ()).map(|_| ())?;
        self.polkadot.import_block().map_err(|_| ()).map(|_| ())?;
        self.centrifuge.build_block().map_err(|_| ()).map(|_| ())?;
        self.centrifuge.import_block().map_err(|_| ()).map(|_| ())?;
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
    }
}
