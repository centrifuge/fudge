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

use crate::builder::relay_chain::types::Heads;
///! Test for the ParachainBuilder
use crate::digest::{DigestCreator, FudgeBabeDigest};
use crate::inherent::{
	FudgeDummyInherentRelayParachain, FudgeInherentParaParachain, FudgeInherentTimestamp,
};
use crate::provider::EnvProvider;
use crate::RelaychainBuilder;
use crate::{FudgeParaChain, ParachainBuilder};
use centrifuge_runtime::{
	Block as PTestBlock, Runtime as PRuntime, RuntimeApi as PTestRtApi, WASM_BINARY as PCODE,
};
use frame_support::traits::GenesisBuild;
use polkadot_core_primitives::{Block as RTestBlock, Header as RTestHeader};
use polkadot_parachain::primitives::Id;
use polkadot_runtime::{Runtime as RRuntime, RuntimeApi as RTestRtApi, WASM_BINARY as RCODE};
use polkadot_runtime_parachains::paras;
use sc_executor::{WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{TFullBackend, TFullClient, TaskManager};
use sp_api::BlockId;
use sp_consensus_babe::{digests::CompatibleDigestItem, SlotDuration};
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::{DigestItem, Storage};
use sp_std::sync::Arc;
use tokio::runtime::Handle;

type RelayBuilder<R> = RelaychainBuilder<
	RTestBlock,
	RTestRtApi,
	TestExec<sp_io::SubstrateHostFunctions>,
	Box<
		dyn CreateInherentDataProviders<
			RTestBlock,
			(),
			InherentDataProviders = (
				FudgeInherentTimestamp,
				sp_consensus_babe::inherents::InherentDataProvider,
				sp_authorship::InherentDataProvider<RTestHeader>,
				FudgeDummyInherentRelayParachain<RTestHeader>,
			),
		>,
	>,
	(),
	Box<dyn DigestCreator + Send + Sync>,
	R,
	TFullBackend<RTestBlock>,
	TFullClient<RTestBlock, RTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
>;

fn generate_default_setup_parachain<CIDP, DP>(
	manager: &TaskManager,
	storage: Storage,
	cidp: Box<
		dyn FnOnce(
			Arc<TFullClient<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>>,
		) -> CIDP,
	>,
	dp: DP,
) -> ParachainBuilder<
	PTestBlock,
	PTestRtApi,
	TestExec<sp_io::SubstrateHostFunctions>,
	CIDP,
	(),
	DP,
	TFullBackend<PTestBlock>,
	TFullClient<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
>
where
	CIDP: CreateInherentDataProviders<PTestBlock, ()> + 'static,
	DP: DigestCreator + 'static,
{
	let mut provider =
		EnvProvider::<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>::with_code(
			PCODE.unwrap(),
		);
	provider.insert_storage(storage);

	let (client, backend) = provider.init_default(
		TestExec::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
		Box::new(manager.spawn_handle()),
	);
	let client = Arc::new(client);
	let clone_client = client.clone();

	ParachainBuilder::<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>, _, _, _>::new(
		manager,
		backend,
		client,
		cidp(clone_client),
		dp,
	)
}

fn generate_default_setup_relay_chain<Runtime>(
	manager: &TaskManager,
	mut storage: Storage,
) -> RelaychainBuilder<
	RTestBlock,
	RTestRtApi,
	TestExec<sp_io::SubstrateHostFunctions>,
	Box<
		dyn CreateInherentDataProviders<
			RTestBlock,
			(),
			InherentDataProviders = (
				FudgeInherentTimestamp,
				sp_consensus_babe::inherents::InherentDataProvider,
				sp_authorship::InherentDataProvider<RTestHeader>,
				FudgeDummyInherentRelayParachain<RTestHeader>,
			),
		>,
	>,
	(),
	Box<dyn DigestCreator + Send + Sync>,
	Runtime,
	TFullBackend<RTestBlock>,
	TFullClient<RTestBlock, RTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
>
where
	Runtime: pallet_babe::Config
		+ polkadot_runtime_parachains::configuration::Config
		+ paras::Config
		+ frame_system::Config
		+ pallet_timestamp::Config<Moment = u64>,
{
	let mut provider =
		EnvProvider::<RTestBlock, RTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>::with_code(
			RCODE.unwrap(),
		);
	polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime>::default()
		.assimilate_storage(&mut storage)
		.unwrap();
	provider.insert_storage(storage);

	let (client, backend) = provider.init_default(
		TestExec::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
		Box::new(manager.spawn_handle()),
	);
	let client = Arc::new(client);
	let clone_client = client.clone();
	// Init timestamp instance
	FudgeInherentTimestamp::new(0, sp_std::time::Duration::from_secs(6), None);

	let cidp = Box::new(
		|clone_client: Arc<
			TFullClient<RTestBlock, RTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
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

					let timestamp = FudgeInherentTimestamp::get_instance(0)
						.expect("Instance is initialized. qed");

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							timestamp.current_time(),
							SlotDuration::from_millis(6 * 1000),
						);

					let relay_para_inherent = FudgeDummyInherentRelayParachain::new(parent_header);
					Ok((timestamp, slot, uncles, relay_para_inherent))
				}
			}
		},
	);

	let dp = Box::new(move || async move {
		let mut digest = sp_runtime::Digest::default();
		let slot_duration = pallet_babe::Pallet::<Runtime>::slot_duration();

		digest.push(<DigestItem as CompatibleDigestItem>::babe_pre_digest(
			FudgeBabeDigest::pre_digest(
				FudgeInherentTimestamp::get_instance(0)
					.expect("Instance is initialised. qed")
					.current_time(),
				SlotDuration::from_millis(slot_duration.into()),
			),
		));

		Ok(digest)
	});

	RelaychainBuilder::<
		RTestBlock,
		RTestRtApi,
		TestExec<sp_io::SubstrateHostFunctions>,
		_,
		_,
		_,
		Runtime,
	>::new(manager, backend, client, Box::new(cidp(clone_client)), dp)
}

#[tokio::test]
async fn parachain_creates_correct_inherents() {
	super::utils::init_logs();
	let manager = TaskManager::new(Handle::current(), None).unwrap();

	let mut relay_builder =
		generate_default_setup_relay_chain::<RRuntime>(&manager, Storage::default());
	let para_id = Id::from(2001u32);
	let inherent_builder = relay_builder.inherent_builder(para_id.clone());
	// Init timestamp instance
	FudgeInherentTimestamp::new(1, sp_std::time::Duration::from_secs(12), None);

	let cidp = Box::new(|_| {
		move |_parent: H256, ()| {
			let inherent_builder_clone = inherent_builder.clone();
			async move {
				let timestamp =
					FudgeInherentTimestamp::get_instance(1).expect("Instance is initialized. qed");

				let slot =
					sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						timestamp.current_time(),
						SlotDuration::from_millis(12 * 1000),
					);
				let inherent = inherent_builder_clone.parachain_inherent().await.unwrap();
				let relay_para_inherent = FudgeInherentParaParachain::new(inherent);
				Ok((timestamp, slot, relay_para_inherent))
			}
		}
	});
	let dp = Box::new(move || async move { Ok(sp_runtime::Digest::default()) });
	let mut builder = generate_default_setup_parachain(&manager, Storage::default(), cidp, dp);

	let para = FudgeParaChain {
		id: para_id,
		head: builder.head(),
		code: builder.code(),
	};
	relay_builder.onboard_para(para).unwrap();
	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	let num_start = builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let num_after_one = builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	assert_eq!(num_start + 1, num_after_one);

	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let num_after_two = builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	assert_eq!(num_start + 2, num_after_two);

	let para = FudgeParaChain {
		id: para_id,
		head: builder.head(),
		code: builder.code(),
	};
	relay_builder.onboard_para(para).unwrap();
	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	let para_head = relay_builder
		.with_state(|| Heads::try_get(para_id).unwrap())
		.unwrap();

	assert_eq!(builder.head(), para_head);
}
