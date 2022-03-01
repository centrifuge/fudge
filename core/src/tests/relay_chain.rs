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

// TODO: test onbotd parachain with dummy data. i.e. see if storage of latest block is updated to contain wanted data

use crate::digest::{DigestCreator, FudgeBabeDigest};
use crate::inherent::{FudgeInherentRelayParachain, FudgeInherentTimestamp};
use crate::provider::EnvProvider;
use crate::FudgeParaChain;
use crate::RelayChainBuilder;
use crate::RelayChainTypes;
use centrifuge_runtime::WASM_BINARY as PARA_CODE;
use polkadot_parachain::primitives::{HeadData, Id, ValidationCode};
use polkadot_runtime::{Block as TestBlock, Runtime, RuntimeApi as TestRtApi, WASM_BINARY as CODE};
use polkadot_runtime_parachains::paras;
use sc_executor::sp_wasm_interface::HostFunctions;
use sc_executor::{WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{SpawnTaskHandle, TFullBackend, TFullClient, TaskManager};
use sp_api::BlockId;
use sp_consensus_babe::digests::CompatibleDigestItem;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::traits::Hash as _;
use sp_runtime::{DigestItem, Storage};
use sp_std::sync::Arc;
use tokio::runtime::Handle;

fn generate_default_setup_relay_chain<CIDP, DP, Runtime>(
	handle: SpawnTaskHandle,
	storage: Storage,
	cidp: Box<dyn FnOnce(Arc<TFullClient<TestBlock, TestRtApi, TestExec>>) -> CIDP>,
	dp: DP,
) -> RelayChainBuilder<
	TestBlock,
	TestRtApi,
	TestExec,
	CIDP,
	(),
	DP,
	Runtime,
	TFullBackend<TestBlock>,
	TFullClient<TestBlock, TestRtApi, TestExec>,
>
where
	CIDP: CreateInherentDataProviders<TestBlock, ()> + 'static,
	DP: DigestCreator<H256> + 'static,
	Runtime: paras::Config + frame_system::Config,
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

	RelayChainBuilder::<TestBlock, TestRtApi, TestExec, _, _, _, Runtime>::new(
		handle.clone(),
		backend,
		client,
		cidp(clone_client),
		dp,
	)
}

#[tokio::test]
async fn onboarding_parachain_works() {
	super::utils::init_logs();

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

					let timestamp =
						FudgeInherentTimestamp::new(0, sp_std::time::Duration::from_secs(6), None);

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
							timestamp.current_time(),
							std::time::Duration::from_secs(6),
						);

					let relay_para_inherent =
						FudgeInherentRelayParachain::new(parent_header, Vec::new());
					Ok((timestamp, uncles, slot, relay_para_inherent))
				}
			}
		},
	);
	let dp = Box::new(move || async move {
		let mut digest = sp_runtime::Digest::default();

		let slot_duration = pallet_babe::Pallet::<Runtime>::slot_duration();
		digest.push(<DigestItem<H256> as CompatibleDigestItem>::babe_pre_digest(
			FudgeBabeDigest::pre_digest(
				FudgeInherentTimestamp::get_instance(0).current_time(),
				sp_std::time::Duration::from_millis(slot_duration),
			),
		));

		Ok(digest)
	});
	let mut builder = generate_default_setup_relay_chain::<_, _, Runtime>(
		manager.spawn_handle(),
		Storage::default(),
		cidp,
		dp,
	);

	let id = Id::from(2002u32);
	let code = ValidationCode(PARA_CODE.unwrap().to_vec());
	let code_hash = code.hash();
	let head = HeadData(Vec::new());
	let dummy_para = FudgeParaChain {
		id,
		head: head.clone(),
		code: code.clone(),
	};

	//builder.build_block().unwrap();
	//builder.import_block();
	builder.onboard_para(dummy_para);
	builder.build_block().unwrap();
	builder.import_block();

	let res = builder
		.with_state(|| {
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

	builder.onboard_para(dummy_para_new);
	builder.build_block().unwrap();
	builder.import_block();

	let res = builder
		.with_state(|| {
			let mut chains = RelayChainTypes::Parachains::get();
			chains.retain(|para_id| para_id == &id);
			let head = RelayChainTypes::Heads::get(&id).unwrap();
			let code_hash = RelayChainTypes::CurrentCodeHash::get(&id).unwrap();

			(chains[0], head, code_hash)
		})
		.unwrap();

	assert_eq!((id, head_new, code_hash), res);
}