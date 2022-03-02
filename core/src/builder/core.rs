extern crate sc_client_api;
extern crate sc_client_db;
extern crate sc_consensus;
extern crate sc_service;
extern crate sp_api;
extern crate sp_consensus;
extern crate sp_runtime;

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

use crate::provider::ExternalitiesProvider;
use codec::Encode;
use frame_support::dispatch::TransactionPriority;
use frame_support::pallet_prelude::{TransactionLongevity, TransactionSource, TransactionTag};
use frame_support::sp_runtime::traits::NumberFor;
use sc_client_api::{AuxStore, Backend as BackendT, BlockOf, HeaderBackend, UsageProvider};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams};
use sc_executor::RuntimeVersionOf;
use sc_service::TFullClient;
use sc_transaction_pool_api::{PoolStatus, ReadyTransactions};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::Environment;
use sp_core::traits::CodeExecutor;
use sp_runtime::{generic::BlockId, traits::One};
use sp_state_machine::StorageProof;
use sp_std::time::Duration;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use self::sc_client_api::blockchain::Backend as BlockchainBackend;
use self::sc_client_api::{BlockImportOperation, NewBlockState};
use self::sc_consensus::ImportResult;
use self::sc_service::{InPoolTransaction, SpawnTaskHandle, TransactionPool};
use self::sp_api::HashFor;
use self::sp_consensus::{InherentData, Proposal, Proposer};
use self::sp_runtime::traits::{Block as BlockT, Hash as HashT, Header as HeaderT, Zero};
use self::sp_runtime::Digest;
use crate::StoragePair;
use sc_client_api::backend::TransactionFor;
use sp_storage::StateVersion;

pub enum Operation {
	Commit,
	DryRun,
}

#[derive(Clone)]
pub struct SimplePool<Block: BlockT> {
	pool: Vec<Block::Extrinsic>,
}

impl<Block: BlockT> SimplePool<Block> {
	fn new() -> Self {
		SimplePool { pool: Vec::new() }
	}

	fn push(&mut self, xt: Block::Extrinsic) {
		self.pool.push(xt)
	}
}

pub struct ExtWrapper<Block: BlockT> {
	xt: Block::Extrinsic,
}

impl<Block: BlockT> ExtWrapper<Block> {
	pub fn new(xt: Block::Extrinsic) -> Self {
		Self { xt }
	}
}

impl<Block: BlockT> InPoolTransaction for ExtWrapper<Block> {
	type Transaction = Block::Extrinsic;
	type Hash = Block::Hash;

	fn data(&self) -> &Self::Transaction {
		&self.xt
	}

	fn hash(&self) -> &Self::Hash {
		todo!()
	}

	fn priority(&self) -> &TransactionPriority {
		todo!()
	}

	fn longevity(&self) -> &TransactionLongevity {
		todo!()
	}

	fn requires(&self) -> &[TransactionTag] {
		todo!()
	}

	fn provides(&self) -> &[TransactionTag] {
		todo!()
	}

	fn is_propagable(&self) -> bool {
		todo!()
	}
}

#[derive(Debug)]
pub enum Error {}

impl From<sc_transaction_pool_api::error::Error> for Error {
	fn from(_: sc_transaction_pool_api::error::Error) -> Self {
		todo!()
	}
}

impl Display for Error {
	fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
		todo!()
	}
}

impl std::error::Error for Error {}

impl sc_transaction_pool_api::error::IntoPoolError for Error {}

impl<Block: BlockT> Iterator for SimplePool<Block> {
	type Item = Arc<ExtWrapper<Block>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.pool
			.pop()
			.and_then(|xt| Some(Arc::new(ExtWrapper::new(xt))))
	}
}

impl<Block: BlockT> ReadyTransactions for SimplePool<Block> {
	fn report_invalid(&mut self, _tx: &Self::Item) {
		todo!()
	}
}

impl<Block: BlockT> TransactionPool for SimplePool<Block> {
	type Block = Block;
	type Hash = Block::Hash;
	type InPoolTransaction = ExtWrapper<Block>;
	type Error = Error;

	fn submit_at(
		&self,
		_at: &BlockId<Self::Block>,
		_source: TransactionSource,
		_xts: Vec<sc_transaction_pool_api::TransactionFor<Self>>,
	) -> sc_transaction_pool_api::PoolFuture<
		Vec<Result<sc_transaction_pool_api::TxHash<Self>, Self::Error>>,
		Self::Error,
	> {
		todo!()
	}

	fn submit_one(
		&self,
		_at: &BlockId<Self::Block>,
		_source: TransactionSource,
		_xt: sc_transaction_pool_api::TransactionFor<Self>,
	) -> sc_transaction_pool_api::PoolFuture<sc_transaction_pool_api::TxHash<Self>, Self::Error> {
		todo!()
	}

	fn submit_and_watch(
		&self,
		_at: &BlockId<Self::Block>,
		_source: TransactionSource,
		_xt: sc_transaction_pool_api::TransactionFor<Self>,
	) -> sc_transaction_pool_api::PoolFuture<
		Pin<Box<sc_transaction_pool_api::TransactionStatusStreamFor<Self>>>,
		Self::Error,
	> {
		todo!()
	}

	fn ready_at(
		&self,
		_at: NumberFor<Self::Block>,
	) -> Pin<
		Box<
			dyn Future<
					Output = Box<
						dyn sc_transaction_pool_api::ReadyTransactions<
								Item = Arc<Self::InPoolTransaction>,
							> + Send,
					>,
				> + Send,
		>,
	> {
		let i = Box::new(std::iter::empty::<Arc<Self::InPoolTransaction>>())
			as Box<
				(dyn ReadyTransactions<Item = Arc<ExtWrapper<Block>>>
				     + std::marker::Send
				     + 'static),
			>;
		Box::pin(async { i })
	}

	fn ready(
		&self,
	) -> Box<
		dyn sc_transaction_pool_api::ReadyTransactions<Item = Arc<Self::InPoolTransaction>> + Send,
	> {
		Box::new(self.clone())
	}

	fn remove_invalid(
		&self,
		_hashes: &[sc_transaction_pool_api::TxHash<Self>],
	) -> Vec<Arc<Self::InPoolTransaction>> {
		Vec::new()
	}

	fn status(&self) -> sc_transaction_pool_api::PoolStatus {
		PoolStatus {
			ready: self.pool.len(),
			ready_bytes: self
				.pool
				.iter()
				.fold(0, |weight, xt| weight + xt.size_hint()),
			future: 0,
			future_bytes: 0,
		}
	}

	fn import_notification_stream(
		&self,
	) -> sc_transaction_pool_api::ImportNotificationStream<sc_transaction_pool_api::TxHash<Self>> {
		todo!()
	}

	fn on_broadcasted(
		&self,
		_propagations: HashMap<sc_transaction_pool_api::TxHash<Self>, Vec<String>>,
	) {
		todo!()
	}

	fn hash_of(
		&self,
		_xt: &sc_transaction_pool_api::TransactionFor<Self>,
	) -> sc_transaction_pool_api::TxHash<Self> {
		todo!()
	}

	fn ready_transaction(
		&self,
		_hash: &sc_transaction_pool_api::TxHash<Self>,
	) -> Option<Arc<Self::InPoolTransaction>> {
		todo!()
	}
}

pub struct TransitionCache<Block: BlockT> {
	extrinsics: Arc<SimplePool<Block>>,
	auxilliary: Vec<StoragePair>,
}

pub struct Builder<
	Block: BlockT,
	RtApi,
	Exec,
	B = Backend<Block>,
	C = TFullClient<Block, RtApi, Exec>,
> {
	backend: Arc<B>,
	client: Arc<C>,
	cache: TransitionCache<Block>,
	_phantom: PhantomData<(Block, RtApi, Exec)>,
}

impl<Block, RtApi, Exec, B, C> Builder<Block, RtApi, Exec, B, C>
where
	B: BackendT<Block> + 'static,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block> + ApiExt<Block, StateBackend = B::State>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>
		+ sc_block_builder::BlockBuilderProvider<B, Block, C>,
{
	/// Create a new Builder with provided backend and client.
	pub fn new(backend: Arc<B>, client: Arc<C>) -> Self {
		Builder {
			backend: backend,
			client: client,
			cache: TransitionCache {
				extrinsics: Arc::new(SimplePool::new()),
				auxilliary: Vec::new(),
			},
			_phantom: PhantomData::default(),
		}
	}

	pub fn latest_block(&self) -> Block::Hash {
		self.client.info().best_hash
	}

	pub fn latest_header(&self) -> Block::Header {
		self.backend
			.blockchain()
			.header(BlockId::Hash(self.latest_block()))
			.ok()
			.flatten()
			.expect("State is available. qed")
	}

	pub fn latest_code(&self) -> Vec<u8> {
		self.with_state(Operation::DryRun, None, || {
			frame_support::storage::unhashed::get_raw(sp_storage::well_known_keys::CODE).unwrap()
		})
		.unwrap()
	}

	pub fn with_state<R>(
		&self,
		op: Operation,
		at: Option<BlockId<Block>>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		let (state, at) = if let Some(req_at) = at {
			(self.backend.state_at(req_at), req_at)
		} else {
			let at = BlockId::Hash(self.client.info().best_hash);
			(self.backend.state_at(at.clone()), at)
		};

		let state = state.map_err(|_| "State at INSERT_AT_HERE not available".to_string())?;

		match op {
			Operation::Commit => {
				let mut op = self
					.backend
					.begin_operation()
					.map_err(|_| "Unable to start state-operation on backend".to_string())?;
				self.backend.begin_state_operation(&mut op, at).unwrap();

				let res = if self
					.backend
					.blockchain()
					.block_number_from_id(&at)
					.unwrap()
					.unwrap() == Zero::zero()
				{
					self.mutate_genesis(&mut op, &state, exec)
				} else {
					// We need to unfinalize the latest block and re-import it again in order to
					// mutate it
					let info = self.client.info();
					if info.best_hash == info.finalized_hash {
						self.backend
							.revert(NumberFor::<Block>::one(), true)
							.unwrap();
					}
					self.mutate_normal(&mut op, &state, exec, at)
				};

				self.backend
					.commit_operation(op)
					.map_err(|_| "Unable to commit state-operation on backend".to_string())?;

				res
			}
			// TODO: Does this actually NOT change the state?
			Operation::DryRun => Ok(
				ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state).execute_with(exec),
			),
		}
	}

	fn mutate_genesis<R>(
		&self,
		op: &mut B::BlockImportOperation,
		state: &B::State,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);
		let (r, changes) = ext.execute_with_mut(exec);
		let (_main_sc, _child_sc, _, tx, root, _tx_index) = changes.into_inner();

		// We nee this in order to UNSET commited
		// op.set_genesis_state(Storage::default(), true, StateVersion::V0)
		//	.unwrap();
		op.update_db_storage(tx).unwrap();

		let genesis_block = Block::new(
			Block::Header::new(
				Zero::zero(),
				<<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
					Vec::new(),
					StateVersion::V0,
				),
				root,
				Default::default(),
				Default::default(),
			),
			Default::default(),
		);

		op.set_block_data(
			genesis_block.deconstruct().0,
			Some(vec![]),
			None,
			None,
			NewBlockState::Final,
		)
		.map_err(|_| "Could not set block data".to_string())?;

		Ok(r)
	}

	fn mutate_normal<R>(
		&self,
		op: &mut B::BlockImportOperation,
		state: &B::State,
		exec: impl FnOnce() -> R,
		at: BlockId<Block>,
	) -> Result<R, String> {
		let chain_backend = self.backend.blockchain();
		let mut header = chain_backend
			.header(at)
			.ok()
			.flatten()
			.expect("State is available. qed");

		let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);
		let (r, changes) = ext.execute_with_mut(exec);

		let (main_sc, child_sc, _, tx, root, tx_index) = changes.into_inner();

		header.set_state_root(root);
		op.update_db_storage(tx).unwrap();
		op.update_storage(main_sc, child_sc)
			.map_err(|_| "Updating storage not possible.")
			.unwrap();
		op.update_transaction_index(tx_index)
			.map_err(|_| "Updating transaction index not possible.")
			.unwrap();

		let body = chain_backend.body(at).expect("State is available. qed.");
		let indexed_body = chain_backend
			.block_indexed_body(at)
			.expect("State is available. qed.");
		let justifications = chain_backend
			.justifications(at)
			.expect("State is available. qed.");

		// TODO: We set as final, this might not be correct.
		op.set_block_data(
			header,
			body,
			indexed_body,
			justifications,
			NewBlockState::Final,
		)
		.unwrap();
		Ok(r)
	}

	/// Append a given set of key-value-pairs into the builder cache
	pub fn append_transition(&mut self, trans: StoragePair) -> &mut Self {
		self.cache.auxilliary.push(trans);
		self
	}

	/// Caches a given extrinsic in the builder. The extrinsic will be
	pub fn append_extrinsic(&mut self, ext: Block::Extrinsic) -> &mut Self {
		// TODO: Handle this with mutex instead of this hack
		let pt =
			self.cache.extrinsics.as_ref() as *const SimplePool<Block> as *mut SimplePool<Block>;
		let pool = unsafe { &mut *(pt) };
		pool.push(ext);
		self
	}

	/// Create a block from a given state of the Builder.
	pub fn build_block(
		&mut self,
		handle: SpawnTaskHandle,
		inherents: InherentData,
		digest: Digest,
		time: Duration,
		limit: usize,
	) -> Proposal<Block, TransactionFor<B, Block>, StorageProof> {
		let mut factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
			handle,
			self.client.clone(),
			self.cache.extrinsics.clone(),
			None,
			None,
		);
		let header = self
			.backend
			.blockchain()
			.header(BlockId::Hash(self.latest_block()))
			.ok()
			.flatten()
			.expect("State is available. qed");
		let proposer = futures::executor::block_on(factory.init(&header)).unwrap();
		futures::executor::block_on(proposer.propose(inherents, digest, time, Some(limit))).unwrap()

		// TODO: -pool implementation correctly
		//       - auxiliary data (Check if those even go into state?)
		//           - If NOT: Append set-storage calls here manually. This will of course prevent syncing, but we don't care
	}

	/// Import a block, that has been previosuly build
	pub fn import_block(
		&mut self,
		params: BlockImportParams<Block, C::Transaction>,
	) -> Result<(), ()> {
		// TODO: This works but is pretty dirty and unsafe. I am not sure, why the BlockImport needs a mut client
		//       Check if I can put the client into a Mutex
		let client = self.client.as_ref() as *const C as *mut C;
		let client = unsafe { &mut *(client) };
		match futures::executor::block_on(client.import_block(params, Default::default())).unwrap()
		{
			ImportResult::Imported(_) => Ok(()),
			ImportResult::AlreadyInChain => Err(()),
			ImportResult::KnownBad => Err(()),
			ImportResult::UnknownParent => Err(()),
			ImportResult::MissingState => Err(()),
		}
	}
}

// TODO: Nice code examples that could help implementing this idea of taking over a chain locally
// This should be miminced
/*
fn execute_and_import_block(
	&self,
	operation: &mut ClientImportOperation<Block, B>,
	origin: BlockOrigin,
	hash: Block::Hash,
	import_headers: PrePostHeader<Block::Header>,
	justifications: Option<Justifications>,
	body: Option<Vec<Block::Extrinsic>>,
	indexed_body: Option<Vec<Vec<u8>>>,
	storage_changes: Option<
		sc_consensus::StorageChanges<Block, backend::TransactionFor<B, Block>>,
	>,
	new_cache: HashMap<CacheKeyId, Vec<u8>>,
	finalized: bool,
	aux: Vec<(Vec<u8>, Option<Vec<u8>>)>,
	fork_choice: ForkChoiceStrategy,
	import_existing: bool,
) -> sp_blockchain::Result<ImportResult>
	where
		Self: ProvideRuntimeApi<Block>,
		<Self as ProvideRuntimeApi<Block>>::Api:
		CoreApi<Block> + ApiExt<Block, StateBackend = B::State>,
{
	let parent_hash = import_headers.post().parent_hash().clone();
	let status = self.backend.blockchain().status(BlockId::Hash(hash))?;
	let parent_exists = self.backend.blockchain().status(BlockId::Hash(parent_hash))? ==
		blockchain::BlockStatus::InChain;
	match (import_existing, status) {
		(false, blockchain::BlockStatus::InChain) => return Ok(ImportResult::AlreadyInChain),
		(false, blockchain::BlockStatus::Unknown) => {},
		(true, blockchain::BlockStatus::InChain) => {},
		(true, blockchain::BlockStatus::Unknown) => {},
	}

	let info = self.backend.blockchain().info();
	let gap_block = info
		.block_gap
		.map_or(false, |(start, _)| *import_headers.post().number() == start);

	assert!(justifications.is_some() && finalized || justifications.is_none() || gap_block);

	// the block is lower than our last finalized block so it must revert
	// finality, refusing import.
	if status == blockchain::BlockStatus::Unknown &&
		*import_headers.post().number() <= info.finalized_number &&
		!gap_block
	{
		return Err(sp_blockchain::Error::NotInFinalizedChain)
	}

	// this is a fairly arbitrary choice of where to draw the line on making notifications,
	// but the general goal is to only make notifications when we are already fully synced
	// and get a new chain head.
	let make_notifications = match origin {
		BlockOrigin::NetworkBroadcast | BlockOrigin::Own | BlockOrigin::ConsensusBroadcast =>
			true,
		BlockOrigin::Genesis | BlockOrigin::NetworkInitialSync | BlockOrigin::File => false,
	};

	let storage_changes = match storage_changes {
		Some(storage_changes) => {
			let storage_changes = match storage_changes {
				sc_consensus::StorageChanges::Changes(storage_changes) => {
					self.backend
						.begin_state_operation(&mut operation.op, BlockId::Hash(parent_hash))?;
					let (main_sc, child_sc, offchain_sc, tx, _, changes_trie_tx, tx_index) =
						storage_changes.into_inner();

					if self.config.offchain_indexing_api {
						operation.op.update_offchain_storage(offchain_sc)?;
					}

					operation.op.update_db_storage(tx)?;
					operation.op.update_storage(main_sc.clone(), child_sc.clone())?;
					operation.op.update_transaction_index(tx_index)?;

					if let Some(changes_trie_transaction) = changes_trie_tx {
						operation.op.update_changes_trie(changes_trie_transaction)?;
					}
					Some((main_sc, child_sc))
				},
				sc_consensus::StorageChanges::Import(changes) => {
					let storage = sp_storage::Storage {
						top: changes.state.into_iter().collect(),
						children_default: Default::default(),
					};

					let state_root = operation.op.reset_storage(storage)?;
					if state_root != *import_headers.post().state_root() {
						// State root mismatch when importing state. This should not happen in
						// safe fast sync mode, but may happen in unsafe mode.
						warn!("Error imporing state: State root mismatch.");
						return Err(Error::InvalidStateRoot)
					}
					None
				},
			};
			// Ensure parent chain is finalized to maintain invariant that
			// finality is called sequentially. This will also send finality
			// notifications for top 250 newly finalized blocks.
			if finalized && parent_exists {
				self.apply_finality_with_block_hash(
					operation,
					parent_hash,
					None,
					info.best_hash,
					make_notifications,
				)?;
			}

			operation.op.update_cache(new_cache);
			storage_changes
		},
		None => None,
	};

	let is_new_best = !gap_block &&
		(finalized ||
			match fork_choice {
				ForkChoiceStrategy::LongestChain =>
					import_headers.post().number() > &info.best_number,
				ForkChoiceStrategy::Custom(v) => v,
			});

	let leaf_state = if finalized {
		NewBlockState::Final
	} else if is_new_best {
		NewBlockState::Best
	} else {
		NewBlockState::Normal
	};

	let tree_route = if is_new_best && info.best_hash != parent_hash && parent_exists {
		let route_from_best =
			sp_blockchain::tree_route(self.backend.blockchain(), info.best_hash, parent_hash)?;
		Some(route_from_best)
	} else {
		None
	};

	trace!(
		"Imported {}, (#{}), best={}, origin={:?}",
		hash,
		import_headers.post().number(),
		is_new_best,
		origin,
	);

	operation.op.set_block_data(
		import_headers.post().clone(),
		body,
		indexed_body,
		justifications,
		leaf_state,
	)?;

	operation.op.insert_aux(aux)?;

	// we only notify when we are already synced to the tip of the chain
	// or if this import triggers a re-org
	if make_notifications || tree_route.is_some() {
		if finalized {
			operation.notify_finalized.push(hash);
		}

		operation.notify_imported = Some(ImportSummary {
			hash,
			origin,
			header: import_headers.into_post(),
			is_new_best,
			storage_changes,
			tree_route,
		})
	}

	Ok(ImportResult::imported(is_new_best))
}


/// Verify a justification of a block
#[async_trait::async_trait]
pub trait Verifier<B: BlockT>: Send + Sync {
	/// Verify the given data and return the BlockImportParams and an optional
	/// new set of validators to import. If not, err with an Error-Message
	/// presented to the User in the logs.
	async fn verify(
		&mut self,
		block: BlockImportParams<B, ()>,
	) -> Result<(BlockImportParams<B, ()>, Option<Vec<(CacheKeyId, Vec<u8>)>>), String>;
}

/// Build a genesis Block
	let storage = chain_spec.build_storage()?;

	let child_roots = storage.children_default.iter().map(|(sk, child_content)| {
		let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
			child_content.data.clone().into_iter().collect(),
		);
		(sk.clone(), state_root.encode())
	});
	let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
		storage.top.clone().into_iter().chain(child_roots).collect(),
	);

	let extrinsics_root =
		<<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(Vec::new());

	Ok(Block::new(
		<<Block as BlockT>::Header as HeaderT>::new(
			Zero::zero(),
			extrinsics_root,
			state_root,
			Default::default(),
			Default::default(),
		),
		Default::default(),
	))

// The actual importing logic lies in the block_import queue and used "pub(crate) async fn import_single_block_metered(...) "
	let cache = HashMap::from_iter(maybe_keys.unwrap_or_default());
	let import_block = import_block.clear_storage_changes_and_mutate();
	let imported = import_handle.import_block(import_block, cache).await;



sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<
			Block,
			StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
		> + sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>,
	sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,

 */
