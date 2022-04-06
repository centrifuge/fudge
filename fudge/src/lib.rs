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

///! FUDGE - FUlly Decoupled Generic Environment for Substrate-based Chains
///!
///! Generally only this dependency is needed in order to use FUDGE.
///! Developers who want to use the more raw apis and types are
///! referred to the fudge-core repository.
use sc_executor::WasmExecutor;
use sc_service::{TFullBackend, TFullClient};

pub use fudge_companion::companion;
pub use fudge_core::{digest, inherent, provider::EnvProvider};

pub type ParachainBuilder<Block, RtApi, CIDP, DP, H> = fudge_core::ParachainBuilder<
	Block,
	RtApi,
	WasmExecutor<H>,
	CIDP,
	(),
	DP,
	TFullBackend<Block>,
	TFullClient<Block, RtApi, WasmExecutor<H>>,
>;

pub type RelaychainBuilder<Block, RtApi, Runtime, CIDP, DP, H> = fudge_core::RelaychainBuilder<
	Block,
	RtApi,
	WasmExecutor<H>,
	CIDP,
	(),
	DP,
	Runtime,
	TFullBackend<Block>,
	TFullClient<Block, RtApi, WasmExecutor<H>>,
>;

pub type StandaloneBuilder<Block, RtApi, CIDP, DP, H> = fudge_core::StandAloneBuilder<
	Block,
	RtApi,
	WasmExecutor<H>,
	CIDP,
	(),
	DP,
	TFullBackend<Block>,
	TFullClient<Block, RtApi, WasmExecutor<H>>,
>;

pub mod primitives {
	pub use fudge_core::FudgeParaChain;
	pub use polkadot_parachain::primitives::Id as ParaId;

	#[derive(Copy, Clone)]
	pub enum Chain {
		Relay,
		Para(u32),
	}
}
