// Copyright 2021 Centrifuge Foundation (centrifuge.io).
//
// This file is part of the FUDGE project.
// FUDGE is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).
// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

//! FUDGE - FUlly Decoupled Generic Environment for Substrate-based Chains
//!
//! Generally only this dependency is needed in order to use FUDGE.
//! Developers who want to use the more raw apis and types are
//! referred to the fudge-core repository.

pub use fudge_companion::companion;
pub use fudge_core::{
	builder::relay_chain::InherentBuilder,
	digest, inherent,
	provider::{
		backend, initiator, state, BackendProvider, ClientProvider, DefaultClient, Initiator,
		TWasmExecutor,
	},
};
use sc_service::{TFullBackend, TFullClient};

pub mod primitives {
	pub use fudge_core::builder::{
		parachain::{FudgeParaBuild, FudgeParaChain},
		PoolState,
	};
	pub use polkadot_parachain_primitives::primitives::Id as ParaId;

	#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash)]
	pub enum Chain {
		Relay,
		Para(u32),
	}
}

pub mod builder {
	pub use fudge_core::builder::{
		parachain::ParachainBuilder, relay_chain::RelaychainBuilder, stand_alone::StandAloneBuilder,
	};

	pub mod core {
		pub use fudge_core::builder::core::{Builder, Operation};
	}
}

pub type ParachainBuilder<Block, RtApi, CIDP, DP> =
	fudge_core::builder::parachain::ParachainBuilder<
		Block,
		RtApi,
		TWasmExecutor,
		CIDP,
		(),
		DP,
		TFullBackend<Block>,
		TFullClient<Block, RtApi, TWasmExecutor>,
	>;

pub type RelaychainBuilder<Block, RtApi, Runtime, CIDP, DP> =
	fudge_core::builder::relay_chain::RelaychainBuilder<
		Block,
		RtApi,
		TWasmExecutor,
		CIDP,
		(),
		DP,
		Runtime,
		TFullBackend<Block>,
		TFullClient<Block, RtApi, TWasmExecutor>,
	>;

pub type StandaloneBuilder<Block, RtApi, CIDP, DP> =
	fudge_core::builder::stand_alone::StandAloneBuilder<
		Block,
		RtApi,
		TWasmExecutor,
		CIDP,
		(),
		DP,
		TFullBackend<Block>,
		TFullClient<Block, RtApi, TWasmExecutor>,
	>;
