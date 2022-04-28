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

use bitvec::vec::BitVec;
use codec::{Decode, Encode};
use cumulus_primitives_core::PersistedValidationData;
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_relay_chain_inprocess_interface::RelayChainInProcessInterface;
use polkadot_core_primitives::Block as PBlock;
use polkadot_node_primitives::Collation;
use polkadot_parachain::primitives::{Id, ValidationCodeHash};
use polkadot_primitives::{
	v1::{
		CandidateCommitments, CandidateDescriptor, CandidateReceipt, CoreIndex,
		OccupiedCoreAssumption,
	},
	v2::ParachainHost,
};
use polkadot_runtime_parachains::{inclusion::CandidatePendingAvailability, paras, ParaLifecycle};
use polkadot_service::Handle;
use sc_client_api::{
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, BlockchainEvents, HeaderBackend,
	TransactionFor, UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{TFullBackend, TFullClient};
use sc_transaction_pool::FullPool;
use sc_transaction_pool_api::{MaintainedTransactionPool, TransactionPool};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi, StorageProof};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, NoNetwork, Proposal};
use sp_consensus_babe::BabeApi;
use sp_core::{traits::CodeExecutor, H256};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, BlockIdTo, Header as HeaderT},
};
use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use types::*;

use crate::{
	builder::{
		core::{Builder, Operation},
		parachain::FudgeParaChain,
		PoolState,
	},
	digest::DigestCreator,
	inherent::ArgsProvider,
	provider::Initiator,
	types::StoragePair,
	PoolState,
};

/// Recreating private storage types for easier handling storage access
pub mod types {
	use bitvec::{order::Lsb0 as BitOrderLsb0, vec::BitVec};
	use codec::{Decode, Encode};
	use frame_support::{
		storage::types::{StorageMap, StorageValue, ValueQuery},
		traits::StorageInstance,
		Identity, Twox64Concat,
	};
	use polkadot_parachain::primitives::{
		HeadData, Id as ParaId, ValidationCode, ValidationCodeHash,
	};
	use polkadot_primitives::v1::{
		CandidateCommitments, CandidateDescriptor, CandidateHash, CoreIndex, GroupIndex,
	};
	use polkadot_runtime_parachains::{inclusion::CandidatePendingAvailability, ParaLifecycle};
	use scale_info::TypeInfo;

	pub struct ParaLifecyclesPrefix;
	impl StorageInstance for ParaLifecyclesPrefix {
		const STORAGE_PREFIX: &'static str = "Parachains";

		fn pallet_prefix() -> &'static str {
			"ParaLifecycles"
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
	pub type PastCodeHash<T: frame_system::Config> =
		StorageMap<PastCodeHashPrefix, Twox64Concat, (ParaId, T::BlockNumber), ValidationCodeHash>;

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
		CandidatePendingAvailability<T::Hash, T::BlockNumber>,
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

	// TODO: Need a test that automatically detects wheter this changes
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
		pub availability_votes: BitVec<BitOrderLsb0, u8>,
		/// The backers of the candidate pending availability.
		pub backers: BitVec<BitOrderLsb0, u8>,
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
	fn collation(&self, validation_data: PersistedValidationData) -> Option<Collation>;

	fn judge(&self, judgement: CollationJudgement);
}

pub struct InherentBuilder<C, B> {
	id: Id,
	client: Arc<C>,
	backend: Arc<B>,
}

impl<C, B> Clone for InherentBuilder<C, B> {
	fn clone(&self) -> Self {
		InherentBuilder {
			id: self.id.clone(),
			client: self.client.clone(),
			backend: self.backend.clone(),
		}
	}

	fn clone_from(&mut self, _source: &Self) {
		todo!()
	}
}

impl<C> InherentBuilder<C, TFullBackend<PBlock>>
where
	C::Api: BlockBuilder<PBlock>
		+ ParachainHost<PBlock>
		+ BabeApi<PBlock>
		+ ApiExt<PBlock, StateBackend = <TFullBackend<PBlock> as BackendT<PBlock>>::State>
		+ TaggedTransactionQueue<PBlock>,
	C: 'static
		+ ProvideRuntimeApi<PBlock>
		+ BlockOf
		+ Send
		+ BlockBackend<PBlock>
		+ BlockIdTo<PBlock>
		+ Sync
		+ AuxStore
		+ UsageProvider<PBlock>
		+ BlockchainEvents<PBlock>
		+ HeaderBackend<PBlock>
		+ BlockImport<PBlock>
		+ CallApiAt<PBlock>
		+ sc_block_builder::BlockBuilderProvider<TFullBackend<PBlock>, PBlock, C>,
{
	pub async fn parachain_inherent(&self) -> Option<ParachainInherentData> {
		let parent = self.client.info().best_hash;
		let (chnl_sndr, _chnl_rcvr) = metered::channel(64);
		let dummy_handler = Handle::new(chnl_sndr);
		let relay_interface = RelayChainInProcessInterface::new(
			self.client.clone(),
			self.backend.clone(),
			Arc::new(NoNetwork {}),
			dummy_handler,
		);
		let api = self.client.runtime_api();
		let pvd = api
			.persisted_validation_data(
				&BlockId::Hash(parent),
				self.id,
				OccupiedCoreAssumption::TimedOut,
			)
			.unwrap()
			.unwrap();
		ParachainInherentData::create_at(parent, &relay_interface, &pvd, self.id).await
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
	Runtime: paras::Config + frame_system::Config,
	C::Api: BlockBuilder<Block>
		+ ParachainHost<Block>
		+ ApiExt<Block, StateBackend = B::State>
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
		+ BlockchainEvents<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>
		+ sc_block_builder::BlockBuilderProvider<B, Block, C>,
	for<'r> &'r C: BlockImport<Block, Transaction = TransactionFor<B, Block>>,
	A: TransactionPool<Block = Block, Hash = Block::Hash> + MaintainedTransactionPool + 'static,
{
	pub fn new<I, F>(initiator: I, setup: F) -> Self
	where
		I: Initiator<Block, Api = C::Api, Client = C, Backend = B, Pool = A, Executor = Exec>,
		F: FnOnce(Arc<C>) -> (CIDP, DP),
	{
		let (client, backend, pool, executor, task_manager) = initiator.init().unwrap();
		let (cidp, dp) = setup(client.clone());

		Self {
			builder: Builder::new(client, backend, pool, executor, task_manager),
			cidp,
			dp,
			next: None,
			imports: Vec::new(),
			parachains: Vec::new(),
			collations: Vec::new(),
			handle: manager.spawn_handle(),
			_phantom: Default::default(),
		}
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

	pub fn inherent_builder(&self, para_id: Id) -> InherentBuilder<C, B> {
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
	) -> Result<(), ()> {
		let FudgeParaChain { id, head, code } = para;
		self.with_mut_state(|| {
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

		self.parachains.push((id, collator));

		Ok(())
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

		let parent = self.builder.latest_header();
		let inherents = futures::executor::block_on(provider.create_inherent_data()).unwrap();
		let digest = self
			.with_state(|| {
				futures::executor::block_on(self.dp.create_digest(parent, inherents.clone()))
					.unwrap()
			})
			.unwrap();

		let Proposal { block, proof, .. } = self.builder.build_block(
			self.builder.handle(),
			inherents,
			digest,
			Duration::from_secs(60),
			6_000_000,
		);
		self.next = Some((block.clone(), proof));

		Ok(block)
	}

	pub fn import_block(&mut self) -> Result<(), ()> {
		let (block, proof) = self.next.take().unwrap();
		let (header, body) = block.clone().deconstruct();
		let mut params = BlockImportParams::new(BlockOrigin::NetworkInitialSync, header);
		params.body = Some(body);
		params.finalized = true;
		params.fork_choice = Some(ForkChoiceStrategy::Custom(true));

		self.builder.import_block(params).unwrap();
		self.imports.push((block.clone(), proof));
		self.collect_collations(block.header().clone());
		self.force_enact_collations()?;
		Ok(())
	}

	fn collect_collations(&mut self, parent_head: Block::Header) {
		let rt_api = self.client();
		let parent = self.client().info().best_hash;
		for (id, para) in self.parachains.iter() {
			let pvd = self
				.with_state(|| {
					persisted_validation_data(
						rt_api.clone(),
						parent,
						*id,
						OccupiedCoreAssumption::TimedOut,
					)
				})
				.unwrap();

			if let Some(collation) = para.collation(pvd) {
				if self.collations.iter().any(|(in_id, _, _)| in_id == id) {
					para.judge(CollationJudgement::Rejected)
				} else {
					self.collations.push((*id, collation, parent_head.clone()))
				}
			}
		}
	}

	fn force_enact_collations(&mut self) -> Result<(), ()> {
		let rt_api = self.client();
		let parent = self.client().info().best_hash;
		let parent_number = self.client().info().best_number;
		let collations = self.collations.clone();
		self.collations = Vec::new();

		for (id, collation, generation_parent) in collations {
			let pvd = self
				.with_state(|| {
					persisted_validation_data(
						rt_api.clone(),
						parent,
						id,
						OccupiedCoreAssumption::TimedOut,
					)
				})
				.unwrap();

			self.with_mut_state(|| {
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
							rt_api.clone(),
							parent,
							id,
							OccupiedCoreAssumption::TimedOut,
						),
					},
				};

				let core = CoreIndex(
					Parachains::get()
						.iter()
						.position(|in_id| *in_id == id)
						.expect("Parachain is onboarded. qed.")
						.try_into()
						.expect("Never more than u32::MAX parachains. qed"),
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
					cpa.using_encoded(|scale| {
						CandidatePendingAvailability::decode(&mut scale.as_ref())
							.expect("Same layout. qed.")
					}),
				);

				// NOTE: Generating the validation like this, force enacts a block
				let _ = persisted_validation_data(
					rt_api.clone(),
					parent,
					id,
					OccupiedCoreAssumption::Included,
				);
			})
			.map_err(|_| ())?;

			let index = self
				.parachains
				.iter()
				.position(|(in_id, _)| *in_id == id)
				.expect("Parachain is onbaorded. qed");
			let (_, collator) = self.parachains.get(index).expect("Index is existing. qed");
			collator.judge(CollationJudgement::Approved)
		}

		Ok(())
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

// TODO: Make this error instead
fn persisted_validation_data<RtApi, Block>(
	rt_api: Arc<RtApi>,
	parent: Block::Hash,
	id: Id,
	assumption: OccupiedCoreAssumption,
) -> PersistedValidationData
where
	Block: BlockT,
	RtApi: ProvideRuntimeApi<Block>,
	RtApi::Api: ParachainHost<Block>,
{
	let api = rt_api.runtime_api();
	api.persisted_validation_data(&BlockId::Hash(parent), id, assumption)
		.unwrap()
		.unwrap()
}

fn validation_code_hash<RtApi, Block>(
	rt_api: Arc<RtApi>,
	parent: Block::Hash,
	id: Id,
	assumption: OccupiedCoreAssumption,
) -> ValidationCodeHash
where
	Block: BlockT,
	RtApi: ProvideRuntimeApi<Block>,
	RtApi::Api: ParachainHost<Block>,
{
	let api = rt_api.runtime_api();
	api.validation_code_hash(&BlockId::Hash(parent), id, assumption)
		.unwrap()
		.unwrap()
}
