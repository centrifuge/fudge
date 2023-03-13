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

use std::path::PathBuf;

use frame_benchmarking::account;
use polkadot_runtime::{Block as TestBlock, Runtime, RuntimeApi as TestRtApi, WASM_BINARY as CODE};
use sc_service::TFullClient;
use sp_api::BlockId;
use sp_consensus_babe::SlotDuration;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::{AccountId32, MultiAddress, Storage};
use sp_std::sync::Arc;
use tokio::runtime::Handle;

use crate::{
	builder::stand_alone::StandAloneBuilder,
	digest::{DigestCreator, DigestProvider, FudgeBabeDigest},
	inherent::{FudgeDummyInherentRelayParachain, FudgeInherentTimestamp},
	provider::{backend::DiskDb, state::StateProvider, TWasmExecutor},
};

fn cidp_and_dp(
	client: Arc<TFullClient<TestBlock, TestRtApi, TWasmExecutor>>,
) -> (
	impl CreateInherentDataProviders<TestBlock, ()>,
	impl DigestCreator<TestBlock>,
) {
	// Init timestamp instance_id
	let instance_id =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(6), None);

	let cidp = move |clone_client: Arc<TFullClient<TestBlock, TestRtApi, TWasmExecutor>>| {
		move |parent: H256, ()| {
			let client = clone_client.clone();
			let parent_header = client
				.header(parent.clone())
				.unwrap()
				.unwrap();

			async move {
				let uncles =
					sc_consensus_uncles::create_uncles_inherent_data_provider(&*client, parent)?;

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
	};

	let dp = move |parent, inherents| async move {
		let mut digest = sp_runtime::Digest::default();

		let babe = FudgeBabeDigest::<TestBlock>::new();
		babe.append_digest(&mut digest, &parent, &inherents).await?;

		Ok(digest)
	};

	(cidp(client), dp)
}

fn default_builder(
	handle: Handle,
	genesis: Storage,
) -> StandAloneBuilder<
	TestBlock,
	TestRtApi,
	TWasmExecutor,
	impl CreateInherentDataProviders<TestBlock, ()>,
	(),
	impl DigestCreator<TestBlock>,
> {
	let mut state = StateProvider::new(CODE.expect("Wasm is build. Qed."));
	state.insert_storage(genesis);

	let mut init = crate::provider::initiator::default(handle);
	init.with_genesis(Box::new(state));

	StandAloneBuilder::new(init, cidp_and_dp)
}

fn default_builder_disk(
	handle: Handle,
	path: PathBuf,
	genesis: Storage,
) -> StandAloneBuilder<
	TestBlock,
	TestRtApi,
	TWasmExecutor,
	impl CreateInherentDataProviders<TestBlock, ()>,
	(),
	impl DigestCreator<TestBlock>,
> {
	let mut state = StateProvider::new(CODE.expect("Wasm is build. Qed."));
	state.insert_storage(genesis);

	let mut init = crate::provider::initiator::default_with(handle, DiskDb::new(path));
	init.with_genesis(Box::new(state));

	StandAloneBuilder::new(init, cidp_and_dp)
}
#[tokio::test]
async fn mutating_genesis_works() {
	super::utils::init_logs();

	let genesis = pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::new([0u8; 32]), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();

	let mut builder = default_builder(Handle::current(), genesis);
	let (send_data_pre, recv_data_pre) = builder
		.with_mut_state(|| {
			polkadot_runtime::Balances::transfer(
				polkadot_runtime::RuntimeOrigin::signed(AccountId32::new([0u8; 32])),
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

#[tokio::test]
async fn opening_state_from_db_path_works() {
	use std::fs::{create_dir_all, remove_dir_all};

	super::utils::init_logs();

	// We need some temp folder here for testing
	let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("src");
	path.push("tests");
	path.push("test_dbs");
	path.push("test_opening_database");
	let static_path = path;
	if static_path.is_dir() {
		// Clean up before the test, in case there where failures in between
		remove_dir_all(&static_path).unwrap();
	}
	create_dir_all(&static_path).unwrap();

	let genesis = pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::new([0u8; 32]), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();

	let mut builder = default_builder_disk(Handle::current(), static_path.clone(), genesis);

	for _ in 0..20 {
		builder.build_block().unwrap();
		builder.import_block().unwrap();
	}

	let (send_data_pre_1, recv_data_pre_1) = builder
		.with_state_at(BlockId::Number(1), || {
			(
				frame_system::Account::<Runtime>::get(AccountId32::new([0u8; 32])),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	let (send_data_pre_20, recv_data_pre_20) = builder
		.with_state_at(BlockId::Number(20), || {
			(
				frame_system::Account::<Runtime>::get(AccountId32::new([0u8; 32])),
				frame_system::Account::<Runtime>::get(account::<AccountId32>("test", 0, 0)),
			)
		})
		.unwrap();

	assert_eq!(send_data_pre_1, send_data_pre_20);
	assert_eq!(recv_data_pre_1, recv_data_pre_20);

	let events_at_20_before = builder
		.with_state_at(BlockId::Number(20), || {
			frame_system::Pallet::<Runtime>::events()
		})
		.unwrap();
	assert!(!events_at_20_before
		.into_iter()
		.map(|record| record.event.clone())
		.collect::<Vec<_>>()
		.contains(&polkadot_runtime::RuntimeEvent::Balances(
			pallet_balances::Event::<Runtime>::Transfer {
				from: AccountId32::new([0u8; 32]),
				to: account::<AccountId32>("test", 0, 0),
				amount: 1_000_000_000_000u128
			}
		)));

	let (send_data_post_20, recv_data_post_20) = builder
		.with_mut_state(|| {
			polkadot_runtime::Balances::transfer(
				polkadot_runtime::RuntimeOrigin::signed(AccountId32::new([0u8; 32])),
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

	let events_at_20_after = builder
		.with_state_at(BlockId::Number(20), || {
			frame_system::Pallet::<Runtime>::events()
		})
		.unwrap();

	assert_eq!(
		send_data_pre_20.data.free,
		send_data_post_20.data.free + 1_000_000_000_000u128,
	);
	assert_eq!(
		recv_data_pre_20.data.free,
		recv_data_post_20.data.free - 1_000_000_000_000u128
	);
	assert!(events_at_20_after
		.iter()
		.map(|record| record.event.clone())
		.collect::<Vec<_>>()
		.contains(&polkadot_runtime::RuntimeEvent::Balances(
			pallet_balances::Event::<Runtime>::Transfer {
				from: AccountId32::new([0u8; 32]),
				to: account::<AccountId32>("test", 0, 0),
				amount: 1_000_000_000_000u128
			}
		)));

	// Clean up after the test

	remove_dir_all(&static_path).unwrap();
}

#[tokio::test]
async fn build_relay_block_works() {
	// install global collector configured based on RUST_LOG env var.
	super::utils::init_logs();

	let mut builder = default_builder(Handle::current(), Storage::default());

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

	let genesis = pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::new([0u8; 32]), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();

	let mut builder = default_builder(Handle::current(), genesis);

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
				polkadot_runtime::RuntimeOrigin::signed(AccountId32::new([0u8; 32])),
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
