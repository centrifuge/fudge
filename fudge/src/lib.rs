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

pub use fudge_companion::companion;
use fudge_core::provider::TWasmExecutor;
pub use fudge_core::{
	digest, inherent,
	provider::{
		backend::{DiskDatabaseType, DiskDb, MemDb},
		initiator::{FromConfiguration, Init, PoolConfig},
		state::StateProvider,
		BackendProvider, ClientProvider, DefaultClient, Initiator,
	},
};
///! FUDGE - FUlly Decoupled Generic Environment for Substrate-based Chains
///!
///! Generally only this dependency is needed in order to use FUDGE.
///! Developers who want to use the more raw apis and types are
///! referred to the fudge-core repository.
use sc_service::{TFullBackend, TFullClient};

pub type ParachainBuilder<Block, RtApi, CIDP, DP> = fudge_core::ParachainBuilder<
	Block,
	RtApi,
	TWasmExecutor,
	CIDP,
	(),
	DP,
	TFullBackend<Block>,
	TFullClient<Block, RtApi, TWasmExecutor>,
>;

pub type RelaychainBuilder<Block, RtApi, Runtime, CIDP, DP> = fudge_core::RelaychainBuilder<
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

pub type StandaloneBuilder<Block, RtApi, CIDP, DP> = fudge_core::StandAloneBuilder<
	Block,
	RtApi,
	TWasmExecutor,
	CIDP,
	(),
	DP,
	TFullBackend<Block>,
	TFullClient<Block, RtApi, TWasmExecutor>,
>;

pub mod primitives {
	pub use fudge_core::{FudgeParaChain, PoolState};
	pub use polkadot_parachain::primitives::Id as ParaId;

	#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash)]
	pub enum Chain {
		Relay,
		Para(u32),
	}
}
