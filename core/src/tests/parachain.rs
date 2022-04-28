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
	impl DigestCreator<PTestBlock>,
> {
	let mut state = StateProvider::new(PCODE.expect("Wasm is build. Qed."));
	state.insert_storage(
		pallet_aura::GenesisConfig::<PRuntime> {
			authorities: vec![AuraId::from(sp_core::sr25519::Public([0u8; 32]))],
		}
		.build_storage()
		.unwrap(),
	);
	state.insert_storage(genesis);

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
	impl DigestCreator<RTestBlock>,
	RRuntime,
> {
	let mut state = StateProvider::new(RCODE.expect("Wasm is build. Qed."));
	state.insert_storage(
		polkadot_runtime_parachains::configuration::GenesisConfig::<RRuntime>::default()
			.build_storage()
			.unwrap(),
	);
	state.insert_storage(genesis);

	let mut init = crate::provider::initiator::default(handle);
	init.with_genesis(Box::new(state));

	RelaychainBuilder::new(init, cidp_and_dp_relay)
}

#[tokio::test]
async fn parachain_creates_correct_inherents() {
	super::utils::init_logs();

	let mut relay_builder = default_relay_builder(Handle::current(), Storage::default());
	let para_id = Id::from(2001u32);
	let inherent_builder = relay_builder.inherent_builder(para_id.clone());
	let mut builder = default_para_builder(Handle::current(), Storage::default(), inherent_builder);

	let para = FudgeParaChain {
		id: para_id,
		head: builder.head(),
		code: builder.code(),
	};
	relay_builder
		.onboard_para(para, Box::new(builder.collator()))
		.unwrap();

	let para_head = relay_builder
		.with_state(|| Heads::try_get(para_id).unwrap())
		.unwrap();
	assert_eq!(builder.head(), para_head);

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
	assert_eq!(builder.head(), para_head);
}
