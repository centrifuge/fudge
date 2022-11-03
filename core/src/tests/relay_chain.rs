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

use fudge_test_runtime::WASM_BINARY as PARA_CODE;
use polkadot_parachain::primitives::{HeadData, Id, ValidationCode};
use polkadot_runtime::{Block as TestBlock, Runtime, RuntimeApi as TestRtApi, WASM_BINARY as CODE};
use sc_service::TFullClient;
use sp_api::BlockId;
use sp_consensus_babe::SlotDuration;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::{traits::Hash as _, Storage};
use sp_std::sync::Arc;
use tokio::runtime::Handle;

use crate::{
	digest::{DigestCreator, DigestProvider, FudgeBabeDigest},
	inherent::{FudgeDummyInherentRelayParachain, FudgeInherentTimestamp},
	provider::TWasmExecutor,
	FudgeParaChain, RelayChainTypes, RelaychainBuilder, StateProvider,
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
				.header(&BlockId::Hash(parent.clone()))
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

fn default_relay_builder(
	handle: Handle,
	genesis: Storage,
) -> RelaychainBuilder<
	TestBlock,
	TestRtApi,
	TWasmExecutor,
	impl CreateInherentDataProviders<TestBlock, ()>,
	(),
	impl DigestCreator<TestBlock>,
	Runtime,
> {
	let mut state = StateProvider::new(CODE.expect("Wasm is build. Qed."));
	state.insert_storage(genesis);

	let mut init = crate::provider::initiator::default(handle);
	init.with_genesis(Box::new(state));

	RelaychainBuilder::new(init, cidp_and_dp)
}

#[tokio::test]
async fn onboarding_parachain_works() {
	super::utils::init_logs();

	let mut builder = default_relay_builder(Handle::current(), Storage::default());
	let id = Id::from(2002u32);
	let code = ValidationCode(PARA_CODE.unwrap().to_vec());
	let code_hash = code.hash();
	let head = HeadData(Vec::new());
	let dummy_para = FudgeParaChain {
		id,
		head: head.clone(),
		code: code.clone(),
	};

	builder.onboard_para(dummy_para).unwrap();
	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let res = builder
		.with_state_at(BlockId::Number(1), || {
			let mut chains = RelayChainTypes::Parachains::get();
			chains.retain(|para_id| para_id == &id);
			let head = RelayChainTypes::Heads::get(&id).unwrap();
			let code_hash = RelayChainTypes::CurrentCodeHash::get(&id).unwrap();

			(chains[0], head, code_hash)
		})
		.unwrap();

	assert_eq!((id, head, code_hash), res);

	let head_new = HeadData(
		sp_runtime::traits::BlakeTwo256::hash(vec![1, 2, 4, 5, 6].as_slice())
			.as_ref()
			.to_vec(),
	);
	let dummy_para_new = FudgeParaChain {
		id,
		head: head_new.clone(),
		code: code.clone(),
	};

	builder.onboard_para(dummy_para_new).unwrap();
	builder.build_block().unwrap();
	builder.import_block().unwrap();

	let res = builder
		.with_state_at(BlockId::Number(2), || {
			let mut chains = RelayChainTypes::Parachains::get();
			chains.retain(|para_id| para_id == &id);
			let head = RelayChainTypes::Heads::get(&id).unwrap();
			let code_hash = RelayChainTypes::CurrentCodeHash::get(&id).unwrap();

			(chains[0], head, code_hash)
		})
		.unwrap();

	assert_eq!((id, head_new, code_hash), res);
}
