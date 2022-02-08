// Copyright 2021 Centrifuge Foundation (centrifuge.io).
//
// This file is part of the Centrifuge chain project.
// Centrifuge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).
// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use crate::{RelaychainBuilder, StateProvider};
//use node_primitives::{AccountId, Block as TestBlock};
use polkadot_runtime::{Block as TestBlock, RuntimeApi as TestRtApi, WASM_BINARY as CODE};
use sc_client_db::Backend;
use sc_executor::WasmExecutor as TestExec;
use sc_service::{LocalCallExecutor, TFullClient};
use sp_runtime::{AccountId32, MultiAddress};

#[test]
fn dummy_test() {
	let provider = StateProvider::<sc_client_db::Backend<TestBlock>, TestBlock>::empty(CODE);
	let mut builder = RelaychainBuilder::<
		TestBlock,
		TestRtApi,
		TestExec,
		Backend<TestBlock>,
		TFullClient<
			Backend<TestBlock>,
			TestRtApi,
			LocalCallExecutor<TestBlock, Backend<TestBlock>, TestExec>,
		>,
	>::new(provider);

	let res = builder.with_state(|| {
		polkadot_runtime::Balances::transfer(
			polkadot_runtime::Origin::signed(AccountId32::default()),
			MultiAddress::Id(AccountId32::default()),
			1_000_000_000_000u128,
		)
	});
}
