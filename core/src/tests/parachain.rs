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

use frame_support::traits::GenesisBuild;
use fudge_test_runtime::{
	AuraId, Block as PTestBlock, Runtime as PRuntime, RuntimeApi as PTestRtApi,
	WASM_BINARY as PCODE,
};
use polkadot_core_primitives::Block as RTestBlock;
use polkadot_parachain::primitives::Id;
use polkadot_runtime::{Runtime as RRuntime, RuntimeApi as RTestRtApi, WASM_BINARY as RCODE};
use sc_service::{TFullBackend, TFullClient};
use sp_consensus_babe::SlotDuration;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::Storage;
use sp_std::sync::Arc;
use tokio::runtime::Handle;

///! Test for the ParachainBuilder
use crate::digest::{DigestCreator, DigestProvider, FudgeAuraDigest, FudgeBabeDigest};
use crate::{
	builder::{
		parachain::{FudgeParaChain, ParachainBuilder},
		relay_chain::{types::Heads, InherentBuilder, RelaychainBuilder},
	},
	inherent::{
		FudgeDummyInherentRelayParachain, FudgeInherentParaParachain, FudgeInherentTimestamp,
	},
	provider::{state::StateProvider, TWasmExecutor},
};

fn default_para_builder(
	handle: Handle,
	genesis: Storage,
	inherent_builder: InherentBuilder<
		TFullClient<RTestBlock, RTestRtApi, TWasmExecutor>,
		TFullBackend<RTestBlock>,
	>,
) -> ParachainBuilder<
	PTestBlock,
	PTestRtApi,
	TWasmExecutor,
	impl CreateInherentDataProviders<PTestBlock, ()>,
	(),
	DP,
	TFullBackend<PTestBlock>,
	TFullClient<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
>
where
	CIDP: CreateInherentDataProviders<PTestBlock, ()> + 'static,
	DP: DigestCreator<PTestBlock> + 'static,
{
	let mut provider =
		EnvProvider::<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>::with_code(
			PCODE.unwrap(),
		)
		.unwrap();

	pallet_aura::GenesisConfig::<PRuntime> {
		authorities: vec![AuraId::from(sp_core::sr25519::Public([0u8; 32]))],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	provider.insert_storage(storage).unwrap();

	let (client, backend) = provider
		.init_default(
			TestExec::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
			Box::new(manager.spawn_handle()),
		)
		.unwrap();
	let client = Arc::new(client);

	let mut init = crate::provider::initiator::default(handle);

	init.with_genesis(Box::new(state));

	// Init timestamp instance_id
	let instance_id_para =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(12), None);

	let cidp = move |_parent: H256, ()| {
		let inherent_builder_clone = inherent_builder.clone();
		async move {
			let timestamp = FudgeInherentTimestamp::get_instance(instance_id_para)
				.expect("Instance is initialized. qed");

			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					timestamp.current_time(),
					SlotDuration::from_millis(std::time::Duration::from_secs(6).as_millis() as u64),
				);
			let inherent = inherent_builder_clone.parachain_inherent().await.unwrap();
			let relay_para_inherent = FudgeInherentParaParachain::new(inherent);
			Ok((timestamp, slot, relay_para_inherent))
		}
	};

	let dp = |clone_client: Arc<TFullClient<PTestBlock, PTestRtApi, TWasmExecutor>>| {
		move |parent, inherents| {
			let client = clone_client.clone();

			async move {
				let aura = FudgeAuraDigest::<
					PTestBlock,
					TFullClient<PTestBlock, PTestRtApi, TWasmExecutor>,
				>::new(&*client);

				let digest = aura.build_digest(&parent, &inherents).await?;
				Ok(digest)
			}
		}
	};

	ParachainBuilder::new(init, |client| (cidp, dp(client)))
}

fn cidp_and_dp_relay(
	client: Arc<TFullClient<RTestBlock, RTestRtApi, TWasmExecutor>>,
) -> (
	impl CreateInherentDataProviders<RTestBlock, ()>,
	impl DigestCreator<RTestBlock>,
) {
	// Init timestamp instance_id
	let instance_id =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(6), None);

	let cidp = move |clone_client: Arc<TFullClient<RTestBlock, RTestRtApi, TWasmExecutor>>| {
		move |parent: H256, ()| {
			let client = clone_client.clone();
			let parent_header = client.header(parent).unwrap().unwrap();

			async move {
				let timestamp = FudgeInherentTimestamp::get_instance(instance_id)
					.expect("Instance is initialized. qed");

				let slot =
					sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						timestamp.current_time(),
						SlotDuration::from_millis(std::time::Duration::from_secs(6).as_millis() as u64),
					);

				let relay_para_inherent = FudgeDummyInherentRelayParachain::new(parent_header);
				Ok((timestamp, slot, relay_para_inherent))
			}
		}
	};

	let dp = move |parent, inherents| async move {
		let mut digest = sp_runtime::Digest::default();

		let babe = FudgeBabeDigest::<RTestBlock>::new();
		babe.append_digest(&mut digest, &parent, &inherents).await?;

		Ok(digest)
	};

	(cidp(client), dp)
}

fn default_relay_builder(
	handle: Handle,
	genesis: Storage,
) -> RelaychainBuilder<
	RTestBlock,
	RTestRtApi,
	TWasmExecutor,
	impl CreateInherentDataProviders<RTestBlock, ()>,
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
		+ pallet_timestamp::Config<Moment = u64>
		+ polkadot_runtime_parachains::initializer::Config,
{
	let mut provider =
		EnvProvider::<RTestBlock, RTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>::with_code(
			RCODE.unwrap(),
		)
		.unwrap();

	polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime>::default()
		.assimilate_storage(&mut storage)
		.unwrap();

	provider.insert_storage(storage).unwrap();

	let (client, backend) = provider
		.init_default(
			TestExec::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
			Box::new(manager.spawn_handle()),
		)
		.unwrap();
	let client = Arc::new(client);
	let clone_client = client.clone();
	// Init timestamp instance_id
	let instance_id =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(6), None)
			.unwrap();

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

				let mut init = crate::provider::initiator::default(handle);
				init.with_genesis(Box::new(state));
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
		let babe = FudgeBabeDigest::<RTestBlock>::new();
		let digest = babe.build_digest(parent, &inherents).await?;
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

	let mut relay_builder = default_relay_builder(Handle::current(), Storage::default());
	let para_id = Id::from(2001u32);
	let inherent_builder = relay_builder.inherent_builder(para_id.clone());

	// Init timestamp instance_id
	let instance_id_para =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(12), None)
			.unwrap();

	let cidp = Box::new(move |_| {
		move |_parent: H256, ()| {
			let inherent_builder_clone = inherent_builder.clone();
			async move {
				let timestamp = FudgeInherentTimestamp::get_instance(instance_id_para)
					.expect("Instance is initialized. qed");

				let slot =
					sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						timestamp.current_time(),
						SlotDuration::from_millis(std::time::Duration::from_secs(6).as_millis() as u64),
					);
				let inherent = inherent_builder_clone.parachain_inherent().await.unwrap();
				let relay_para_inherent = FudgeInherentParaParachain::new(inherent);
				Ok((timestamp, slot, relay_para_inherent))
			}
		}
	});
	let dp = Box::new(
		|clone_client: Arc<
			TFullClient<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
		>| {
			move |parent, inherents| {
				let client = clone_client.clone();

				async move {
					let slot_duration = sc_consensus_aura::slot_duration(&*client).unwrap();
					let aura = FudgeAuraDigest::<PTestBlock>::new(slot_duration);

					let digest = aura.build_digest(parent, &inherents).await?;
					Ok(digest)
				}
			}
		},
	);

	let mut builder = generate_default_setup_parachain(&manager, Storage::default(), cidp, dp);

	let para = FudgeParaChain {
		id: para_id,
		head: builder.head().unwrap(),
		code: builder.code().unwrap(),
	};

	let collator = builder.collator();

	relay_builder
		.onboard_para(para, Box::new(collator))
		.unwrap();

	let para_head = relay_builder
		.with_state(|| Heads::try_get(para_id).unwrap())
		.unwrap();
	let start_head = builder.head().unwrap();
	assert_eq!(builder.head().unwrap(), para_head);

	relay_builder.build_block().unwrap();

	let num_start = builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	// Importing the relay-blocks results in the collations being collected.
	// Hence: Parachain must build its block before
	relay_builder.import_block().unwrap();

	relay_builder.build_block().unwrap();
	// Imports collation
	relay_builder.import_block().unwrap();

	builder.import_block().unwrap();
	let num_after_one = builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	assert_eq!(num_start + 1, num_after_one);
	let para_head = relay_builder
		.with_state(|| Heads::try_get(para_id).unwrap())
		.unwrap();
	assert_eq!(builder.head().unwrap(), para_head);
	assert_ne!(builder.head().unwrap(), start_head);
}

#[tokio::test]
async fn xcm_is_transported() {
	super::utils::init_logs();
	let manager = TaskManager::new(Handle::current(), None).unwrap();

	let mut relay_builder =
		generate_default_setup_relay_chain::<RRuntime>(&manager, Storage::default());
	let para_id = Id::from(2001u32);
	let inherent_builder = relay_builder.inherent_builder(para_id.clone());
	// Init timestamp instance_id
	let instance_id_para =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(12), None)
			.unwrap();

	let cidp = Box::new(move |_| {
		move |_parent: H256, ()| {
			let inherent_builder_clone = inherent_builder.clone();
			async move {
				let timestamp = FudgeInherentTimestamp::get_instance(instance_id_para)
					.expect("Instance is initialized. qed");

				let slot =
					sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						timestamp.current_time(),
						SlotDuration::from_millis(std::time::Duration::from_secs(6).as_millis() as u64),
					);
				let inherent = inherent_builder_clone.parachain_inherent().await.unwrap();
				let relay_para_inherent = FudgeInherentParaParachain::new(inherent);
				Ok((timestamp, slot, relay_para_inherent))
			}
		}
	});
	let dp = Box::new(
		|clone_client: Arc<
			TFullClient<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
		>| {
			move |parent, inherents| {
				let client = clone_client.clone();

				async move {
					let slot_duration = sc_consensus_aura::slot_duration(&*client).unwrap();
					let aura = FudgeAuraDigest::<PTestBlock>::new(slot_duration);

					let digest = aura.build_digest(parent, &inherents).await?;
					Ok(digest)
				}
			}
		},
	);

	let builder = generate_default_setup_parachain(&manager, Storage::default(), cidp, dp);

	let para = FudgeParaChain {
		id: para_id,
		head: builder.head().unwrap(),
		code: builder.code().unwrap(),
	};

	relay_builder
		.onboard_para(para, Box::new(builder.collator()))
		.unwrap();
}
