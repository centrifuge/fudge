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

use crate::digest::{DigestCreator, DigestProvider, FudgeBabeDigest};
use crate::inherent::{FudgeDummyInherentRelayParachain, FudgeInherentTimestamp};
use crate::provider::EnvProvider;
use crate::StandAloneBuilder;
use frame_benchmarking::account;
use polkadot_runtime::{Block as TestBlock, Runtime, RuntimeApi as TestRtApi, WASM_BINARY as CODE};
use sc_executor::{WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{TFullBackend, TFullClient, TaskManager};
use sp_api::BlockId;
use sp_consensus_babe::SlotDuration;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::{AccountId32, CryptoTypeId, KeyTypeId, MultiAddress, Storage};
use sp_std::sync::Arc;
use tokio::runtime::Handle;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");
const CRYPTO_TYPE: CryptoTypeId = CryptoTypeId(*b"test");

fn generate_default_setup_stand_alone<CIDP, DP>(
	manager: &TaskManager,
	storage: Storage,
	cidp: Box<
		dyn FnOnce(
			Arc<TFullClient<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>>>,
		) -> CIDP,
	>,
	dp: DP,
) -> StandAloneBuilder<
	TestBlock,
	TestRtApi,
	TestExec<sp_io::SubstrateHostFunctions>,
	CIDP,
	(),
	DP,
	TFullBackend<TestBlock>,
	TFullClient<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
>
where
	CIDP: CreateInherentDataProviders<TestBlock, ()> + 'static,
	DP: DigestCreator<TestBlock> + 'static,
{
	let mut provider =
		EnvProvider::<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>>::with_code(
			CODE.unwrap(),
		);
	provider.insert_storage(storage);

	let (client, backend) = provider.init_default(
		TestExec::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
		Box::new(manager.spawn_handle()),
	);
	let client = Arc::new(client);
	let clone_client = client.clone();

	StandAloneBuilder::<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>, _, _, _>::new(
		manager,
		backend,
		client,
		cidp(clone_client),
		dp,
	)
}

#[tokio::test]
async fn mutating_genesis_works() {
	super::utils::init_logs();

	let manager = TaskManager::new(Handle::current(), None).unwrap();
	let storage = pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::new([0u8; 32]), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();
	// Init timestamp instance_id
	let instance_id =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(6), None);

	let cidp = Box::new(
		move |clone_client: Arc<
			TFullClient<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
		>| {
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

					let timestamp = FudgeInherentTimestamp::get_instance(instance_id)
						.expect("Instance is initialized. qed");

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							timestamp.current_time(),
							SlotDuration::from_millis(std::time::Duration::from_secs(6).as_millis() as u64),
						);

					let relay_para_inherent = FudgeDummyInherentRelayParachain::new(parent_header);
					Ok((timestamp, uncles, slot, relay_para_inherent))
				}
			}
		},
	);

	let dp = Box::new(move |parent, inherents| async move {
		let mut digest = sp_runtime::Digest::default();

		let babe = FudgeBabeDigest::<TestBlock>::new();
		babe.append_digest(&mut digest, &parent, &inherents).await?;

		Ok(digest)
	});

	let mut builder = generate_default_setup_stand_alone(&manager, storage, cidp, dp);

	let (send_data_pre, recv_data_pre) = builder
		.with_mut_state(|| {
			polkadot_runtime::Balances::transfer(
				polkadot_runtime::Origin::signed(AccountId32::new([0u8; 32])),
				MultiAddress::Id(account("test", 0, 0)),
				1_000_000_000_000u128,
			)
			.unwrap();

			(
				frame_system::Account::<Runtime>::get(AccountId32::new([0u8; 32])),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	let (send_data_post, recv_data_post) = builder
		.with_state(|| {
			(
				frame_system::Account::<Runtime>::get(AccountId32::new([0u8; 32])),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	assert_eq!(send_data_pre, send_data_post);
	assert_eq!(recv_data_pre, recv_data_post);
}

// TODO: THis should be tested....
/*
#[tokio::test]
async fn opening_state_from_db_path_works() {
	super::utils::init_logs();


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
}
*/

#[tokio::test]
async fn build_relay_block_works() {
	// install global collector configured based on RUST_LOG env var.
	super::utils::init_logs();

	let manager = TaskManager::new(Handle::current(), None).unwrap();
	// Init timestamp instance_id
	let instance_id =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(6), None);

	let cidp = Box::new(
		move |clone_client: Arc<
			TFullClient<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
		>| {
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

					let timestamp = FudgeInherentTimestamp::get_instance(instance_id)
						.expect("Instance is initialized. qed");
					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							timestamp.current_time(),
							SlotDuration::from_millis(std::time::Duration::from_secs(6).as_millis() as u64),
						);

					let relay_para_inherent = FudgeDummyInherentRelayParachain::new(parent_header);
					Ok((timestamp, slot, uncles, relay_para_inherent))
				}
			}
		},
	);
	let dp = Box::new(move |parent, inherents| async move {
		let mut digest = sp_runtime::Digest::default();

		let babe = FudgeBabeDigest::<TestBlock>::new();
		babe.append_digest(&mut digest, &parent, &inherents).await?;

		Ok(digest)
	});
	let mut builder = generate_default_setup_stand_alone(&manager, Storage::default(), cidp, dp);

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);
}

#[tokio::test]
async fn build_relay_block_works_and_mut_is_build_upon() {
	super::utils::init_logs();

	let manager = TaskManager::new(Handle::current(), None).unwrap();
	// Init timestamp instance_id
	let instance_id =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(6), None);

	let cidp = Box::new(
		move |clone_client: Arc<
			TFullClient<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
		>| {
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

					let timestamp = FudgeInherentTimestamp::get_instance(instance_id)
						.expect("Instance is initialized. qed");
					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							timestamp.current_time(),
							SlotDuration::from_millis(std::time::Duration::from_secs(6).as_millis() as u64),
						);

					let relay_para_inherent = FudgeDummyInherentRelayParachain::new(parent_header);
					Ok((timestamp, slot, uncles, relay_para_inherent))
				}
			}
		},
	);
	let dp = Box::new(move |parent, inherents| async move {
		let mut digest = sp_runtime::Digest::default();

		let babe = FudgeBabeDigest::<TestBlock>::new();
		babe.append_digest(&mut digest, &parent, &inherents).await?;

		Ok(digest)
	});

	let storage = pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::new([0u8; 32]), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();

	let mut builder = generate_default_setup_stand_alone(&manager, storage, cidp, dp);

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);

	let (send_data_pre, recv_data_pre) = builder
		.with_mut_state(|| {
			polkadot_runtime::Balances::transfer(
				polkadot_runtime::Origin::signed(AccountId32::new([0u8; 32])),
				MultiAddress::Id(account("test", 0, 0)),
				1_000_000_000_000u128,
			)
			.unwrap();

			(
				frame_system::Account::<Runtime>::get(AccountId32::new([0u8; 32])),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);

	let (send_data_post, recv_data_post) = builder
		.with_state(|| {
			(
				frame_system::Account::<Runtime>::get(AccountId32::new([0u8; 32])),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	assert_eq!(send_data_pre, send_data_post);
	assert_eq!(recv_data_pre, recv_data_post);
}
