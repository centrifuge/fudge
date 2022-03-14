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

use crate::builder::parachain::FudgeParaChain;
use crate::digest::DigestCreator;
use crate::inherent::ArgsProvider;
use crate::{
	builder::core::{Builder, Operation},
	types::{Bytes, StoragePair},
};
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_relay_chain_local::RelayChainLocal;
use parking_lot::Mutex;
use polkadot_core_primitives::Block as PBlock;
use polkadot_parachain::primitives::{Id, ValidationCodeHash};
use polkadot_primitives::v1::{OccupiedCoreAssumption, PersistedValidationData};
use polkadot_primitives::v2::ParachainHost;
use polkadot_runtime_parachains::{paras, ParaLifecycle};
use sc_client_api::{
	AuxStore, Backend as BackendT, BlockOf, BlockchainEvents, HeaderBackend, UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TFullBackend, TFullClient};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi, StorageProof};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, NoNetwork, Proposal};
use sp_consensus_babe::BabeApi;
use sp_core::traits::CodeExecutor;
use sp_core::H256;
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
use types::*;

/// Recreating private storage types for easier handling storage access
pub mod types {
	use frame_support::traits::StorageInstance;
	use frame_support::{
		storage::types::{StorageMap, StorageValue, ValueQuery},
		Identity, Twox64Concat,
	};
	use polkadot_parachain::primitives::{
		HeadData, Id as ParaId, ValidationCode, ValidationCodeHash,
	};
	use polkadot_runtime_parachains::ParaLifecycle;

	pub struct ParaLifecyclesPrefix;
	impl StorageInstance for ParaLifecyclesPrefix {
		fn pallet_prefix() -> &'static str {
			"ParaLifecycles"
		}

		const STORAGE_PREFIX: &'static str = "Parachains";
	}
	pub type ParaLifecycles = StorageMap<ParaLifecyclesPrefix, Twox64Concat, ParaId, ParaLifecycle>;

	pub struct ParachainsPrefix;
	impl StorageInstance for ParachainsPrefix {
		fn pallet_prefix() -> &'static str {
			"Paras"
		}

		const STORAGE_PREFIX: &'static str = "Parachains";
	}
	pub type Parachains = StorageValue<ParachainsPrefix, Vec<ParaId>, ValueQuery>;

	pub struct HeadsPrefix;
	impl StorageInstance for HeadsPrefix {
		fn pallet_prefix() -> &'static str {
			"Paras"
		}

		const STORAGE_PREFIX: &'static str = "Heads";
	}
	pub type Heads = StorageMap<HeadsPrefix, Twox64Concat, ParaId, HeadData>;

	pub struct CurrentCodeHashPrefix;
	impl StorageInstance for CurrentCodeHashPrefix {
		fn pallet_prefix() -> &'static str {
			"Paras"
		}

		const STORAGE_PREFIX: &'static str = "CurrentCodeHash";
	}
	pub type CurrentCodeHash =
		StorageMap<CurrentCodeHashPrefix, Twox64Concat, ParaId, ValidationCodeHash>;

	pub struct CodeByHashPrefix;
	impl StorageInstance for CodeByHashPrefix {
		fn pallet_prefix() -> &'static str {
			"Paras"
		}

		const STORAGE_PREFIX: &'static str = "CodeByHash";
	}
	pub type CodeByHash =
		StorageMap<CodeByHashPrefix, Identity, ValidationCodeHash, ValidationCode>;

	pub struct CodeByHashRefsPrefix;
	impl StorageInstance for CodeByHashRefsPrefix {
		fn pallet_prefix() -> &'static str {
			"Paras"
		}

		const STORAGE_PREFIX: &'static str = "CodeByHashRefs";
	}
	pub type CodeByHashRefs =
		StorageMap<CodeByHashRefsPrefix, Identity, ValidationCodeHash, u32, ValueQuery>;

	pub struct PastCodeHashPrefix;
	impl StorageInstance for PastCodeHashPrefix {
		fn pallet_prefix() -> &'static str {
			"Paras"
		}

		const STORAGE_PREFIX: &'static str = "PastCodeHash";
	}
	#[allow(type_alias_bounds)]
	pub type PastCodeHash<T: frame_system::Config> =
		StorageMap<PastCodeHashPrefix, Twox64Concat, (ParaId, T::BlockNumber), ValidationCodeHash>;
}

pub struct RelayChainBuilder<
	Block: BlockT,
	RtApi,
	Exec,
	CIDP,
	ExtraArgs,
	DP,
	Runtime,
	B = Backend<Block>,
	C = TFullClient<Block, RtApi, Exec>,
> {
	builder: Builder<Block, RtApi, Exec, B, C>,
	cidp: CIDP,
	dp: DP,
	next: Option<(Block, StorageProof)>,
	imports: Vec<(Block, StorageProof)>,
	handle: SpawnTaskHandle,
	_phantom: PhantomData<(ExtraArgs, Runtime)>,
}

impl<RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, C>
	RelayChainBuilder<PBlock, RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, TFullBackend<PBlock>, C>
where
	RtApi: ConstructRuntimeApi<PBlock, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	CIDP: CreateInherentDataProviders<PBlock, ExtraArgs> + Send + Sync + 'static,
	CIDP::InherentDataProviders: Send,
	DP: DigestCreator,
	ExtraArgs: ArgsProvider<ExtraArgs>,
	Runtime: paras::Config + frame_system::Config,
	C::Api: BlockBuilder<PBlock>
		+ ParachainHost<PBlock>
		+ BabeApi<PBlock>
		+ ApiExt<PBlock, StateBackend = <TFullBackend<PBlock> as BackendT<PBlock>>::State>,
	C: 'static
		+ ProvideRuntimeApi<PBlock>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<PBlock>
		+ BlockchainEvents<PBlock>
		+ HeaderBackend<PBlock>
		+ BlockImport<PBlock>
		+ CallApiAt<PBlock>
		+ sc_block_builder::BlockBuilderProvider<TFullBackend<PBlock>, PBlock, C>,
{
	pub async fn parachain_inherent(&self, para_id: Id) -> Option<ParachainInherentData> {
		let parent = self.builder.latest_block();
		let relay_interface = RelayChainLocal::new(
			self.builder.client(),
			self.builder.backend(),
			Arc::new(Mutex::new(Box::new(NoNetwork {}))),
			None,
		);
		let persisted_validation_data = self.persisted_validation_data(parent, para_id);

		ParachainInherentData::create_at(
			parent,
			&relay_interface,
			&persisted_validation_data,
			para_id,
		)
		.await
	}
}

impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, B, C>
	RelayChainBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, B, C>
where
	B: BackendT<Block> + 'static,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	CIDP: CreateInherentDataProviders<Block, ExtraArgs> + Send + Sync + 'static,
	CIDP::InherentDataProviders: Send,
	DP: DigestCreator,
	ExtraArgs: ArgsProvider<ExtraArgs>,
	Runtime: paras::Config + frame_system::Config,
	C::Api: BlockBuilder<Block> + ParachainHost<Block> + ApiExt<Block, StateBackend = B::State>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ BlockchainEvents<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>
		+ sc_block_builder::BlockBuilderProvider<B, Block, C>,
{
	pub fn new(
		handle: SpawnTaskHandle,
		backend: Arc<B>,
		client: Arc<C>,
		cidp: CIDP,
		dp: DP,
	) -> Self {
		Self {
			builder: Builder::new(backend, client),
			cidp,
			dp,
			next: None,
			imports: Vec::new(),
			handle,
			_phantom: Default::default(),
		}
	}

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
		self.builder.append_extrinsic(xt);
		self
	}

	pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
		xts.into_iter().for_each(|xt| {
			self.builder.append_extrinsic(xt);
		});
		self
	}

	pub fn append_transition(&mut self, aux: StoragePair) -> &mut Self {
		self.builder.append_transition(aux);
		self
	}

	pub fn append_transitions(&mut self, auxs: Vec<StoragePair>) -> &mut Self {
		auxs.into_iter().for_each(|aux| {
			self.builder.append_transition(aux);
		});
		self
	}

	pub fn append_xcm(&mut self, _xcm: Bytes) -> &mut Self {
		todo!()
	}

	pub fn append_xcms(&mut self, _xcms: Vec<Bytes>) -> &mut Self {
		todo!()
	}

	fn persisted_validation_data(
		&self,
		parent: Block::Hash,
		para_id: Id,
	) -> PersistedValidationData {
		let client = self.builder.client();
		let api = client.runtime_api();
		// We use the runtime api as if the block was enacted
		api.persisted_validation_data(
			&BlockId::Hash(parent),
			para_id,
			OccupiedCoreAssumption::TimedOut,
		)
		.unwrap()
		.unwrap()
	}

	pub fn onboard_para(&mut self, para: FudgeParaChain) -> &mut Self {
		self.with_mut_state(|| {
			let FudgeParaChain { id, head, code } = para;
			let current_block = frame_system::Pallet::<Runtime>::block_number();
			let code_hash = code.hash();

			Parachains::try_mutate::<(), (), _>(|paras| {
				if !paras.contains(&id) {
					paras.push(id);
					paras.sort();
				}
				Ok(())
			})
			.unwrap();

			let curr_code_hash = if let Some(curr_code_hash) = CurrentCodeHash::get(&id) {
				PastCodeHash::<Runtime>::insert(&(id, current_block), curr_code_hash);
				curr_code_hash
			} else {
				ValidationCodeHash::from(H256::zero())
			};

			if curr_code_hash != code_hash {
				CurrentCodeHash::insert(&id, code_hash);
				CodeByHash::insert(code_hash, code);
				CodeByHashRefs::mutate(code_hash, |refs| {
					if *refs == 0 {
						*refs += 1;
					}
				});
			}

			ParaLifecycles::try_mutate::<_, (), (), _>(id, |para_lifecylce| {
				if let Some(lifecycle) = para_lifecylce.as_mut() {
					*lifecycle = ParaLifecycle::Parachain;
				} else {
					*para_lifecylce = Some(ParaLifecycle::Parachain);
				}

				Ok(())
			})
			.unwrap();

			Heads::insert(&id, head);
		})
		.unwrap();

		self
	}

	pub fn build_block(&mut self) -> Result<Block, ()> {
		assert!(self.next.is_none());

		let provider = self
			.with_state(|| {
				futures::executor::block_on(self.cidp.create_inherent_data_providers(
					self.builder.latest_block(),
					ExtraArgs::extra(),
				))
				.unwrap()
			})
			.unwrap();

		let digest = self
			.with_state(|| futures::executor::block_on(self.dp.create_digest()).unwrap())
			.unwrap();
		// NOTE: Need to crate inherents AFTER digest, as timestamp updates itself
		//       afterwards
		let inherent = provider.create_inherent_data().unwrap();

		let Proposal { block, proof, .. } = self.builder.build_block(
			self.handle.clone(),
			inherent,
			digest,
			Duration::from_secs(60),
			6_000_000,
		);
		self.next = Some((block.clone(), proof));

		Ok(block)
	}

	pub fn import_block(&mut self) -> &mut Self {
		let (block, proof) = self.next.take().unwrap();
		let (header, body) = block.clone().deconstruct();
		let mut params = BlockImportParams::new(BlockOrigin::ConsensusBroadcast, header);
		params.body = Some(body);
		params.finalized = true;
		params.fork_choice = Some(ForkChoiceStrategy::Custom(true));

		self.builder.import_block(params).unwrap();
		self.imports.push((block, proof));
		self
	}

	pub fn imports(&self) -> Vec<(Block, StorageProof)> {
		self.imports.clone()
	}

	pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, None, exec)
	}

	pub fn with_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, Some(at), exec)
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		assert!(self.next.is_none());

		self.builder.with_state(Operation::Commit, None, exec)
	}

	/// Mutating past states not supported yet...
	fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		assert!(self.next.is_none());

		self.builder.with_state(Operation::Commit, Some(at), exec)
	}
}
