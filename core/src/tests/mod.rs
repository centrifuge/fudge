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

use crate::inherent::{FudgeInherentRelayParachain, FudgeInherentTimestamp};
use crate::provider::EnvProvider;
use crate::StandAloneBuilder;
use frame_benchmarking::account;
use frame_support::inherent::BlockT;
use fudge_utils::Signer;
use polkadot_runtime::{
	Block as TestBlock, Runtime, RuntimeApi as TestRtApi, SignedExtra, WASM_BINARY as CODE,
};
use sc_client_db::Backend;
use sc_executor::sp_wasm_interface::HostFunctions;
use sc_executor::{RuntimeVersionOf, WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{LocalCallExecutor, SpawnTaskHandle, TFullBackend, TFullClient, TaskManager};
use sp_api::{BlockId, ConstructRuntimeApi};
use sp_core::traits::CodeExecutor;
use sp_core::H256;
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_keystore::SyncCryptoStore;
use sp_runtime::traits::HashFor;
use sp_runtime::{AccountId32, CryptoTypeId, KeyTypeId, MultiAddress, Storage};
use sp_std::str::FromStr;
use sp_std::sync::Arc;
use sp_storage::well_known_keys::CODE as CODE_KEY;
use tokio::runtime::Handle;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");
const CRYPTO_TYPE: CryptoTypeId = CryptoTypeId(*b"test");

fn generate_default_setup_stand_alone<CIDP>(
	handle: SpawnTaskHandle,
	storage: Storage,
	cidp: Box<dyn FnOnce(Arc<TFullClient<TestBlock, TestRtApi, TestExec>>) -> CIDP>,
) -> StandAloneBuilder<
	TestBlock,
	TestRtApi,
	TestExec,
	CIDP,
	(),
	TFullBackend<TestBlock>,
	TFullClient<TestBlock, TestRtApi, TestExec>,
>
where
	CIDP: CreateInherentDataProviders<TestBlock, ()> + 'static,
{
	let host_functions = sp_io::SubstrateHostFunctions::host_functions();
	let mut provider = EnvProvider::<TestBlock, TestRtApi, TestExec>::with_code(CODE.unwrap());
	provider.insert_storage(storage);

	let (client, backend) = provider.init_default(
		TestExec::new(
			WasmExecutionMethod::Interpreted,
			None,
			host_functions,
			6,
			None,
		),
		Box::new(handle.clone()),
	);
	let client = Arc::new(client);
	let clone_client = client.clone();

	StandAloneBuilder::<TestBlock, TestRtApi, TestExec, _, _>::new(
		handle.clone(),
		backend,
		client,
		cidp(clone_client),
	)
}

#[tokio::test]
async fn mutating_genesis_works() {
	let manager = TaskManager::new(Handle::current(), None).unwrap();
	let mut storage = pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::default(), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();

	let cidp = Box::new(
		|clone_client: Arc<TFullClient<TestBlock, TestRtApi, TestExec>>| {
			move |parent: H256, ()| {
				let client = clone_client.clone();
				let parent_header = client
					.header(&BlockId::Hash(parent.clone()))
					.unwrap()
					.unwrap();

				async move {
					let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
						&*client, parent,
					)?;

					let timestamp = FudgeInherentTimestamp::new(0, 12, None);

					let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
					timestamp.current_time(),
					std::time::Duration::from_secs(6),
				);

					let relay_para_inherent = FudgeInherentRelayParachain::new(parent_header);
					Ok((timestamp, uncles, slot, relay_para_inherent))
				}
			}
		},
	);

	let mut builder = generate_default_setup_stand_alone(manager.spawn_handle(), storage, cidp);

	let (send_data_pre, recv_data_pre) = builder
		.with_mut_state(|| {
			polkadot_runtime::Balances::transfer(
				polkadot_runtime::Origin::signed(AccountId32::default()),
				MultiAddress::Id(account("test", 0, 0)),
				1_000_000_000_000u128,
			)
			.unwrap();

			(
				frame_system::Account::<Runtime>::get(AccountId32::default()),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	let (send_data_post, recv_data_post) = builder
		.with_state(|| {
			(
				frame_system::Account::<Runtime>::get(AccountId32::default()),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	assert_eq!(send_data_pre, send_data_post);
	assert_eq!(recv_data_pre, recv_data_post);
}

#[tokio::test]
async fn opening_state_from_db_path_works() {
	/*
		let mut host_functions = sp_io::SubstrateHostFunctions::host_functions();
		let manager = TaskManager::new(Handle::current(), None).unwrap();

		let mut provider = EnvProvider::<TestBlock, TestRtApi, TestExec>::from_db(std::path::PathBuf::from("/Users/frederik/Projects/centrifuge-fudge/core/src/tests/data/relay-chain/rococo_local_testnet/db/full"));
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
		let client = Arc::new(client);

		let mut builder = StandAloneBuilder::<TestBlock, TestRtApi, TestExec,  _, _>::new(backend, client);

		builder.with_state_at(BlockId::Number(1), || {

		}).unwrap();

		builder.with_state_at(BlockId::Number(20), || {

		}).unwrap();
	*/
}

#[tokio::test]
async fn build_relay_block_works() {
	let manager = TaskManager::new(Handle::current(), None).unwrap();
	let cidp = Box::new(
		|clone_client: Arc<TFullClient<TestBlock, TestRtApi, TestExec>>| {
			move |parent: H256, ()| {
				let client = clone_client.clone();
				let parent_header = client
					.header(&BlockId::Hash(parent.clone()))
					.unwrap()
					.unwrap();

				async move {
					let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
						&*client, parent,
					)?;

					let timestamp = FudgeInherentTimestamp::new(0, 12, None);

					let slot =
					sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
						timestamp.current_time(),
						std::time::Duration::from_secs(6),
					);

					let relay_para_inherent = FudgeInherentRelayParachain::new(parent_header);
					Ok((timestamp, uncles, slot, relay_para_inherent))
				}
			}
		},
	);
	let mut builder =
		generate_default_setup_stand_alone(manager.spawn_handle(), Storage::default(), cidp);

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block();
	builder.import_block();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after)
}

#[tokio::test]
async fn building_relay_block_with_extrinsics_works() {
	let manager = TaskManager::new(Handle::current(), None).unwrap();
	let key_store = sc_keystore::LocalKeystore::in_memory();
	let sender = key_store.sr25519_generate_new(KEY_TYPE, None).unwrap();
	let receiver = key_store.sr25519_generate_new(KEY_TYPE, None).unwrap();

	let mut storage = Storage::default();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(AccountId32::from(sender), 10_000_000_000_000u128),
			(AccountId32::from(receiver), 10_000_000_000_000u128),
		],
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	let cidp = Box::new(
		|clone_client: Arc<TFullClient<TestBlock, TestRtApi, TestExec>>| {
			move |parent: H256, ()| {
				let client = clone_client.clone();
				let parent_header = client
					.header(&BlockId::Hash(parent.clone()))
					.unwrap()
					.unwrap();

				async move {
					let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
						&*client, parent,
					)?;

					let timestamp = FudgeInherentTimestamp::new(0, 12, None);

					let slot =
					sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
						timestamp.current_time(),
						std::time::Duration::from_secs(6),
					);

					let relay_para_inherent = FudgeInherentRelayParachain::new(parent_header);
					Ok((timestamp, uncles, slot, relay_para_inherent))
				}
			}
		},
	);
	let mut builder = generate_default_setup_stand_alone(manager.spawn_handle(), storage, cidp);

	let signer = Signer::new(key_store.into(), CRYPTO_TYPE, KEY_TYPE);
	/*
	let extra: SignedExtra = (
		frame_system::CheckSpecVersion::<Runtime>::new(),
		frame_system::CheckTxVersion::<Runtime>::new(),
		frame_system::CheckGenesis::<Runtime>::new(),
		frame_system::CheckMortality::<Runtime>::from(sp_runtime::generic::Era::Immortal),
		frame_system::CheckNonce::<Runtime>::from(0),
		frame_system::CheckWeight::<Runtime>::new(),
		pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
		claims::PrevalidateAttests::<Runtime>::new(),
	);

	builder.append_extrinsic(signer.signed_ext(
		Call::Balances(
			pallet_balances::Call::transfer {
				dest: MultiAddress::Id(receiver.clone()),
				value: 1_000_000_000_000u128,
			}),
	sender.clone(),
			extra
		).unwrap()
	);
	*/
}
