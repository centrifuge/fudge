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

/// The logging target.
// TODO: Make this more adaptable for giving a parachain a name
const LOG_TARGET: &str = "fudge-parachain";

use codec::{Decode, Encode};
use cumulus_primitives_core::{
	CollationInfo, CollectCollationInfo, ParachainBlockData, PersistedValidationData,
};
use polkadot_node_primitives::{Collation, MaybeCompressedPoV, PoV};
use polkadot_parachain::primitives::{BlockData, HeadData, Id, ValidationCode};
use sc_client_api::{
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, HeaderBackend, UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TFullClient, TaskManager};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, HashFor, ProvideRuntimeApi, StorageProof};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, Proposal};
use sp_core::traits::CodeExecutor;
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, BlockIdTo, Header},
};
use sp_std::{
	marker::PhantomData,
	sync::{Arc, Mutex},
	time::Duration,
};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;

use crate::{
	builder::{
		core::{Builder, Operation},
		relay_chain::{CollationBuilder, CollationJudgement},
	},
	digest::DigestCreator,
	inherent::ArgsProvider,
	types::StoragePair,
	PoolState,
};

pub struct FudgeParaBuild {
	pub parent_head: HeadData,
	pub block: BlockData,
	pub code: ValidationCode,
}

pub struct FudgeParaChain {
	pub id: Id,
	pub head: HeadData,
	pub code: ValidationCode,
}

pub struct FudgeCollator<Block, C, B> {
	client: Arc<C>,
	backend: Arc<B>,
	next_block: Arc<Mutex<Option<(Block, StorageProof)>>>,
	next_import: Arc<Mutex<Option<(Block, StorageProof)>>>,
}

impl<Block, C, B> CollationBuilder for FudgeCollator<Block, C, B>
where
	Block: BlockT,
	B: BackendT<Block>,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: CollectCollationInfo<Block>,
{
	fn collation(&self, validation_data: PersistedValidationData) -> Option<Collation> {
		self.collation(validation_data)
	}

	fn judge(&self, judgement: CollationJudgement) {
		match judgement {
			CollationJudgement::Approved => self.approve(),
			CollationJudgement::Rejected => self.reject(),
		}
	}
}

impl<Block, C, B> FudgeCollator<Block, C, B>
where
	Block: BlockT,
	B: BackendT<Block>,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: CollectCollationInfo<Block>,
{
	pub fn new(
		client: Arc<C>,
		backend: Arc<B>,
		next_block: Arc<Mutex<Option<(Block, StorageProof)>>>,
		next_import: Arc<Mutex<Option<(Block, StorageProof)>>>,
	) -> Self {
		Self{
			client,
			backend,
			next_block,
			next_import,
		}
	}

	fn fetch_collation_info(
		&self,
		block_hash: Block::Hash,
		header: &Block::Header,
	) -> Result<Option<CollationInfo>, sp_api::ApiError> {
		let runtime_api = self.client.runtime_api();
		let block_id = BlockId::Hash(block_hash);

		let api_version =
			match runtime_api.api_version::<dyn CollectCollationInfo<Block>>(&block_id)? {
				Some(version) => version,
				None => {
					tracing::error!(
						target: LOG_TARGET,
						"Could not fetch `CollectCollationInfo` runtime api version."
					);
					return Ok(None);
				}
			};

		let collation_info = if api_version < 2 {
			#[allow(deprecated)]
			runtime_api
				.collect_collation_info_before_version_2(&block_id)?
				.into_latest(header.encode().into())
		} else {
			runtime_api.collect_collation_info(&block_id, header)?
		};

		Ok(Some(collation_info))
	}

	pub fn collation(&self, validation_data: PersistedValidationData) -> Option<Collation> {
		let at = BlockId::Hash(self.client.info().best_hash);
		let _state = self.backend.state_at(at.clone()).ok()?;
		//ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state)
		//	.execute_with(|| self.create_collation(validation_data))
		self.create_collation(validation_data)
	}

	fn create_collation(&self, validation_data: PersistedValidationData) -> Option<Collation> {
		let locked = self.next_block.lock().ok()?;
		if let Some((block, proof)) = &*locked {
			let last_head = match Block::Header::decode(&mut &validation_data.parent_head.0[..]) {
				Ok(x) => x,
				Err(e) => {
					tracing::error!(
						target: LOG_TARGET,
						error = ?e,
						"Could not decode the head data."
					);
					return None;
				}
			};

			let compact_proof = match proof
				.clone()
				.into_compact_proof::<HashFor<Block>>(last_head.state_root().clone())
			{
				Ok(proof) => proof,
				Err(e) => {
					tracing::error!(target: "cumulus-collator", "Failed to compact proof: {:?}", e);
					return None;
				}
			};

			let b = ParachainBlockData::<Block>::new(
				block.header().clone(),
				block.extrinsics().to_vec(),
				compact_proof,
			);
			let block_data = BlockData(b.encode());
			let block_hash = Header::hash(b.header());

			let collation_info = self
				.fetch_collation_info(block_hash, b.header())
				.map_err(|e| {
					tracing::error!(
						target: LOG_TARGET,
						error = ?e,
						"Failed to collect collation info.",
					)
				})
				.ok()
				.flatten()?;

			Some(Collation {
				upward_messages: collation_info.upward_messages,
				new_validation_code: collation_info.new_validation_code,
				processed_downward_messages: collation_info.processed_downward_messages,
				horizontal_messages: collation_info.horizontal_messages,
				hrmp_watermark: collation_info.hrmp_watermark,
				head_data: collation_info.head_data,
				proof_of_validity: MaybeCompressedPoV::Raw(PoV { block_data }),
			})
		} else {
			None
		}
	}

	pub fn approve(&self) {
		let mut locked_next = self.next_block.lock().expect("Locking must work");
		let build = locked_next.take().expect("Only approve when build is some");
		let mut locked_import = self.next_import.lock().expect("Locking must work");
		*locked_import = Some(build);
	}

	pub fn reject(&self) {
		let mut locked = self.next_block.lock().expect("Locking must work");
		let _build = locked.take().expect("Only reject when build is some");
	}
}

pub struct ParachainBuilder<
	Block: BlockT,
	RtApi,
	Exec,
	CIDP,
	ExtraArgs,
	DP,
	B = Backend<Block>,
	C = TFullClient<Block, RtApi, Exec>,
> where
	Block: BlockT,
	C: ProvideRuntimeApi<Block>
		+ BlockBackend<Block>
		+ BlockIdTo<Block>
		+ HeaderBackend<Block>
		+ Send
		+ Sync
		+ 'static,
	C::Api: TaggedTransactionQueue<Block>,
{
	builder: Builder<Block, RtApi, Exec, B, C>,
	cidp: CIDP,
	dp: DP,
	next_block: Arc<Mutex<Option<(Block, StorageProof)>>>,
	next_import: Arc<Mutex<Option<(Block, StorageProof)>>>,
	imports: Vec<(Block, StorageProof)>,
	handle: SpawnTaskHandle,
	_phantom: PhantomData<ExtraArgs>,
}

impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
	ParachainBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
where
	B: BackendT<Block> + 'static,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	CIDP: CreateInherentDataProviders<Block, ExtraArgs> + Send + Sync + 'static,
	CIDP::InherentDataProviders: Send,
	DP: DigestCreator<Block>,
	ExtraArgs: ArgsProvider<ExtraArgs>,
	C::Api: BlockBuilder<Block>
		+ ApiExt<Block, StateBackend = B::State>
		+ CollectCollationInfo<Block>
		+ TaggedTransactionQueue<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ BlockBackend<Block>
		+ BlockIdTo<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>
		+ sc_block_builder::BlockBuilderProvider<B, Block, C>,
{
	pub fn new(manager: &TaskManager, backend: Arc<B>, client: Arc<C>, cidp: CIDP, dp: DP) -> Self {
		Self {
			builder: Builder::new(backend, client, manager),
			cidp,
			dp,
			next_block: Arc::new(Mutex::new(None)),
			next_import: Arc::new(Mutex::new(None)),
			imports: Vec::new(),
			handle: manager.spawn_handle(),
			_phantom: Default::default(),
		}
	}

	pub fn collator(&self) -> FudgeCollator<Block, C, B> {
		FudgeCollator::new(
			self.client(),
			self.backend(),
			self.next_block.clone(),
			self.next_import.clone(),
		)
	}

	pub fn client(&self) -> Arc<C> {
		self.builder.client()
	}

	pub fn backend(&self) -> Arc<B> {
		self.builder.backend()
	}

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> Result<Block::Hash, ()> {
		self.builder.append_extrinsic(xt)
	}

	pub fn append_extrinsics(
		&mut self,
		xts: Vec<Block::Extrinsic>,
	) -> Result<Vec<Block::Hash>, ()> {
		xts.into_iter().fold(Ok(Vec::new()), |hashes, xt| {
			if let Ok(mut hashes) = hashes {
				hashes.push(self.builder.append_extrinsic(xt)?);
				Ok(hashes)
			} else {
				Err(())
			}
		})
	}

	pub fn append_transition(&mut self, aux: StoragePair) {
		self.builder.append_transition(aux);
	}

	pub fn append_transitions(&mut self, auxs: Vec<StoragePair>) {
		auxs.into_iter().for_each(|aux| {
			self.builder.append_transition(aux);
		});
	}

	pub fn pool_state(&self) -> PoolState {
		self.builder.pool_state()
	}

	pub fn build_block(&mut self) -> Result<(), ()> {
		let provider = self
			.with_state(|| {
				futures::executor::block_on(self.cidp.create_inherent_data_providers(
					self.builder.latest_block(),
					ExtraArgs::extra(),
				))
				.unwrap()
			})
			.unwrap();

		let parent = self.builder.latest_header();
		let inherents = provider.create_inherent_data().unwrap();
		let digest = self
			.with_state(|| {
				futures::executor::block_on(self.dp.create_digest(parent, inherents.clone()))
					.unwrap()
			})
			.unwrap();

		let Proposal { block, proof, .. } = self.builder.build_block(
			self.handle.clone(),
			inherents,
			digest,
			Duration::from_secs(60), // TODO: This should be configurable, best via an public config on the builder
			6_000_000, // TODO: This should be configurable, best via an public config on the builder
		);

		// As collation info needs latest state in db we import without finalizing here already.
		let (header, body) = block.clone().deconstruct();
		let mut params = BlockImportParams::new(BlockOrigin::Own, header);
		params.body = Some(body);
		params.fork_choice = Some(ForkChoiceStrategy::Custom(false));
		self.builder.import_block(params).unwrap();

		let locked = self.next_block.clone();
		let mut locked = locked.lock().expect(
			"ESSENTIAL: If this is poisoned or still locked, the builder is currently bricked.",
		);
		*locked = Some((block, proof));

		Ok(())
	}

	pub fn head(&self) -> HeadData {
		HeadData(self.builder.latest_header().encode())
	}

	pub fn code(&self) -> ValidationCode {
		ValidationCode(self.builder.latest_code())
	}

	pub fn import_block(&mut self) -> Result<(), ()> {
		let locked = self.next_import.clone();
		let mut locked = locked.lock().expect(
			"ESSENTIAL: If this is poisoned or still locked, the builder is currently bricked.",
		);

		if let Some((block, proof)) = &*locked {
			let (header, body) = block.clone().deconstruct();
			let mut params = BlockImportParams::new(BlockOrigin::NetworkInitialSync, header);
			params.body = Some(body);
			params.finalized = true;
			params.import_existing = true;
			params.fork_choice = Some(ForkChoiceStrategy::Custom(true));

			self.builder.import_block(params).unwrap();
			self.imports.push((block.clone(), proof.clone()));

			*locked = None;
			Ok(())
		} else {
			tracing::warn!(target: LOG_TARGET, "No import for parachain available.");
			Ok(())
		}
	}

	pub fn imports(&self) -> Vec<(Block, StorageProof)> {
		self.imports.clone()
	}

	pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, None, exec)
	}

	pub fn with_state_at<R>(
		&self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, Some(at), exec)
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		// TODO: still check this
		// assert!(self.next_block.is_none());

		self.builder.with_state(Operation::Commit, None, exec)
	}

	/// Mutating past states not supported yet...
	fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		// TODO: still check this
		// assert!(self.next_block.is_none());

		self.builder.with_state(Operation::Commit, Some(at), exec)
	}
}
