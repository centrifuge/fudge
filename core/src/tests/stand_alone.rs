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

use crate::digest::{DigestCreator, FudgeBabeDigest};
use crate::inherent::{FudgeInherentRelayParachain, FudgeInherentTimestamp};
use crate::provider::EnvProvider;
use crate::StandAloneBuilder;
use frame_benchmarking::account;
use fudge_utils::Signer;
use polkadot_runtime::{Block as TestBlock, Runtime, RuntimeApi as TestRtApi, WASM_BINARY as CODE};
use sc_executor::{WasmExecutionMethod, WasmExecutor as TestExec};
use sc_service::{SpawnTaskHandle, TFullBackend, TFullClient, TaskManager};
use sp_api::BlockId;
use sp_consensus_babe::digests::CompatibleDigestItem;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_keystore::SyncCryptoStore;
use sp_runtime::{AccountId32, CryptoTypeId, DigestItem, KeyTypeId, MultiAddress, Storage};
use sp_std::sync::Arc;
use tokio::runtime::Handle;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");
const CRYPTO_TYPE: CryptoTypeId = CryptoTypeId(*b"test");

fn generate_default_setup_stand_alone<CIDP, DP>(
	handle: SpawnTaskHandle,
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
	DP: DigestCreator + 'static,
{
	let mut provider =
		EnvProvider::<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>>::with_code(
			CODE.unwrap(),
		);
	provider.insert_storage(storage);

	let (client, backend) = provider.init_default(
		TestExec::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
		Box::new(handle.clone()),
	);
	let client = Arc::new(client);
	let clone_client = client.clone();

	StandAloneBuilder::<TestBlock, TestRtApi, TestExec<sp_io::SubstrateHostFunctions>, _, _, _>::new(
		handle.clone(),
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
			(AccountId32::default(), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();

	let cidp = Box::new(
		|clone_client: Arc<
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

	let dp = Box::new(move || async move { Ok(sp_runtime::Digest::default()) });

	let mut builder = generate_default_setup_stand_alone(manager.spawn_handle(), storage, cidp, dp);

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
	super::utils::init_logs();

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
	// install global collector configured based on RUST_LOG env var.
	super::utils::init_logs();

	let manager = TaskManager::new(Handle::current(), None).unwrap();
	let cidp = Box::new(
		|clone_client: Arc<
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

					let slot_duration = pallet_babe::Pallet::<Runtime>::slot_duration();
					let timestamp = FudgeInherentTimestamp::new(
						0,
						sp_std::time::Duration::from_millis(slot_duration),
						None,
					);
					let slot =
                        sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
                            timestamp.current_time(),
                            sp_std::time::Duration::from_millis(slot_duration),
                        );

					let relay_para_inherent =
						FudgeInherentRelayParachain::new(parent_header, Vec::new());
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
				FudgeInherentTimestamp::get_instance(0).current_time(),
				sp_std::time::Duration::from_millis(slot_duration),
			),
		));

		Ok(digest)
	});
	let mut builder =
		generate_default_setup_stand_alone(manager.spawn_handle(), Storage::default(), cidp, dp);

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);
}

#[tokio::test]
async fn building_relay_block_with_extrinsics_works() {
	super::utils::init_logs();

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
		|clone_client: Arc<
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
	let dp = Box::new(move || async move { Ok(sp_runtime::Digest::default()) });
	let _builder = generate_default_setup_stand_alone(manager.spawn_handle(), storage, cidp, dp);

	let _signer = Signer::new(key_store.into(), CRYPTO_TYPE, KEY_TYPE);
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

// NOTE ######### Building valid blocks an signing extr example ### FROM bin/node/cli/src/service.rs
/*

   fn test_sync() {
	   sp_tracing::try_init_simple();

	   let keystore_path = tempfile::tempdir().expect("Creates keystore path");
	   let keystore: SyncCryptoStorePtr =
		   Arc::new(LocalKeystore::open(keystore_path.path(), None).expect("Creates keystore"));
	   let alice: sp_consensus_babe::AuthorityId =
		   SyncCryptoStore::sr25519_generate_new(&*keystore, BABE, Some("//Alice"))
			   .expect("Creates authority pair")
			   .into();

	   let chain_spec = crate::chain_spec::tests::integration_test_config_with_single_authority();

	   // For the block factory
	   let mut slot = 1u64;

	   // For the extrinsics factory
	   let bob = Arc::new(AccountKeyring::Bob.pair());
	   let charlie = Arc::new(AccountKeyring::Charlie.pair());
	   let mut index = 0;

	   sc_service_test::sync(
		   chain_spec,
		   |config| {
			   let mut setup_handles = None;
			   let NewFullBase { task_manager, client, network, transaction_pool, .. } =
				   new_full_base(
					   config,
					   |block_import: &sc_consensus_babe::BabeBlockImport<Block, _, _>,
						babe_link: &sc_consensus_babe::BabeLink<Block>| {
						   setup_handles = Some((block_import.clone(), babe_link.clone()));
					   },
				   )?;

			   let node = sc_service_test::TestNetComponents::new(
				   task_manager,
				   client,
				   network,
				   transaction_pool,
			   );
			   Ok((node, setup_handles.unwrap()))
		   },
		   |service, &mut (ref mut block_import, ref babe_link)| {
			   let parent_id = BlockId::number(service.client().chain_info().best_number);
			   let parent_header = service.client().header(&parent_id).unwrap().unwrap();
			   let parent_hash = parent_header.hash();
			   let parent_number = *parent_header.number();

			   futures::executor::block_on(service.transaction_pool().maintain(
				   ChainEvent::NewBestBlock { hash: parent_header.hash(), tree_route: None },
			   ));

			   let mut proposer_factory = sc_basic_authorship::ProposerFactory::new(
				   service.spawn_handle(),
				   service.client(),
				   service.transaction_pool(),
				   None,
				   None,
			   );

			   let mut digest = Digest::default();

			   // even though there's only one authority some slots might be empty,
			   // so we must keep trying the next slots until we can claim one.
			   let (babe_pre_digest, epoch_descriptor) = loop {
				   let epoch_descriptor = babe_link
					   .epoch_changes()
					   .shared_data()
					   .epoch_descriptor_for_child_of(
						   descendent_query(&*service.client()),
						   &parent_hash,
						   parent_number,
						   slot.into(),
					   )
					   .unwrap()
					   .unwrap();

				   let epoch = babe_link
					   .epoch_changes()
					   .shared_data()
					   .epoch_data(&epoch_descriptor, |slot| {
						   sc_consensus_babe::Epoch::genesis(&babe_link.config(), slot)
					   })
					   .unwrap();

				   if let Some(babe_pre_digest) =
					   sc_consensus_babe::authorship::claim_slot(slot.into(), &epoch, &keystore)
						   .map(|(digest, _)| digest)
				   {
					   break (babe_pre_digest, epoch_descriptor)
				   }

				   slot += 1;
			   };

			   let inherent_data = (
				   sp_timestamp::InherentDataProvider::new(
					   std::time::Duration::from_millis(SLOT_DURATION * slot).into(),
				   ),
				   sp_consensus_babe::inherents::InherentDataProvider::new(slot.into()),
			   )
				   .create_inherent_data()
				   .expect("Creates inherent data");

			   digest.push(<DigestItem as CompatibleDigestItem>::babe_pre_digest(babe_pre_digest));

			   let new_block = futures::executor::block_on(async move {
				   let proposer = proposer_factory.init(&parent_header).await;
				   proposer
					   .unwrap()
					   .propose(inherent_data, digest, std::time::Duration::from_secs(1), None)
					   .await
			   })
			   .expect("Error making test block")
			   .block;

			   let (new_header, new_body) = new_block.deconstruct();
			   let pre_hash = new_header.hash();
			   // sign the pre-sealed hash of the block and then
			   // add it to a digest item.
			   let to_sign = pre_hash.encode();
			   let signature = SyncCryptoStore::sign_with(
				   &*keystore,
				   sp_consensus_babe::AuthorityId::ID,
				   &alice.to_public_crypto_pair(),
				   &to_sign,
			   )
			   .unwrap()
			   .unwrap()
			   .try_into()
			   .unwrap();
			   let item = <DigestItem as CompatibleDigestItem>::babe_seal(signature);
			   slot += 1;

			   let mut params = BlockImportParams::new(BlockOrigin::File, new_header);
			   params.post_digests.push(item);
			   params.body = Some(new_body);
			   params.intermediates.insert(
				   Cow::from(INTERMEDIATE_KEY),
				   Box::new(BabeIntermediate::<Block> { epoch_descriptor }) as Box<_>,
			   );
			   params.fork_choice = Some(ForkChoiceStrategy::LongestChain);

			   futures::executor::block_on(block_import.import_block(params, Default::default()))
				   .expect("error importing test block");
		   },
		   |service, _| {
			   let amount = 5 * CENTS;
			   let to: Address = AccountPublic::from(bob.public()).into_account().into();
			   let from: Address = AccountPublic::from(charlie.public()).into_account().into();
			   let genesis_hash = service.client().block_hash(0).unwrap().unwrap();
			   let best_block_id = BlockId::number(service.client().chain_info().best_number);
			   let (spec_version, transaction_version) = {
				   let version = service.client().runtime_version_at(&best_block_id).unwrap();
				   (version.spec_version, version.transaction_version)
			   };
			   let signer = charlie.clone();

			   let function =
				   Call::Balances(BalancesCall::transfer { dest: to.into(), value: amount });

			   let check_non_zero_sender = frame_system::CheckNonZeroSender::new();
			   let check_spec_version = frame_system::CheckSpecVersion::new();
			   let check_tx_version = frame_system::CheckTxVersion::new();
			   let check_genesis = frame_system::CheckGenesis::new();
			   let check_era = frame_system::CheckEra::from(Era::Immortal);
			   let check_nonce = frame_system::CheckNonce::from(index);
			   let check_weight = frame_system::CheckWeight::new();
			   let tx_payment = pallet_asset_tx_payment::ChargeAssetTxPayment::from(0, None);
			   let extra = (
				   check_non_zero_sender,
				   check_spec_version,
				   check_tx_version,
				   check_genesis,
				   check_era,
				   check_nonce,
				   check_weight,
				   tx_payment,
			   );
			   let raw_payload = SignedPayload::from_raw(
				   function,
				   extra,
				   ((), spec_version, transaction_version, genesis_hash, genesis_hash, (), (), ()),
			   );
			   let signature = raw_payload.using_encoded(|payload| signer.sign(payload));
			   let (function, extra, _) = raw_payload.deconstruct();
			   index += 1;
			   UncheckedExtrinsic::new_signed(function, from.into(), signature.into(), extra)
				   .into()
		   },
	   );
*/

#[tokio::test]
async fn build_relay_block_works_and_mut_is_build_upon() {
	super::utils::init_logs();

	let manager = TaskManager::new(Handle::current(), None).unwrap();
	let cidp = Box::new(
		|clone_client: Arc<
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

					let slot_duration = pallet_babe::Pallet::<Runtime>::slot_duration();
					let timestamp = FudgeInherentTimestamp::new(
						0,
						sp_std::time::Duration::from_millis(slot_duration),
						None,
					);
					let slot =
                        sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
                            timestamp.current_time(),
                            sp_std::time::Duration::from_millis(slot_duration),
                        );

					let relay_para_inherent =
						FudgeInherentRelayParachain::new(parent_header, Vec::new());
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
				FudgeInherentTimestamp::get_instance(0).current_time(),
				sp_std::time::Duration::from_millis(slot_duration),
			),
		));

		Ok(digest)
	});

	let storage = pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(account("test", 0, 0), 10_000_000_000_000u128),
			(AccountId32::default(), 10_000_000_000_000u128),
		],
	}
	.build_storage()
	.unwrap();

	let mut builder = generate_default_setup_stand_alone(manager.spawn_handle(), storage, cidp, dp);

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);

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

	let num_before = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	builder.build_block().unwrap();
	builder.import_block();

	let num_after = builder
		.with_state(|| frame_system::Pallet::<Runtime>::block_number())
		.unwrap();

	assert_eq!(num_before + 1, num_after);

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
