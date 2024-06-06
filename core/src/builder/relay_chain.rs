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

use std::{collections::BTreeMap, pin::Pin};

use async_trait::async_trait;
use bitvec::{order::Lsb0 as BitOrderLsb0, vec::BitVec};
use cumulus_client_parachain_inherent::ParachainInherentDataProvider;
use cumulus_primitives_core::relay_chain::{Hash as PHash, Header as PHeader, SessionIndex};
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainResult};
use frame_support::{
	storage::types::{StorageMap, StorageValue, ValueQuery},
	traits::StorageInstance,
	Identity, Twox64Concat,
};
use futures::{FutureExt, Stream, StreamExt};
use parity_scale_codec::{Decode, Encode};
use polkadot_core_primitives::{
	Block as PBlock, CandidateHash, Hash, InboundDownwardMessage, InboundHrmpMessage,
};
use polkadot_node_primitives::Collation;
use polkadot_overseer::{dummy::dummy_overseer_builder, HeadSupportsParachains};
use polkadot_parachain_primitives::primitives::{
	HeadData, Id, Id as ParaId, ValidationCode, ValidationCodeHash,
};
use polkadot_primitives::{
	runtime_api::ParachainHost,
	v6::{
		CandidateCommitments, CandidateDescriptor, CandidateReceipt, CoreIndex,
		OccupiedCoreAssumption,
	},
	CommittedCandidateReceipt, GroupIndex, PersistedValidationData, ValidatorId,
};
use polkadot_runtime_parachains::{inclusion::CandidatePendingAvailability, paras, ParaLifecycle};
use polkadot_service::Handle;
use sc_client_api::{
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, BlockchainEvents, HeaderBackend,
	ImportNotifications, UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TFullClient};
use sc_transaction_pool::FullPool;
use sc_transaction_pool_api::{MaintainedTransactionPool, TransactionPool};
use scale_info::TypeInfo;
use sp_api::{ApiExt, ApiRef, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi, StorageProof};
use sp_block_builder::BlockBuilder;
use sp_blockchain::BlockStatus;
use sp_consensus::{BlockOrigin, NoNetwork, Proposal, SyncOracle};
use sp_core::{traits::CodeExecutor, H256};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, BlockIdTo, Header},
	TransactionOutcome,
};
use sp_state_machine::Backend as StateBackend;
use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use thiserror::Error;
use types::*;

use crate::{
	builder::{
		core::{Builder, InnerError, Operation},
		parachain::FudgeParaChain,
		PoolState,
	},
	digest::DigestCreator,
	inherent::ArgsProvider,
	provider::Initiator,
	types::StoragePair,
};

const DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET: &str = "fudge-relaychain";

#[derive(Error, Debug)]
pub enum Error {
	#[error("core builder: {0}")]
	CoreBuilder(InnerError),

	#[error("initiator: {0}")]
	Initiator(InnerError),

	#[error("parachain judge error: {0}")]
	ParachainJudgeError(InnerError),

	#[error("parachain mutation error")]
	ParachainMutation,

	#[error("para lifecycles mutation error")]
	ParaLifecyclesMutation,

	#[error("persisted validation data retrieval: {0}")]
	PersistedValidationDataRetrieval(InnerError),

	#[error("persisted validation data not found")]
	PersistedValidationDataNotFound,

	#[error("validation code hash retrieval: {0}")]
	ValidationCodeHashRetrieval(InnerError),

	#[error("validation code hash not found")]
	ValidationCodeHashNotFound,

	#[error("inherent data providers creation: {0}")]
	InherentDataProvidersCreation(InnerError),

	#[error("inherent data creation: {0}")]
	InherentDataCreation(InnerError),

	#[error("digest creation: {0}")]
	DigestCreation(InnerError),

	#[error("candidate pending availability decoding: {0}")]
	CandidatePendingAvailabilityDecoding(InnerError),

	#[error("parachain not onboarded")]
	ParachainNotOnboarded,

	#[error("parachain core index creation: {0}")]
	ParachainCoreIndexCreation(InnerError),

	#[error("parachain not found")]
	ParachainNotFound,

	#[error("parachain inherent data creation")]
	ParachainInherentDataCreation,

	#[error("next block not found")]
	NextBlockNotFound,

	#[error("parachain {0} collation error: {1}")]
	ParachainCollation(Id, InnerError),
}

/// Recreating private storage types for easier handling storage access
pub mod types {
	use frame_system::pallet_prelude::BlockNumberFor;

	use super::*;

	pub struct ParaLifecyclesPrefix;
	impl StorageInstance for ParaLifecyclesPrefix {
		const STORAGE_PREFIX: &'static str = "ParaLifecycles";

		fn pallet_prefix() -> &'static str {
			"Paras"
		}
	}
	pub type ParaLifecycles = StorageMap<ParaLifecyclesPrefix, Twox64Concat, ParaId, ParaLifecycle>;

	pub struct ParachainsPrefix;
	impl StorageInstance for ParachainsPrefix {
		const STORAGE_PREFIX: &'static str = "Parachains";

		fn pallet_prefix() -> &'static str {
			"Paras"
		}
	}
	pub type Parachains = StorageValue<ParachainsPrefix, Vec<ParaId>, ValueQuery>;

	pub struct HeadsPrefix;
	impl StorageInstance for HeadsPrefix {
		const STORAGE_PREFIX: &'static str = "Heads";

		fn pallet_prefix() -> &'static str {
			"Paras"
		}
	}
	pub type Heads = StorageMap<HeadsPrefix, Twox64Concat, ParaId, HeadData>;

	pub struct CurrentCodeHashPrefix;
	impl StorageInstance for CurrentCodeHashPrefix {
		const STORAGE_PREFIX: &'static str = "CurrentCodeHash";

		fn pallet_prefix() -> &'static str {
			"Paras"
		}
	}
	pub type CurrentCodeHash =
		StorageMap<CurrentCodeHashPrefix, Twox64Concat, ParaId, ValidationCodeHash>;

	pub struct CodeByHashPrefix;
	impl StorageInstance for CodeByHashPrefix {
		const STORAGE_PREFIX: &'static str = "CodeByHash";

		fn pallet_prefix() -> &'static str {
			"Paras"
		}
	}
	pub type CodeByHash =
		StorageMap<CodeByHashPrefix, Identity, ValidationCodeHash, ValidationCode>;

	pub struct CodeByHashRefsPrefix;
	impl StorageInstance for CodeByHashRefsPrefix {
		const STORAGE_PREFIX: &'static str = "CodeByHashRefs";

		fn pallet_prefix() -> &'static str {
			"Paras"
		}
	}
	pub type CodeByHashRefs =
		StorageMap<CodeByHashRefsPrefix, Identity, ValidationCodeHash, u32, ValueQuery>;

	pub struct PastCodeHashPrefix;
	impl StorageInstance for PastCodeHashPrefix {
		const STORAGE_PREFIX: &'static str = "PastCodeHash";

		fn pallet_prefix() -> &'static str {
			"Paras"
		}
	}
	#[allow(type_alias_bounds)]
	pub type PastCodeHash<T: frame_system::Config> = StorageMap<
		PastCodeHashPrefix,
		Twox64Concat,
		(ParaId, BlockNumberFor<T>),
		ValidationCodeHash,
	>;

	pub struct PendingAvailabilityPrefix;
	impl StorageInstance for PendingAvailabilityPrefix {
		const STORAGE_PREFIX: &'static str = "PendingAvailability";

		fn pallet_prefix() -> &'static str {
			"ParaInclusion"
		}
	}
	#[allow(type_alias_bounds)]
	pub type PendingAvailability<T: frame_system::Config> = StorageMap<
		PendingAvailabilityPrefix,
		Twox64Concat,
		ParaId,
		CandidatePendingAvailability<T::Hash, BlockNumberFor<T>>,
	>;

	pub struct PendingAvailabilityCommitmentsPrefix;
	impl StorageInstance for PendingAvailabilityCommitmentsPrefix {
		const STORAGE_PREFIX: &'static str = "PendingAvailabilityCommitments";

		fn pallet_prefix() -> &'static str {
			"ParaInclusion"
		}
	}
	pub type PendingAvailabilityCommitments = StorageMap<
		PendingAvailabilityCommitmentsPrefix,
		Twox64Concat,
		ParaId,
		CandidateCommitments,
	>;

	// TODO: Need a test that automatically detects whether this changes
	//       on the polkadot side. Via encode from this type and decode into
	//       imported polkadot type.
	/// A backed candidate pending availability.
	#[derive(Encode, Decode, PartialEq, TypeInfo)]
	#[cfg_attr(test, derive(Debug))]
	pub struct FudgeCandidatePendingAvailability<H, N> {
		/// The availability core this is assigned to.
		pub core: CoreIndex,
		/// The candidate hash.
		pub hash: CandidateHash,
		/// The candidate descriptor.
		pub descriptor: CandidateDescriptor<H>,
		/// The received availability votes. One bit per validator.
		pub availability_votes: BitVec<u8, BitOrderLsb0>,
		/// The backers of the candidate pending availability.
		pub backers: BitVec<u8, BitOrderLsb0>,
		/// The block number of the relay-parent of the receipt.
		pub relay_parent_number: N,
		/// The block number of the relay-chain block this was backed in.
		pub backed_in_number: N,
		/// The group index backing this block.
		pub backing_group: GroupIndex,
	}
}

pub enum CollationJudgement {
	Approved,
	Rejected,
}

pub trait CollationBuilder {
	fn collation(
		&self,
		validation_data: PersistedValidationData,
	) -> Result<Option<Collation>, Box<dyn std::error::Error>>;

	fn judge(&self, judgement: CollationJudgement) -> Result<(), Box<dyn std::error::Error>>;
}

#[cfg(test)]
impl CollationBuilder for () {
	fn collation(
		&self,
		_validation_data: PersistedValidationData,
	) -> Result<Option<Collation>, Box<dyn std::error::Error>> {
		Ok(None)
	}

	fn judge(&self, _judgement: CollationJudgement) -> Result<(), Box<dyn std::error::Error>> {
		Ok(())
	}
}

struct MockSupportsParachains;

#[async_trait]
impl HeadSupportsParachains for MockSupportsParachains {
	async fn head_supports_parachains(&self, _head: &Hash) -> bool {
		true
	}
}

pub struct InherentBuilder<B, C> {
	id: Id,
	backend: Arc<B>,
	client: Arc<C>,
}

impl<B, C> Clone for InherentBuilder<B, C> {
	fn clone(&self) -> Self {
		InherentBuilder {
			id: self.id.clone(),
			backend: self.backend.clone(),
			client: self.client.clone(),
		}
	}

	fn clone_from(&mut self, _source: &Self) {
		todo!()
	}
}

impl<B, C> InherentBuilder<B, C>
where
	B: BackendT<PBlock>,
	C::Api: BlockBuilder<PBlock> + ParachainHost<PBlock> + TaggedTransactionQueue<PBlock>,
	C: 'static
		+ ProvideRuntimeApi<PBlock>
		+ BlockOf
		+ BlockBackend<PBlock>
		+ BlockIdTo<PBlock>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<PBlock>
		+ BlockchainEvents<PBlock>
		+ HeaderBackend<PBlock>
		+ BlockImport<PBlock>
		+ CallApiAt<PBlock>,
	for<'r> &'r C: BlockImport<PBlock>,
{
	pub async fn parachain_inherent(&self) -> Result<ParachainInherentData, Error> {
		let parent = self.client.info().best_hash;

		let spawner = sp_core::testing::TaskExecutor::new();

		let (_overseer, handle) = dummy_overseer_builder(spawner, MockSupportsParachains, None)
			.unwrap()
			.build()
			.unwrap();

		let dummy_handler = Handle::new(handle);
		let relay_interface = FudgeRelayChainProcessInterface::new(
			self.client.clone(),
			self.backend.clone(),
			Arc::new(NoNetwork {}),
			dummy_handler,
		);
		let api = self.client.runtime_api();
		let persisted_validation_data = api
			.persisted_validation_data(parent, self.id, OccupiedCoreAssumption::TimedOut)
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not get persisted validation data",
				);

				Error::PersistedValidationDataRetrieval(e.into())
			})?
			.ok_or_else(|| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					"Persisted validation data not found",
				);

				Error::PersistedValidationDataNotFound
			})?;

		let pid = ParachainInherentDataProvider::create_at(
			parent,
			&relay_interface,
			&persisted_validation_data,
			self.id,
		)
		.await
		.ok_or_else(|| {
			tracing::error!(
				target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
				"Could not create parachain inherent data",
			);

			Error::ParachainInherentDataCreation
		});

		pid
	}
}

pub struct RelaychainBuilder<
	Block: BlockT,
	RtApi,
	Exec,
	CIDP,
	ExtraArgs,
	DP,
	Runtime,
	B = Backend<Block>,
	C = TFullClient<Block, RtApi, Exec>,
	A = FullPool<Block, C>,
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
	A: TransactionPool<Block = Block, Hash = Block::Hash> + MaintainedTransactionPool + 'static,
{
	builder: Builder<Block, RtApi, Exec, B, C, A>,
	cidp: CIDP,
	dp: DP,
	next: Option<(Block, StorageProof)>,
	imports: Vec<(Block, StorageProof)>,
	parachains: Vec<(Id, Box<dyn CollationBuilder>)>,
	collations: Vec<(Id, Collation, Block::Header)>,
	handle: SpawnTaskHandle,
	_phantom: PhantomData<(ExtraArgs, Runtime)>,
}

impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, B, C, A>
	RelaychainBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, Runtime, B, C, A>
where
	B: BackendT<Block> + 'static,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	CIDP: CreateInherentDataProviders<Block, ExtraArgs> + Send + Sync + 'static,
	CIDP::InherentDataProviders: Send,
	DP: DigestCreator<Block> + 'static,
	ExtraArgs: ArgsProvider<ExtraArgs>,
	Runtime: paras::Config
		+ frame_system::Config
		+ polkadot_runtime_parachains::session_info::Config
		+ polkadot_runtime_parachains::initializer::Config,
	C::Api: BlockBuilder<Block> + ParachainHost<Block> + TaggedTransactionQueue<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ BlockBackend<Block>
		+ BlockIdTo<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ BlockchainEvents<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
	for<'r> &'r C: BlockImport<Block>,
	A: TransactionPool<Block = Block, Hash = Block::Hash> + MaintainedTransactionPool + 'static,
{
	pub fn new<I, F>(initiator: I, setup: F) -> Result<Self, Error>
	where
		I: Initiator<Block, Api = C::Api, Client = C, Backend = B, Pool = A, Executor = Exec>,
		F: FnOnce(Arc<C>) -> (CIDP, DP),
	{
		let (client, backend, pool, executor, task_manager) = initiator.init().map_err(|e| {
			tracing::error!(
				target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not initialize."
			);

			Error::Initiator(e.into())
		})?;

		let (cidp, dp) = setup(client.clone());
		let handle = task_manager.spawn_handle();

		Ok(Self {
			builder: Builder::new(client, backend, pool, executor, task_manager),
			cidp,
			dp,
			next: None,
			imports: Vec::new(),
			parachains: Vec::new(),
			collations: Vec::new(),
			handle,
			_phantom: Default::default(),
		})
	}

	pub fn client(&self) -> Arc<C> {
		self.builder.client()
	}

	pub fn backend(&self) -> Arc<B> {
		self.builder.backend()
	}

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> Result<Block::Hash, Error> {
		self.builder
			.append_extrinsic(xt)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	pub fn append_extrinsics(
		&mut self,
		xts: Vec<Block::Extrinsic>,
	) -> Result<Vec<Block::Hash>, Error> {
		xts.into_iter().fold(Ok(Vec::new()), |hashes, xt| {
			let mut hashes = hashes?;

			let block_hash = self
				.builder
				.append_extrinsic(xt)
				.map_err(|e| Error::CoreBuilder(e.into()))?;

			hashes.push(block_hash);

			Ok(hashes)
		})
	}

	pub fn update_para_head(&mut self, id: Id, head: HeadData) -> Result<(), Error> {
		self.with_mut_state(|| {
			Heads::insert(&id, head);
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

	pub fn inherent_builder(&self, para_id: Id) -> InherentBuilder<B, C> {
		InherentBuilder {
			id: para_id,
			client: self.builder.client(),
			backend: self.builder.backend(),
		}
	}

	pub fn onboard_para(
		&mut self,
		para: FudgeParaChain,
		collator: Box<dyn CollationBuilder>,
	) -> Result<(), Error> {
		let FudgeParaChain { id, head, code } = para;

		self.with_mut_state(|| -> Result<(), Error> {
			let current_block = frame_system::Pallet::<Runtime>::block_number();
			let code_hash = code.hash();

			Parachains::try_mutate::<(), (), _>(|paras| {
				if !paras.contains(&id) {
					paras.push(id);
					paras.sort();
				}
				Ok(())
			})
			.map_err(|_| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					"Could not mutate parachains."
				);

				Error::ParachainMutation
			})?;

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

			ParaLifecycles::try_mutate::<_, (), (), _>(id, |para_lifecycle| {
				if let Some(lifecycle) = para_lifecycle.as_mut() {
					*lifecycle = ParaLifecycle::Parachain;
				} else {
					*para_lifecycle = Some(ParaLifecycle::Parachain);
				}

				Ok(())
			})
			.map_err(|_| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					"Could not mutate para lifecycles."
				);

				Error::ParaLifecyclesMutation
			})?;

			Heads::insert(&id, head);

			Ok(())
		})??;

		self.parachains.push((id, collator));

		Ok(())
	}

	pub fn build_block(&mut self) -> Result<Block, Error> {
		assert!(self.next.is_none());

		let provider =
			self.with_state(|| {
				futures::executor::block_on(self.cidp.create_inherent_data_providers(
					self.builder.latest_block(),
					ExtraArgs::extra(),
				))
				.map_err(|e| {
					tracing::error!(
						target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
						error = ?e,
						"Could not create inherent data providers."
					);

					Error::InherentDataProvidersCreation(e)
				})
			})??;

		let inherents =
			futures::executor::block_on(provider.create_inherent_data()).map_err(|e| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not create inherent data."
				);

				Error::InherentDataCreation(e.into())
			})?;

		let parent = self.builder.latest_header().map_err(|e| {
			tracing::error!(
				target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not retrieve latest header."
			);

			Error::CoreBuilder(e.into())
		})?;

		let digest = self.with_state(|| {
			futures::executor::block_on(self.dp.create_digest(parent, inherents.clone())).map_err(
				|e| {
					tracing::error!(
						target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
						error = ?e,
						"Could not create digest."
					);

					Error::DigestCreation(e.into())
				},
			)
		})??;

		let Proposal { block, proof, .. } = self
			.builder
			.build_block(
				self.builder.handle(),
				inherents,
				digest,
				Duration::from_secs(60),
				6_000_000,
			)
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not build block."
				);

				Error::CoreBuilder(e.into())
			})?;

		self.next = Some((block.clone(), proof));

		Ok(block)
	}

	pub fn import_block(&mut self) -> Result<(), Error> {
		let (block, proof) = self.next.take().ok_or_else(|| {
			tracing::error!(
				target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
				"Next block not found."
			);

			Error::NextBlockNotFound
		})?;

		let (header, body) = block.clone().deconstruct();
		let mut params = BlockImportParams::new(BlockOrigin::NetworkInitialSync, header);
		params.body = Some(body);
		params.finalized = true;
		params.fork_choice = Some(ForkChoiceStrategy::Custom(true));

		self.builder.import_block(params).map_err(|e| {
			tracing::error!(
				target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not import block."
			);

			Error::CoreBuilder(e.into())
		})?;

		let block_header = block.header().clone();

		self.imports.push((block, proof));

		self.force_enact_collations()?;
		self.collect_collations(block_header)?;

		Ok(())
	}

	fn collect_collations(&mut self, parent_head: Block::Header) -> Result<(), Error> {
		let client = self.client();
		let mut rt_api = client.runtime_api();
		let parent = self.client().info().best_hash;
		for (id, para) in self.parachains.iter() {
			let pvd = self.with_state(|| {
				persisted_validation_data(
					&mut rt_api,
					parent,
					*id,
					OccupiedCoreAssumption::TimedOut,
				)
			})??;

			let parachain_collation = para
				.collation(pvd)
				.map_err(|e| Error::ParachainCollation(id.clone(), e))?;

			if let Some(collation) = parachain_collation {
				// Only collect collations when they are not already collected
				if !self.collations.iter().any(|(in_id, _, _)| in_id == id) {
					self.collations.push((*id, collation, parent_head.clone()))
				}
			}
		}

		Ok(())
	}

	fn force_enact_collations(&mut self) -> Result<(), Error> {
		let client = self.client();
		let mut rt_api = client.runtime_api();
		let parent = self.client().info().best_hash;
		let parent_number = self.client().info().best_number;

		for (id, collation, generation_parent) in self.collations.clone().into_iter() {
			let pvd = self.with_state(|| {
				persisted_validation_data(&mut rt_api, parent, id, OccupiedCoreAssumption::TimedOut)
			})??;

			self.with_mut_state(|| -> Result<(), Error> {
				let commitments = CandidateCommitments {
					upward_messages: collation.upward_messages,
					horizontal_messages: collation.horizontal_messages,
					new_validation_code: collation.new_validation_code,
					head_data: collation.head_data,
					processed_downward_messages: collation.processed_downward_messages,
					hrmp_watermark: collation.hrmp_watermark,
				};

				// Inject into storage for force enact
				PendingAvailabilityCommitments::insert(id, commitments.clone());

				let signature = sp_core::sr25519::Signature([0u8; 64]);
				let public = sp_core::sr25519::Public([0u8; 32]);
				let ccr = CandidateReceipt {
					commitments_hash: commitments.hash(),
					descriptor: CandidateDescriptor {
						signature: signature.into(),
						para_id: id,
						relay_parent: parent,
						collator: public.into(),
						persisted_validation_data_hash: pvd.hash(),
						pov_hash: collation.proof_of_validity.into_compressed().hash(),
						erasure_root: Default::default(),
						para_head: commitments.head_data.hash(),
						validation_code_hash: validation_code_hash(
							&mut rt_api,
							parent,
							id,
							OccupiedCoreAssumption::TimedOut,
						)?,
					},
				};

				let core = CoreIndex(
					Parachains::get()
						.iter()
						.position(|in_id| *in_id == id)
						.ok_or_else(|| {
							tracing::error!(
								target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
								"Parachain not onboarded",
							);

							Error::ParachainNotOnboarded
						})?
						.try_into()
						.map_err(|e| {
							tracing::error!(
								target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
								error = ?e,
								"Could not create parachain core index",
							);

							Error::ParachainCoreIndexCreation(InnerError::from(e))
						})?,
				);

				let cpa = FudgeCandidatePendingAvailability {
					core,
					hash: Default::default(),
					descriptor: ccr.descriptor,
					availability_votes: BitVec::new(),
					backers: BitVec::new(),
					relay_parent_number: parent_number,
					backed_in_number: generation_parent.number().clone(),
					backing_group: Default::default(),
				};

				PendingAvailability::<Runtime>::insert(
					id,
					cpa.using_encoded(
						|scale| -> Result<CandidatePendingAvailability<_, _>, Error> {
							let res = CandidatePendingAvailability::decode(&mut scale.as_ref())
								.map_err(|e| {
									tracing::error!(
										target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
										error = ?e,
										"Could not decode candidate pending availability",
									);

									Error::CandidatePendingAvailabilityDecoding(e.into())
								})?;

							Ok(res)
						},
					)?,
				);

				// NOTE: Calling this with OccupiedCoreAssumption::Included, force_enacts the para
				polkadot_runtime_parachains::runtime_api_impl::v7::persisted_validation_data::<
					Runtime,
				>(id, OccupiedCoreAssumption::Included)
				.ok_or_else(|| {
					tracing::error!(
						target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
						"Persisted validation data not found",
					);

					Error::PersistedValidationDataNotFound
				})?;

				Ok(())
			})??;

			let index = self
				.parachains
				.iter()
				.position(|(in_id, _)| *in_id == id)
				.ok_or_else(|| {
					tracing::error!(
						target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
						"Parachain not onboarded",
					);

					Error::ParachainNotOnboarded
				})?;

			let (_, collator) = self.parachains.get(index).ok_or_else(|| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					"Parachain not found",
				);

				Error::ParachainNotFound
			})?;

			collator.judge(CollationJudgement::Approved).map_err(|e| {
				tracing::error!(
					target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
					error = ?e,
					"Parachain judge error",
				);

				Error::ParachainJudgeError(e.into())
			})?;
		}

		self.collations = Vec::new();

		Ok(())
	}

	pub fn imports(&self) -> Vec<(Block, StorageProof)> {
		self.imports.clone()
	}

	pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, Error> {
		self.builder
			.with_state(Operation::DryRun, None, exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	pub fn with_state_at<R>(
		&self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, Error> {
		self.builder
			.with_state(Operation::DryRun, Some(at), exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, Error> {
		assert!(self.next.is_none());

		self.builder
			.with_state(Operation::Commit, None, exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	/// Mutating past states not supported yet...
	fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, Error> {
		assert!(self.next.is_none());

		self.builder
			.with_state(Operation::Commit, Some(at), exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}
}

fn persisted_validation_data<RtApi: ApiExt<Block>, Block>(
	rt_api: &mut ApiRef<RtApi>,
	parent: Block::Hash,
	id: Id,
	assumption: OccupiedCoreAssumption,
) -> Result<PersistedValidationData, Error>
where
	Block: BlockT,
	RtApi: ParachainHost<Block>,
{
	let res = rt_api.execute_in_transaction(|api| {
		let pvd = api.persisted_validation_data(parent, id, assumption);

		TransactionOutcome::Commit(pvd)
	});

	res.map_err(|e| {
		tracing::error!(
			target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
			error = ?e,
			"Could get persisted validation data",
		);

		Error::PersistedValidationDataRetrieval(e.into())
	})?
	.ok_or_else(|| {
		tracing::error!(
			target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
			"Persisted validation data not found",
		);

		Error::PersistedValidationDataNotFound
	})
}

fn validation_code_hash<RtApi, Block>(
	rt_api: &mut ApiRef<RtApi>,
	parent: Block::Hash,
	id: Id,
	assumption: OccupiedCoreAssumption,
) -> Result<ValidationCodeHash, Error>
where
	Block: BlockT,
	RtApi: ParachainHost<Block>,
{
	rt_api
		.validation_code_hash(parent, id, assumption)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
				error = ?e,
				"Could get validation code hash.",
			);

			Error::ValidationCodeHashRetrieval(e.into())
		})?
		.ok_or_else(|| {
			tracing::error!(
				target = DEFAULT_RELAY_CHAIN_BUILDER_LOG_TARGET,
				"Validation code hash not found",
			);

			Error::ValidationCodeHashNotFound
		})
}

/// Provides a generic implementation of the [`RelayChainInterface`] using a local in-process relay chain
/// node.
///
/// NOTE - this was created due to the fact that the [`RelayChainInProcessInterface`] provided in the
/// cumulus-relay-chain-inprocess-interface crate expects specific implementations of [`TFullBackend`] and [`TFullClient`].
pub struct FudgeRelayChainProcessInterface<B, C> {
	full_client: Arc<C>,
	backend: Arc<B>,
	sync_oracle: Arc<dyn SyncOracle + Send + Sync>,
	overseer_handle: Handle,
}

impl<B, C> FudgeRelayChainProcessInterface<B, C>
where
	B: BackendT<PBlock>,
	C::Api: BlockBuilder<PBlock> + ParachainHost<PBlock> + TaggedTransactionQueue<PBlock>,
	C: 'static
		+ ProvideRuntimeApi<PBlock>
		+ BlockOf
		+ BlockBackend<PBlock>
		+ BlockIdTo<PBlock>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<PBlock>
		+ BlockchainEvents<PBlock>
		+ HeaderBackend<PBlock>
		+ BlockImport<PBlock>
		+ CallApiAt<PBlock>,
	for<'r> &'r C: BlockImport<PBlock>,
{
	/// Create a new instance of [`FudgeRelayChainProcessInterface`]
	pub fn new(
		full_client: Arc<C>,
		backend: Arc<B>,
		sync_oracle: Arc<dyn SyncOracle + Send + Sync>,
		overseer_handle: Handle,
	) -> Self {
		Self {
			full_client,
			backend,
			sync_oracle,
			overseer_handle,
		}
	}
}

/// The timeout in seconds after that the waiting for a block should be aborted.
const TIMEOUT_IN_SECONDS: u64 = 6;

#[async_trait]
impl<B, C> cumulus_relay_chain_interface::RelayChainInterface
	for FudgeRelayChainProcessInterface<B, C>
where
	B: BackendT<PBlock>,
	C::Api: BlockBuilder<PBlock> + ParachainHost<PBlock> + TaggedTransactionQueue<PBlock>,
	C: 'static
		+ ProvideRuntimeApi<PBlock>
		+ BlockOf
		+ BlockBackend<PBlock>
		+ BlockIdTo<PBlock>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<PBlock>
		+ BlockchainEvents<PBlock>
		+ HeaderBackend<PBlock>
		+ BlockImport<PBlock>
		+ CallApiAt<PBlock>,
	for<'r> &'r C: BlockImport<PBlock>,
{
	async fn retrieve_dmq_contents(
		&self,
		para_id: ParaId,
		relay_parent: PHash,
	) -> RelayChainResult<Vec<InboundDownwardMessage>> {
		Ok(self
			.full_client
			.runtime_api()
			.dmq_contents(relay_parent, para_id)?)
	}

	async fn retrieve_all_inbound_hrmp_channel_contents(
		&self,
		para_id: ParaId,
		relay_parent: PHash,
	) -> RelayChainResult<BTreeMap<ParaId, Vec<InboundHrmpMessage>>> {
		Ok(self
			.full_client
			.runtime_api()
			.inbound_hrmp_channels_contents(relay_parent, para_id)?)
	}

	async fn header(
		&self,
		block_id: polkadot_core_primitives::BlockId,
	) -> RelayChainResult<Option<PHeader>> {
		let hash = match block_id {
			BlockId::Hash(hash) => hash,
			BlockId::Number(num) => {
				if let Some(hash) = self.full_client.hash(num)? {
					hash
				} else {
					return Ok(None);
				}
			}
		};
		let header = self.full_client.header(hash)?;

		Ok(header)
	}

	async fn persisted_validation_data(
		&self,
		hash: PHash,
		para_id: ParaId,
		occupied_core_assumption: OccupiedCoreAssumption,
	) -> RelayChainResult<Option<PersistedValidationData>> {
		Ok(self.full_client.runtime_api().persisted_validation_data(
			hash,
			para_id,
			occupied_core_assumption,
		)?)
	}

	async fn candidate_pending_availability(
		&self,
		hash: PHash,
		para_id: ParaId,
	) -> RelayChainResult<Option<CommittedCandidateReceipt>> {
		Ok(self
			.full_client
			.runtime_api()
			.candidate_pending_availability(hash, para_id)?)
	}

	async fn session_index_for_child(&self, hash: PHash) -> RelayChainResult<SessionIndex> {
		Ok(self
			.full_client
			.runtime_api()
			.session_index_for_child(hash)?)
	}

	async fn validators(&self, hash: PHash) -> RelayChainResult<Vec<ValidatorId>> {
		Ok(self.full_client.runtime_api().validators(hash)?)
	}

	async fn import_notification_stream(
		&self,
	) -> RelayChainResult<Pin<Box<dyn Stream<Item = PHeader> + Send>>> {
		let notification_stream = self
			.full_client
			.import_notification_stream()
			.map(|notification| notification.header);
		Ok(Box::pin(notification_stream))
	}

	async fn finality_notification_stream(
		&self,
	) -> RelayChainResult<Pin<Box<dyn Stream<Item = PHeader> + Send>>> {
		let notification_stream = self
			.full_client
			.finality_notification_stream()
			.map(|notification| notification.header);
		Ok(Box::pin(notification_stream))
	}

	async fn best_block_hash(&self) -> RelayChainResult<PHash> {
		Ok(self.backend.blockchain().info().best_hash)
	}

	async fn finalized_block_hash(&self) -> RelayChainResult<PHash> {
		Ok(self.backend.blockchain().info().finalized_hash)
	}

	async fn is_major_syncing(&self) -> RelayChainResult<bool> {
		Ok(self.sync_oracle.is_major_syncing())
	}

	fn overseer_handle(&self) -> RelayChainResult<Handle> {
		Ok(self.overseer_handle.clone())
	}

	async fn get_storage_by_key(
		&self,
		relay_parent: PHash,
		key: &[u8],
	) -> RelayChainResult<Option<sp_state_machine::StorageValue>> {
		let state = self.backend.state_at(relay_parent)?;
		state
			.storage(key)
			.map_err(|e| RelayChainError::GenericError(e.to_string()))
	}

	async fn prove_read(
		&self,
		relay_parent: PHash,
		relevant_keys: &Vec<Vec<u8>>,
	) -> RelayChainResult<StorageProof> {
		let state_backend = self.backend.state_at(relay_parent)?;

		sp_state_machine::prove_read(state_backend, relevant_keys)
			.map_err(RelayChainError::StateMachineError)
	}

	/// Wait for a given relay chain block in an async way.
	///
	/// The caller needs to pass the hash of a block it waits for and the function will return when
	/// the block is available or an error occurred.
	///
	/// The waiting for the block is implemented as follows:
	///
	/// 1. Get a read lock on the import lock from the backend.
	///
	/// 2. Check if the block is already imported. If yes, return from the function.
	///
	/// 3. If the block isn't imported yet, add an import notification listener.
	///
	/// 4. Poll the import notification listener until the block is imported or the timeout is
	/// fired.
	///
	/// The timeout is set to 6 seconds. This should be enough time to import the block in the
	/// current round and if not, the new round of the relay chain already started anyway.
	async fn wait_for_block(&self, hash: PHash) -> RelayChainResult<()> {
		let mut listener =
			match check_block_in_chain(self.backend.clone(), self.full_client.clone(), hash)? {
				BlockCheckStatus::InChain => return Ok(()),
				BlockCheckStatus::Unknown(listener) => listener,
			};

		let mut timeout = futures_timer::Delay::new(Duration::from_secs(TIMEOUT_IN_SECONDS)).fuse();

		loop {
			futures::select! {
				_ = timeout => return Err(RelayChainError::WaitTimeout(hash)),
				evt = listener.next() => match evt {
					Some(evt) if evt.hash == hash => return Ok(()),
					// Not the event we waited on.
					Some(_) => continue,
					None => return Err(RelayChainError::ImportListenerClosed(hash)),
				}
			}
		}
	}

	async fn new_best_notification_stream(
		&self,
	) -> RelayChainResult<Pin<Box<dyn Stream<Item = PHeader> + Send>>> {
		let notifications_stream =
			self.full_client
				.import_notification_stream()
				.filter_map(|notification| async move {
					notification.is_new_best.then_some(notification.header)
				});
		Ok(Box::pin(notifications_stream))
	}
}

pub enum BlockCheckStatus {
	/// Block is in chain
	InChain,
	/// Block status is unknown, listener can be used to wait for notification
	Unknown(ImportNotifications<PBlock>),
}

fn check_block_in_chain<B, C>(
	backend: Arc<B>,
	client: Arc<C>,
	hash: PHash,
) -> RelayChainResult<BlockCheckStatus>
where
	B: BackendT<PBlock>,
	C::Api: BlockBuilder<PBlock> + ParachainHost<PBlock> + TaggedTransactionQueue<PBlock>,
	C: 'static
		+ ProvideRuntimeApi<PBlock>
		+ BlockOf
		+ BlockBackend<PBlock>
		+ BlockIdTo<PBlock>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<PBlock>
		+ BlockchainEvents<PBlock>
		+ HeaderBackend<PBlock>
		+ BlockImport<PBlock>
		+ CallApiAt<PBlock>,
	for<'r> &'r C: BlockImport<PBlock>,
{
	let _lock = backend.get_import_lock().read();

	if backend.blockchain().status(hash)? == BlockStatus::InChain {
		return Ok(BlockCheckStatus::InChain);
	}

	let listener = client.import_notification_stream();

	Ok(BlockCheckStatus::Unknown(listener))
}
