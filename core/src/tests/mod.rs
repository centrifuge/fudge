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

use crate::RelaychainBuilder;
//use node_primitives::{AccountId, Block as TestBlock};
use polkadot_runtime::{Block as TestBlock, RuntimeApi as TestRtApi, WASM_BINARY as CODE, Runtime};
use sc_client_db::Backend;
use sc_executor::{WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{LocalCallExecutor, TaskManager, TFullClient};
use sp_runtime::{AccountId32, MultiAddress};
use crate::provider::EnvProvider;
use tokio::runtime::{Handle};
use sc_executor::sp_wasm_interface::HostFunctions;
use frame_benchmarking::account;

#[tokio::test]
async fn dummy_test() {
	//let runtime = Runtime::new().unwrap();
	/// TODO: Might need to append here from runtime, as there is no dispatch.
	let mut host_functions = sp_io::SubstrateHostFunctions::host_functions();
	let manager = TaskManager::new(Handle::current(), None).unwrap();

	let mut storage = frame_system::GenesisConfig {
		changes_trie_config: None,
		code: CODE.unwrap().to_vec()
	}
		.build_storage::<Runtime>()
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::default(), 10_000_000_000_000u128)
		]
	}
		.assimilate_storage(&mut storage)
		.unwrap();

	let mut provider = EnvProvider::<TestBlock, TestRtApi, TestExec>::with_code(CODE.unwrap());
	provider.insert_storage(storage);

	let (client, backend) = provider
		.init_default(
			TestExec::new(
			WasmExecutionMethod::Interpreted,
			None,
			host_functions,
			6,
			None,
		),
			Box::new(manager.spawn_handle())
		);

	let mut builder = RelaychainBuilder::<TestBlock, TestRtApi, TestExec, _, _>::new(backend, client);

	let (data, data1) = builder.with_mut_state(|| {

		let data = frame_system::Account::<Runtime>::get(AccountId32::default());

		let res = polkadot_runtime::Balances::transfer(
			polkadot_runtime::Origin::signed(AccountId32::default()),
			MultiAddress::Id(account("test", 0, 0)),
			1_000_000_000_000u128,
		);

		(frame_system::Account::<Runtime>::get(AccountId32::default()), frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)))
	}).unwrap();

	let (data2, data3) = builder.with_mut_state(|| {

		(frame_system::Account::<Runtime>::get(AccountId32::default()), frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)))
	}).unwrap();



	let x = 0;
}
