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

use codec::Encode;
use cumulus_primitives_core::{Instruction, OriginKind, Transact, Xcm};
use frame_support::traits::GenesisBuild;
use fudge_test_runtime::{
	AuraId, Block as PTestBlock, Runtime as PRuntime, RuntimeApi as PTestRtApi,
	RuntimeCall as PRuntimeCall, RuntimeEvent as PRuntimeEvent, WASM_BINARY as PCODE,
};
use polkadot_core_primitives::Block as RTestBlock;
use polkadot_parachain::primitives::Id;
use polkadot_primitives::{AssignmentId, AuthorityDiscoveryId, ValidatorId};
use polkadot_runtime::{Runtime as RRuntime, RuntimeApi as RTestRtApi, WASM_BINARY as RCODE};
use polkadot_runtime_parachains::{configuration, configuration::HostConfiguration, dmp};
use sc_service::{TFullBackend, TFullClient};
use sp_consensus_babe::SlotDuration;
use sp_core::{crypto::AccountId32, ByteArray, H256};
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::{
	traits::{BlakeTwo256, Hash},
	Storage,
};
use sp_std::sync::Arc;
use tokio::runtime::Handle;
use xcm::{v3::Weight, VersionedXcm};

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

const PARA_ID: u32 = 2001;

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
	let mut state: StateProvider<TFullBackend<RTestBlock>, RTestBlock> =
		StateProvider::empty_default(Some(PCODE.expect("Wasm is build. Qed."))).unwrap();

	state
		.insert_storage(
			pallet_aura::GenesisConfig::<PRuntime> {
				authorities: vec![AuraId::from(sp_core::sr25519::Public([0u8; 32]))],
			}
			.build_storage()
			.unwrap(),
		)
		.unwrap();
	state
		.insert_storage(
			<parachain_info::GenesisConfig as GenesisBuild<PRuntime>>::build_storage(
				&parachain_info::GenesisConfig {
					parachain_id: Id::from(PARA_ID),
				},
			)
			.unwrap(),
		)
		.unwrap();
	state.insert_storage(genesis).unwrap();

	let mut init = crate::provider::initiator::default(handle);

	init.with_genesis(Box::new(state));

	// Init timestamp instance_id
	let instance_id_para =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(12), None)
			.unwrap();

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
				>::new(&*client)
				.unwrap();

				let digest = aura.build_digest(parent, &inherents).await?;
				Ok(digest)
			}
		}
	};

	ParachainBuilder::new(init, |client| (cidp, dp(client))).unwrap()
}

fn cidp_and_dp_relay(
	client: Arc<TFullClient<RTestBlock, RTestRtApi, TWasmExecutor>>,
) -> (
	impl CreateInherentDataProviders<RTestBlock, ()>,
	impl DigestCreator<RTestBlock>,
) {
	// Init timestamp instance_id
	let instance_id =
		FudgeInherentTimestamp::create_instance(sp_std::time::Duration::from_secs(6), None)
			.unwrap();

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
		babe.append_digest(parent, &mut digest, &inherents).await?;

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
	let mut state: StateProvider<TFullBackend<RTestBlock>, RTestBlock> =
		StateProvider::empty_default(Some(RCODE.expect("Wasm is build. Qed."))).unwrap();
	let mut configuration = configuration::GenesisConfig::<RRuntime>::default();

	let mut host_config = HostConfiguration::<u32>::default();
	host_config.max_downward_message_size = 1024;

	configuration.config = host_config;

	state
		.insert_storage(configuration.build_storage().unwrap())
		.unwrap();
	state
		.insert_storage(
			pallet_session::GenesisConfig::<RRuntime> {
				keys: vec![(
					AccountId32::from_slice([0u8; 32].as_slice()).unwrap(),
					AccountId32::from_slice([0u8; 32].as_slice()).unwrap(),
					polkadot_runtime::SessionKeys {
						grandpa: pallet_grandpa::AuthorityId::from_slice([0u8; 32].as_slice())
							.unwrap(),
						babe: pallet_babe::AuthorityId::from_slice([0u8; 32].as_slice()).unwrap(),
						im_online: pallet_im_online::sr25519::AuthorityId::from_slice(
							[0u8; 32].as_slice(),
						)
						.unwrap(),
						para_validator: ValidatorId::from_slice([0u8; 32].as_slice()).unwrap(),
						para_assignment: AssignmentId::from_slice([0u8; 32].as_slice()).unwrap(),
						authority_discovery: AuthorityDiscoveryId::from_slice([0u8; 32].as_slice())
							.unwrap(),
					},
				)],
			}
			.build_storage()
			.unwrap(),
		)
		.unwrap();
	state.insert_storage(genesis).unwrap();

	let mut init = crate::provider::initiator::default(handle);
	init.with_genesis(Box::new(state));

	RelaychainBuilder::new(init, cidp_and_dp_relay).unwrap()
}

#[tokio::test]
async fn parachain_creates_correct_inherents() {
	super::utils::init_logs();

	let mut relay_builder = default_relay_builder(Handle::current(), Storage::default());
	let para_id = Id::from(2001u32);
	let inherent_builder = relay_builder.inherent_builder(para_id.clone());
	let mut para_builder =
		default_para_builder(Handle::current(), Storage::default(), inherent_builder);
	let collator = para_builder.collator();

	let para = FudgeParaChain {
		id: para_id,
		head: para_builder.head().unwrap(),
		code: para_builder.code().unwrap(),
	};

	relay_builder
		.onboard_para(para, Box::new(collator))
		.unwrap();

	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	relay_builder.build_block().unwrap();

	let num_start = para_builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	para_builder.build_block().unwrap();

	relay_builder.import_block().unwrap();
	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	para_builder.import_block().unwrap();

	let num_after_one = para_builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	assert_eq!(num_start + 1, num_after_one); // this should be fine after populating session_info account keys

	relay_builder.build_block().unwrap();
	para_builder.build_block().unwrap();

	relay_builder.import_block().unwrap();
	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	para_builder.import_block().unwrap();

	let num_after_two = para_builder
		.with_state(|| frame_system::Pallet::<PRuntime>::block_number())
		.unwrap();

	assert_eq!(num_start + 2, num_after_two);

	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	let para_head = relay_builder
		.with_state(|| Heads::try_get(para_id).unwrap())
		.unwrap();

	assert_eq!(para_builder.head().unwrap(), para_head);
}

#[tokio::test]
async fn parachain_can_process_downward_message() {
	super::utils::init_logs();

	let mut relay_builder = default_relay_builder(Handle::current(), Storage::default());
	let para_id = Id::from(PARA_ID);
	let inherent_builder = relay_builder.inherent_builder(para_id.clone());
	let mut para_builder =
		default_para_builder(Handle::current(), Storage::default(), inherent_builder);
	let collator = para_builder.collator();

	let para = FudgeParaChain {
		id: para_id,
		head: para_builder.head().unwrap(),
		code: para_builder.code().unwrap(),
	};

	relay_builder
		.onboard_para(para, Box::new(collator))
		.unwrap();

	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	let remark = vec![0, 1, 2, 3];

	let call = PRuntimeCall::System(frame_system::Call::remark_with_event {
		remark: remark.clone(),
	});

	let instruction: Instruction<PRuntimeCall> = Transact {
		origin_kind: OriginKind::SovereignAccount,
		require_weight_at_most: Weight::from_ref_time(200_000_000),
		call: call.encode().into(),
	};

	let xcm = VersionedXcm::from(Xcm(vec![instruction]));

	relay_builder
		.with_mut_state(|| {
			let config = <configuration::Pallet<RRuntime>>::config();
			<dmp::Pallet<RRuntime>>::queue_downward_message(&config, para_id, xcm.encode())
				.map_err(|_| ())
				.unwrap();
		})
		.unwrap();

	relay_builder.build_block().unwrap();

	para_builder.build_block().unwrap();

	relay_builder.import_block().unwrap();
	relay_builder.build_block().unwrap();
	relay_builder.import_block().unwrap();

	para_builder.import_block().unwrap();

	let events = para_builder
		.with_state(|| frame_system::Pallet::<PRuntime>::events())
		.unwrap();

	events
		.iter()
		.find(|e| match e.event {
			PRuntimeEvent::System(frame_system::Event::<PRuntime>::Remarked { hash, .. })
				if hash == BlakeTwo256::hash(&remark) =>
			{
				true
			}
			_ => false,
		})
		.unwrap();
}
