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

///! Test for the ParachainBuilder
use crate::digest::{DigestCreator, FudgeBabeDigest};
use crate::inherent::{
	FudgeDummyInherentRelayParachain, FudgeInherentParaParachain, FudgeInherentTimestamp,
};
use crate::provider::EnvProvider;
use crate::RelayChainBuilder;
use crate::RelayChainTypes;
use crate::{FudgeParaChain, ParachainBuilder};
use centrifuge_runtime::{
	Block as PTestBlock, Runtime as PRuntime, RuntimeApi as PTestRtApi, WASM_BINARY as PCODE,
};
use polkadot_parachain::primitives::{HeadData, Id, ValidationCode};
use polkadot_runtime::{
	Block as RTestBlock, Runtime as RRuntime, RuntimeApi as RTestRtApi, WASM_BINARY as RCODE,
};
use polkadot_runtime_parachains::paras;
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

fn generate_default_setup_parachain<CIDP, DP>(
	handle: SpawnTaskHandle,
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
		Box::new(handle.clone()),
	);
	let client = Arc::new(client);
	let clone_client = client.clone();

	ParachainBuilder::<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>, _, _, _>::new(
		handle.clone(),
		backend,
		client,
		cidp(clone_client),
		dp,
	)
}

fn generate_default_setup_relay_chain<CIDP, DP, Runtime>(
	handle: SpawnTaskHandle,
	storage: Storage,
) -> RelayChainBuilder<
	RTestBlock,
	RTestRtApi,
	TestExec<sp_io::SubstrateHostFunctions>,
	CIDP,
	(),
	DP,
	Runtime,
	TFullBackend<RTestBlock>,
	TFullClient<RTestBlock, RTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
>
where
	CIDP: CreateInherentDataProviders<RTestBlock, ()> + 'static,
	DP: DigestCreator + 'static,
	Runtime: paras::Config + frame_system::Config,
{
	let mut provider =
		EnvProvider::<RTestBlock, RTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>::with_code(
			RCODE.unwrap(),
		);
	provider.insert_storage(storage);

	let (client, backend) = provider.init_default(
		TestExec::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
		Box::new(handle.clone()),
	);
	let client = Arc::new(client);
	let clone_client = client.clone();

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

					let timestamp =
						FudgeInherentTimestamp::new(0, sp_std::time::Duration::from_secs(6), None);

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
							timestamp.current_time(),
							std::time::Duration::from_secs(6),
						);

					let relay_para_inherent = FudgeDummyInherentRelayParachain::new(parent_header);
					Ok((timestamp, uncles, slot, relay_para_inherent))
				}
			}
		},
	);
	let dp = Box::new(move || async move {
		let mut digest = sp_runtime::Digest::default();

		let slot_duration = pallet_babe::Pallet::<Runtime>::slot_duration();
		digest.push(<DigestItem as CompatibleDigestItem>::babe_pre_digest(
			FudgeBabeDigest::pre_digest(
				FudgeInherentTimestamp::get_instance(0).current_time(),
				sp_std::time::Duration::from_millis(slot_duration),
			),
		));

		Ok(digest)
	});

	RelayChainBuilder::<
		RTestBlock,
		RTestRtApi,
		TestExec<sp_io::SubstrateHostFunctions>,
		_,
		_,
		_,
		Runtime,
	>::new(handle.clone(), backend, client, cidp(clone_client), dp)
}

#[tokio::test]
async fn parachain_creates_correct_inherents() {
	super::utils::init_logs();
	let manager = TaskManager::new(Handle::current(), None).unwrap();

	let mut relay_builder = generate_default_setup_relay_chain::<_, _, RRuntime>(
		manager.spawn_handle(),
		Storage::default(),
	);

	let mut relay_builder = Arc::new(relay_builder);
	let para_id = Id::from(2001u32);

	let cidp = Box::new(
		|clone_client: Arc<
			TFullClient<PTestBlock, PTestRtApi, TestExec<sp_io::SubstrateHostFunctions>>,
		>| {
			let relay_builder = relay_builder.clone();
			move |parent: H256, ()| {
				let client = clone_client.clone();
				let parent_header = client
					.header(&BlockId::Hash(parent.clone()))
					.unwrap()
					.unwrap();

				async move {
					let timestamp =
						FudgeInherentTimestamp::new(0, sp_std::time::Duration::from_secs(6), None);

					let slot =
                        sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
                            timestamp.current_time(),
                            std::time::Duration::from_secs(6),
                        );

					let relay_para_inherent = FudgeInherentParaParachain::new(
						relay_builder.parachain_inherent(para_id).unwrap(),
					);
					Ok((timestamp, slot, relay_para_inherent))
				}
			}
		},
	);
	let dp = Box::new(move || async move {
		// TODO: Do I need digests for Centrifuge
		let mut digest = sp_runtime::Digest::default();
		Ok(digest)
	});
	let mut builder =
		generate_default_setup_parachain(manager.spawn_handle(), Storage::default(), cidp, dp);

	builder.build_block().unwrap();
	builder.import_block();
}
