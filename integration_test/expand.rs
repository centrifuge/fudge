#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use centrifuge_runtime::{Block as PBlock, RuntimeApi as PRtApi};
use fudge::{
    digest::DigestCreator,
    inherent::{
        CreateInherentDataProviders, FudgeDummyInherentRelayParachain, FudgeInherentParaParachain,
        FudgeInherentTimestamp,
    },
    ParachainBuilder, RelaychainBuilder,
};
use polkadot_core_primitives::{Block as RBlock, Header as RHeader};
use polkadot_runtime::{Runtime as RRuntime, RuntimeApi as RRtApi};
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
        PBlock,
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
const PARA_ID: u32 = 2002u32;
use fudge::primitives::{
    Chain as _hidden_Chain, ParaId as _hidden_ParaId, FudgeParaChain as _hidden_FudgeParaChain,
};
use codec::Decode as __hidden_Decode;
use sp_tracing as __hidden_tracing;
struct TestEnv {
    centrifuge: ParachainBuilder<PBlock, PRtApi, PCidp, Dp>,
    sibling: ParachainBuilder<PBlock, PRtApi, PCidp, Dp>,
    polkadot: RelaychainBuilder<RBlock, RRtApi, RRuntime, RCidp, Dp>,
}
impl TestEnv {
    pub fn new(
        polkadot: RelaychainBuilder<RBlock, RRtApi, RRuntime, RCidp, Dp>,
        centrifuge: ParachainBuilder<PBlock, PRtApi, PCidp, Dp>,
        sibling: ParachainBuilder<PBlock, PRtApi, PCidp, Dp>,
    ) -> Result<Self, ()> {
        let mut companion = Self {
            polkadot,
            centrifuge,
            sibling,
        };
        let para = _hidden_FudgeParaChain {
            id: _hidden_ParaId::from(PARA_ID),
            head: companion.centrifuge.head(),
            code: companion.centrifuge.code(),
        };
        companion
            .polkadot
            .onboard_para(para)
            .map_err(|_| ())
            .map(|_| ())?;
        let para = _hidden_FudgeParaChain {
            id: _hidden_ParaId::from(2000u32),
            head: companion.sibling.head(),
            code: companion.sibling.code(),
        };
        companion
            .polkadot
            .onboard_para(para)
            .map_err(|_| ())
            .map(|_| ())?;
        Ok(companion)
    }
    pub fn append_extrinsic(&mut self, chain: _hidden_Chain, ext: Vec<u8>) -> Result<(), ()> {
        match chain {
            _hidden_Chain::Relay => {
                self.polkadot.append_extrinsic(
                    __hidden_Decode::decode(&mut ext.as_slice()).map_err(|_| ())?,
                );
                Ok(())
            }
            _hidden_Chain::Para(id) => match id {
                PARA_ID => {
                    self.centrifuge.append_extrinsic(
                        __hidden_Decode::decode(&mut ext.as_slice()).map_err(|_| ())?,
                    );
                    Ok(())
                }
                2000u32 => {
                    self.sibling.append_extrinsic(
                        __hidden_Decode::decode(&mut ext.as_slice()).map_err(|_| ())?,
                    );
                    Ok(())
                }
                _ => return Err(()),
            },
        }
    }
    pub fn with_state<R>(&self, chain: _hidden_Chain, exec: impl FnOnce() -> R) -> Result<R, ()> {
        match chain {
            _hidden_Chain::Relay => self.polkadot.with_state(exec).map_err(|_| ()),
            _hidden_Chain::Para(id) => match id {
                PARA_ID => self.centrifuge.with_state(exec).map_err(|_| ()),
                2000u32 => self.sibling.with_state(exec).map_err(|_| ()),
                _ => Err(()),
            },
        }
    }
    pub fn with_mut_state<R>(
        &mut self,
        chain: _hidden_Chain,
        exec: impl FnOnce() -> R,
    ) -> Result<R, ()> {
        match chain {
            _hidden_Chain::Relay => self.polkadot.with_mut_state(exec).map_err(|_| ()),
            _hidden_Chain::Para(id) => match id {
                PARA_ID => self.centrifuge.with_mut_state(exec).map_err(|_| ()),
                2000u32 => self.sibling.with_mut_state(exec).map_err(|_| ()),
                _ => Err(()),
            },
        }
    }
    pub fn evolve(&mut self) -> Result<(), ()> {
        {
            let __within_span__ = {
                use ::tracing::__macro_support::Callsite as _;
                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                    use ::tracing::__macro_support::MacroCallsite;
                    static META: ::tracing::Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "polkadot - BlockBuilding :",
                            "integration_test",
                            sp_tracing::Level::TRACE,
                            Some("integration_test/src/main.rs"),
                            Some(53u32),
                            Some("integration_test"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&CALLSITE),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    MacroCallsite::new(&META)
                };
                let mut interest = ::tracing::subscriber::Interest::never();
                if sp_tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && sp_tracing::Level::TRACE <= ::tracing::level_filters::LevelFilter::current()
                    && {
                        interest = CALLSITE.interest();
                        !interest.is_never()
                    }
                    && CALLSITE.is_enabled(interest)
                {
                    let meta = CALLSITE.metadata();
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    let span = CALLSITE.disabled_span();
                    {};
                    span
                }
            };
            let __tracing_guard__ = __within_span__.enter();
            self.polkadot.build_block().map_err(|_| ()).map(|_| ())?;
            self.polkadot.import_block().map_err(|_| ()).map(|_| ())?;
        }
        {
            let __within_span__ = {
                use ::tracing::__macro_support::Callsite as _;
                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                    use ::tracing::__macro_support::MacroCallsite;
                    static META: ::tracing::Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "centrifuge - BlockBuilding :",
                            "integration_test",
                            sp_tracing::Level::TRACE,
                            Some("integration_test/src/main.rs"),
                            Some(53u32),
                            Some("integration_test"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&CALLSITE),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    MacroCallsite::new(&META)
                };
                let mut interest = ::tracing::subscriber::Interest::never();
                if sp_tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && sp_tracing::Level::TRACE <= ::tracing::level_filters::LevelFilter::current()
                    && {
                        interest = CALLSITE.interest();
                        !interest.is_never()
                    }
                    && CALLSITE.is_enabled(interest)
                {
                    let meta = CALLSITE.metadata();
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    let span = CALLSITE.disabled_span();
                    {};
                    span
                }
            };
            let __tracing_guard__ = __within_span__.enter();
            self.centrifuge.build_block().map_err(|_| ()).map(|_| ())?;
            self.centrifuge.import_block().map_err(|_| ()).map(|_| ())?;
            let __within_span__ = {
                use ::tracing::__macro_support::Callsite as _;
                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                    use ::tracing::__macro_support::MacroCallsite;
                    static META: ::tracing::Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "sibling - BlockBuilding :",
                            "integration_test",
                            sp_tracing::Level::TRACE,
                            Some("integration_test/src/main.rs"),
                            Some(53u32),
                            Some("integration_test"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&CALLSITE),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    MacroCallsite::new(&META)
                };
                let mut interest = ::tracing::subscriber::Interest::never();
                if sp_tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && sp_tracing::Level::TRACE <= ::tracing::level_filters::LevelFilter::current()
                    && {
                        interest = CALLSITE.interest();
                        !interest.is_never()
                    }
                    && CALLSITE.is_enabled(interest)
                {
                    let meta = CALLSITE.metadata();
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    let span = CALLSITE.disabled_span();
                    {};
                    span
                }
            };
            let __tracing_guard__ = __within_span__.enter();
            self.sibling.build_block().map_err(|_| ()).map(|_| ())?;
            self.sibling.import_block().map_err(|_| ()).map(|_| ())?;
        }
        {
            let __within_span__ = {
                use ::tracing::__macro_support::Callsite as _;
                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                    use ::tracing::__macro_support::MacroCallsite;
                    static META: ::tracing::Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "polkadot - BlockBuilding :",
                            "integration_test",
                            sp_tracing::Level::TRACE,
                            Some("integration_test/src/main.rs"),
                            Some(53u32),
                            Some("integration_test"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&CALLSITE),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    MacroCallsite::new(&META)
                };
                let mut interest = ::tracing::subscriber::Interest::never();
                if sp_tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && sp_tracing::Level::TRACE <= ::tracing::level_filters::LevelFilter::current()
                    && {
                        interest = CALLSITE.interest();
                        !interest.is_never()
                    }
                    && CALLSITE.is_enabled(interest)
                {
                    let meta = CALLSITE.metadata();
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    let span = CALLSITE.disabled_span();
                    {};
                    span
                }
            };
            let __tracing_guard__ = __within_span__.enter();
            self.polkadot.build_block().map_err(|_| ()).map(|_| ())?;
            self.polkadot.import_block().map_err(|_| ()).map(|_| ())?;
        }
        {
            let __within_span__ = {
                use ::tracing::__macro_support::Callsite as _;
                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                    use ::tracing::__macro_support::MacroCallsite;
                    static META: ::tracing::Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "polkadot - Onboarding(centrifuge) :",
                            "integration_test",
                            sp_tracing::Level::TRACE,
                            Some("integration_test/src/main.rs"),
                            Some(53u32),
                            Some("integration_test"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&CALLSITE),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    MacroCallsite::new(&META)
                };
                let mut interest = ::tracing::subscriber::Interest::never();
                if sp_tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && sp_tracing::Level::TRACE <= ::tracing::level_filters::LevelFilter::current()
                    && {
                        interest = CALLSITE.interest();
                        !interest.is_never()
                    }
                    && CALLSITE.is_enabled(interest)
                {
                    let meta = CALLSITE.metadata();
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    let span = CALLSITE.disabled_span();
                    {};
                    span
                }
            };
            let __tracing_guard__ = __within_span__.enter();
            let para = _hidden_FudgeParaChain {
                id: _hidden_ParaId::from(PARA_ID),
                head: self.centrifuge.head(),
                code: self.centrifuge.code(),
            };
            self.polkadot
                .onboard_para(para)
                .map_err(|_| ())
                .map(|_| ())?;
            let __within_span__ = {
                use ::tracing::__macro_support::Callsite as _;
                static CALLSITE: ::tracing::__macro_support::MacroCallsite = {
                    use ::tracing::__macro_support::MacroCallsite;
                    static META: ::tracing::Metadata<'static> = {
                        ::tracing_core::metadata::Metadata::new(
                            "polkadot - Onboarding(sibling) :",
                            "integration_test",
                            sp_tracing::Level::TRACE,
                            Some("integration_test/src/main.rs"),
                            Some(53u32),
                            Some("integration_test"),
                            ::tracing_core::field::FieldSet::new(
                                &[],
                                ::tracing_core::callsite::Identifier(&CALLSITE),
                            ),
                            ::tracing::metadata::Kind::SPAN,
                        )
                    };
                    MacroCallsite::new(&META)
                };
                let mut interest = ::tracing::subscriber::Interest::never();
                if sp_tracing::Level::TRACE <= ::tracing::level_filters::STATIC_MAX_LEVEL
                    && sp_tracing::Level::TRACE <= ::tracing::level_filters::LevelFilter::current()
                    && {
                        interest = CALLSITE.interest();
                        !interest.is_never()
                    }
                    && CALLSITE.is_enabled(interest)
                {
                    let meta = CALLSITE.metadata();
                    ::tracing::Span::new(meta, &{ meta.fields().value_set(&[]) })
                } else {
                    let span = CALLSITE.disabled_span();
                    {};
                    span
                }
            };
            let __tracing_guard__ = __within_span__.enter();
            let para = _hidden_FudgeParaChain {
                id: _hidden_ParaId::from(2000u32),
                head: self.sibling.head(),
                code: self.sibling.code(),
            };
            self.polkadot
                .onboard_para(para)
                .map_err(|_| ())
                .map(|_| ())?;
        }
        Ok(())
    }
}
