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
use std::{marker::PhantomData, sync::Arc};

use frame_support::{pallet_prelude::TransactionSource, sp_runtime::traits::NumberFor};
use sc_client_api::{
	backend::TransactionFor, blockchain::Backend as BlockchainBackend, AuxStore,
	Backend as BackendT, BlockBackend, BlockImportOperation, BlockOf, HeaderBackend, NewBlockState,
	UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ImportResult};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TFullClient, TaskManager, TransactionPool};
use sc_transaction_pool::{FullChainApi, RevalidationType};
use sc_transaction_pool_api::{ChainEvent, MaintainedTransactionPool};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, HashFor, ProvideRuntimeApi, StorageChanges};
use sp_block_builder::BlockBuilder;
use sp_consensus::{Environment, InherentData, Proposal, Proposer};
use sp_core::traits::CodeExecutor;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, BlockIdTo, Hash as HashT, Header as HeaderT, One, Zero},
	Digest,
};
use sp_state_machine::StorageProof;
use sp_std::time::Duration;
use sp_storage::StateVersion;
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use thiserror::Error;

use crate::{provider::ExternalitiesProvider, StoragePair};

const DEFAULT_BUILDER_LOG_TARGET: &str = "fudge-builder";

pub type InnerError = Box<dyn std::error::Error>;

#[derive(Error, Debug)]
pub enum Error<Block: sp_api::BlockT> {
	#[error("couldn't retrieve latest header: {0}")]
	LatestHeaderRetrieval(InnerError),

	#[error("latest header not found")]
	LatestHeaderNotFound,

	#[error("couldn't retrieve header at {0}: {1}")]
	HeaderRetrieval(BlockId<Block>, InnerError),

	#[error("header not found at {0}")]
	HeaderNotFound(BlockId<Block>),

	#[error("latest code not found")]
	LatestCodeNotFound,

	#[error("couldn't retrieve state at {0}: {1}")]
	StateRetrieval(BlockId<Block>, InnerError),

	#[error("couldn't retrieve block indexed body at {0}: {1}")]
	BlockIndexedBodyRetrieval(BlockId<Block>, InnerError),

	#[error("couldn't retrieve justifications at {0}: {1}")]
	JustificationsRetrieval(BlockId<Block>, InnerError),

	#[error("couldn't begin backend operation: {0}")]
	BackendBeginOperation(InnerError),

	#[error("couldn't begin state operation: {0}")]
	BackendBeginStateOperation(InnerError),

	#[error("couldn't revert backend operation: {0}")]
	BackendRevert(InnerError),

	#[error("couldn't commit backend operation: {0}")]
	BackendCommit(InnerError),

	#[error("couldn't retrieve state at {0}: {1}")]
	StateNotAvailable(BlockId<Block>, InnerError),

	#[error("couldn't retrieve block number at {0}: {1}")]
	BlockNumberRetrieval(BlockId<Block>, InnerError),

	#[error("block number not found at {0}")]
	BlockNumberNotFound(BlockId<Block>),

	#[error("couldn't update storage: {0}")]
	StorageUpdate(InnerError),

	#[error("couldn't update DB storage: {0}")]
	DBStorageUpdate(InnerError),

	#[error("couldn't update transaction index: {0}")]
	TransactionIndexUpdate(InnerError),

	#[error("couldn't set block data: {0}")]
	BlockDataSet(InnerError),

	#[error("couldn't submit extrinsic: {0}")]
	ExtrinsicSubmission(InnerError),

	#[error("couldn't initialize factory: {0}")]
	FactoryInitialization(InnerError),

	#[error("couldn't propose block: {0}")]
	BlockProposal(InnerError),

	#[error("couldn't import block: {0}")]
	BlockImporting(InnerError),
}

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash)]
pub enum Operation {
	Commit,
	DryRun,
}

#[derive(Clone)]
pub struct TransitionCache {
	auxiliary: Vec<StoragePair>,
}

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash)]
pub enum PoolState {
	Empty,
	Busy(usize),
}

pub struct Builder<
	Block: BlockT,
	RtApi,
	Exec,
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
	backend: Arc<B>,
	client: Arc<C>,
	pool: Arc<sc_transaction_pool::FullPool<Block, C>>,
	cache: TransitionCache,
	_phantom: PhantomData<(Block, RtApi, Exec)>,
}

impl<Block, RtApi, Exec, B, C> Builder<Block, RtApi, Exec, B, C>
where
	B: BackendT<Block> + 'static,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block>
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
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>
		+ sc_block_builder::BlockBuilderProvider<B, Block, C>,
{
	/// Create a new Builder with provided backend and client.
	pub fn new(backend: Arc<B>, client: Arc<C>, manager: &TaskManager) -> Self {
		let pool = Arc::new(
			sc_transaction_pool::FullPool::<Block, C>::with_revalidation_type(
				Default::default(),
				true.into(),
				Arc::new(FullChainApi::new(
					client.clone(),
					None,
					&manager.spawn_essential_handle(),
				)),
				None,
				RevalidationType::Full,
				manager.spawn_essential_handle(),
				client.usage_info().chain.best_number,
			),
		);

		Builder {
			backend,
			client,
			pool,
			cache: TransitionCache {
				auxiliary: Vec::new(),
			},
			_phantom: PhantomData::default(),
		}
	}

	pub fn client(&self) -> Arc<C> {
		self.client.clone()
	}

	pub fn backend(&self) -> Arc<B> {
		self.backend.clone()
	}

	pub fn latest_block(&self) -> Block::Hash {
		self.client.info().best_hash
	}

	pub fn latest_header(&self) -> Result<Block::Header, Error<Block>> {
		self.backend
			.blockchain()
			.header(BlockId::Hash(self.latest_block()))
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					error = ?e,
					"Couldn't retrieve latest header."
				);

				Error::LatestHeaderRetrieval(e.into())
			})?
			.ok_or({
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					"Latest header not found."
				);

				Error::LatestHeaderNotFound
			})
	}

	pub fn latest_code(&self) -> Result<Vec<u8>, Error<Block>> {
		self.with_state(Operation::DryRun, None, || {
			frame_support::storage::unhashed::get_raw(sp_storage::well_known_keys::CODE).ok_or({
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					"Latest code not found."
				);

				Error::LatestCodeNotFound
			})
		})?
	}

	pub fn commit_storage_changes(
		&mut self,
		changes: StorageChanges<B::State, Block>,
		at: BlockId<Block>,
	) -> Result<(), Error<Block>> {
		let mut op = self.backend.begin_operation().map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not begin backend operation."
			);
			Error::BackendBeginOperation(e.into())
		})?;

		self.backend
			.begin_state_operation(&mut op, at)
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not begin backend state operation."
				);

				Error::BackendBeginStateOperation(e.into())
			})?;

		let header = self.get_block_header(at)?;

		self.check_best_hash_and_revert(*header.number())?;

		self.mutate_normal(&mut op, changes, at)?;

		self.backend.commit_operation(op).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not commit backend operation."
			);

			Error::BackendCommit(e.into())
		})
	}

	pub fn with_state<R>(
		&self,
		op: Operation,
		at: Option<BlockId<Block>>,
		exec: impl FnOnce() -> R,
	) -> Result<R, Error<Block>> {
		let (state, at) = if let Some(req_at) = at {
			(self.backend.state_at(req_at), req_at)
		} else {
			let at = BlockId::Hash(self.client.info().best_hash);
			(self.backend.state_at(at.clone()), at)
		};

		let state = state.map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"State not available at {}.",
				at
			);

			Error::StateNotAvailable(at, e.into())
		})?;

		match op {
			Operation::Commit => {
				let mut op = self.backend.begin_operation().map_err(|e| {
					tracing::error!(
						target = DEFAULT_BUILDER_LOG_TARGET,
						error = ?e,
						"Could not begin backend operation."
					);

					Error::BackendBeginOperation(e.into())
				})?;

				self.backend
					.begin_state_operation(&mut op, at)
					.map_err(|e| {
						tracing::error!(
							target = DEFAULT_BUILDER_LOG_TARGET,
							error = ?e,
							"Could not begin backend state operation."
						);

						Error::BackendBeginStateOperation(e.into())
					})?;

				let block_number = self
					.backend
					.blockchain()
					.block_number_from_id(&at)
					.map_err(|e| {
						tracing::error!(
							target = DEFAULT_BUILDER_LOG_TARGET,
							error = ?e,
							"Could not retrieve block number for block at {}.",
							at,
						);

						Error::BlockNumberRetrieval(at, e.into())
					})?
					.ok_or(Error::BlockNumberNotFound(at))?;

				let res = if block_number == Zero::zero() {
					self.mutate_genesis(&mut op, &state, exec)
				} else {
					// We need to unfinalize the latest block and re-import it again in order to
					// mutate it
					self.check_best_hash_and_revert(NumberFor::<Block>::one())?;

					let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);

					let (r, changes) = ext.execute_with_mut(exec);

					self.mutate_normal(&mut op, changes, at)?;

					Ok(r)
				};

				self.backend.commit_operation(op).map_err(|e| {
					tracing::error!(
						target = DEFAULT_BUILDER_LOG_TARGET,
						error = ?e,
						"Could not commit backend operation."
					);

					Error::BackendCommit(e.into())
				})?;

				res
			}
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
	) -> Result<R, Error<Block>> {
		let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);
		let (r, changes) = ext.execute_with_mut(exec);
		let (_main_sc, _child_sc, _, tx, root, _tx_index) = changes.into_inner();

		// We nee this in order to UNSET commited
		// op.set_genesis_state(Storage::default(), true, StateVersion::V0)
		//	.unwrap();
		op.update_db_storage(tx).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update DB storage."
			);

			Error::DBStorageUpdate(e.into())
		})?;

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
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not set block data."
			);

			Error::BlockDataSet(e.into())
		})?;

		Ok(r)
	}

	fn mutate_normal(
		&self,
		op: &mut B::BlockImportOperation,
		changes: StorageChanges<B::State, Block>,
		at: BlockId<Block>,
	) -> Result<(), Error<Block>> {
		let chain_backend = self.backend.blockchain();

		let mut header = self.get_block_header(at)?;

		let (main_sc, child_sc, _, tx, root, tx_index) = changes.into_inner();

		header.set_state_root(root);

		op.update_db_storage(tx).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update DB storage."
			);

			Error::DBStorageUpdate(e.into())
		})?;

		op.update_storage(main_sc, child_sc).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update storage."
			);

			Error::StorageUpdate(e.into())
		})?;

		op.update_transaction_index(tx_index).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update transaction index."
			);

			Error::TransactionIndexUpdate(e.into())
		})?;

		let body = chain_backend.body(at).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not retrieve state at {}.",
				at
			);

			Error::StateRetrieval(at, e.into())
		})?;

		let indexed_body = chain_backend.block_indexed_body(at).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not retrieve block indexed body at {}.",
				at
			);

			Error::BlockIndexedBodyRetrieval(at, e.into())
		})?;

		let justifications = chain_backend.justifications(at).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not retrieve justifications at {}.",
				at
			);

			Error::JustificationsRetrieval(at, e.into())
		})?;

		// TODO: We set as final, this might not be correct.
		op.set_block_data(
			header,
			body,
			indexed_body,
			justifications,
			NewBlockState::Final,
		)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not set block data."
			);

			Error::BlockDataSet(e.into())
		})?;

		Ok(())
	}

	/// Append a given set of key-value-pairs into the builder cache
	pub fn append_transition(&mut self, trans: StoragePair) {
		self.cache.auxiliary.push(trans);
	}

	/// Caches a given extrinsic in the builder. The extrinsic will be
	pub fn append_extrinsic(&mut self, ext: Block::Extrinsic) -> Result<Block::Hash, Error<Block>> {
		let fut = self.pool.submit_one(
			&BlockId::Hash(self.client.info().best_hash),
			TransactionSource::External,
			ext,
		);

		futures::executor::block_on(fut).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not submit extrinsic."
			);

			Error::ExtrinsicSubmission(e.into())
		})
	}

	pub fn pool_state(&self) -> PoolState {
		let num_xts = self.pool.ready().fold(0, |sum, _| sum + 1);
		if num_xts == 0 {
			PoolState::Empty
		} else {
			PoolState::Busy(num_xts)
		}
	}

	/// Create a block from a given state of the Builder.
	pub fn build_block(
		&mut self,
		handle: SpawnTaskHandle,
		inherents: InherentData,
		digest: Digest,
		time: Duration,
		limit: usize,
	) -> Result<Proposal<Block, TransactionFor<B, Block>, StorageProof>, Error<Block>> {
		let mut factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
			handle,
			self.client.clone(),
			self.pool.clone(),
			None,
			None,
		);

		let header = self.get_block_header(BlockId::Hash(self.latest_block()))?;

		let proposer = futures::executor::block_on(factory.init(&header)).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not initialize factory."
			);

			Error::FactoryInitialization(e.into())
		})?;

		futures::executor::block_on(proposer.propose(inherents, digest, time, Some(limit))).map_err(
			|e| {
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not propose block."
				);

				Error::BlockProposal(e.into())
			},
		)
	}

	/// Import a block, that has been previosuly build
	pub fn import_block(
		&mut self,
		params: BlockImportParams<Block, C::Transaction>,
	) -> Result<(), Error<Block>> {
		let prev_hash = self.latest_block();

		// TODO: This works but is pretty dirty and unsafe. I am not sure, why the BlockImport needs a mut client
		//       as the final implementation actually uses &mut &Client, making the actual trait definition absurd.
		//       This does not work here as I am working on an abstract C, or does it?
		//       When using the Arc<T> impl of block import the compiler complains that the given C here does not
		//       satisfy the trait bound for<'r> BlockImport. Given that C is 'static understandable and not able to
		//       be satisfied by a generic C.
		let client = self.client.as_ref() as *const C as *mut C;
		let client = unsafe { &mut *(client) };
		let ret = match futures::executor::block_on(BlockImport::import_block(
			client,
			params,
			Default::default(),
		))
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not import block."
			);

			Error::BlockImporting(e.into())
		})? {
			ImportResult::Imported(_) => Ok(()),
			ImportResult::AlreadyInChain => Ok(()),
			ImportResult::KnownBad => Err(()),
			ImportResult::UnknownParent => Err(()),
			ImportResult::MissingState => Err(()),
		};

		// TODO: The transitions are currently not used when importing
		//       Best would be to:
		//           - import block
		//           - with_mut_state(inject transitions)

		// Trigger pool maintenance
		//
		// We do not re-org and we do always finalize directly. So no actual
		// "routes" provided here.
		if ret.is_ok() {
			let best_hash = self.latest_block();
			let _number = self.client.info().best_number;
			futures::executor::block_on(self.pool.maintain(ChainEvent::NewBestBlock {
				hash: best_hash,
				tree_route: None,
			}));
			let route = [prev_hash];
			futures::executor::block_on(self.pool.maintain(ChainEvent::Finalized {
				hash: best_hash,
				tree_route: Arc::new(route),
			}));
		};

		Ok(())
	}

	fn get_block_header(&self, at: BlockId<Block>) -> Result<Block::Header, Error<Block>> {
		self.backend
			.blockchain()
			.header(at)
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not get header at {}.",
					at,
				);

				Error::HeaderRetrieval(at, e.into())
			})?
			.ok_or({
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					"Header not found at {}",
					at,
				);

				Error::HeaderNotFound(at)
			})
	}

	fn check_best_hash_and_revert(&self, n: NumberFor<Block>) -> Result<(), Error<Block>> {
		let info = self.client.info();

		if info.best_hash == info.finalized_hash {
			self.backend.revert(n, true).map_err(|e| {
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not revert operation."
				);

				Error::BackendRevert(e.into())
			})?;
		}

		Ok(())
	}
}
