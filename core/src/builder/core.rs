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

use std::{collections::hash_map::DefaultHasher, marker::PhantomData, sync::Arc};

use frame_support::{pallet_prelude::TransactionSource, sp_runtime::traits::NumberFor};
use sc_client_api::{
	backend::TransactionFor, blockchain::Backend as BlockchainBackend, AuxStore,
	Backend as BackendT, BlockBackend, BlockImportOperation, BlockOf, HeaderBackend, NewBlockState,
	StateBackend, UsageProvider,
};
use sc_consensus::{BlockImport, BlockImportParams, ImportResult};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TaskManager, TransactionPool};
use sc_transaction_pool_api::{ChainEvent, MaintainedTransactionPool};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, HashFor, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::{Environment, InherentData, Proposal, Proposer};
use sp_core::traits::CodeExecutor;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, BlockIdTo, Hash as HashT, Header as HeaderT, One, Zero},
	Digest,
};
use sp_state_machine::{StorageChanges, StorageProof};
use sp_std::time::Duration;
use sp_storage::StateVersion;
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use thiserror::Error;

use crate::{provider::externalities::ExternalitiesProvider, types::StoragePair};

const DEFAULT_BUILDER_LOG_TARGET: &str = "fudge-builder";

pub type InnerError = Box<dyn std::error::Error>;

#[derive(Error, Debug)]
pub enum Error<Block: sp_api::BlockT> {
	#[error("latest header retrieval: {0}")]
	LatestHeaderRetrieval(InnerError),

	#[error("latest header not found")]
	LatestHeaderNotFound,

	#[error("header retrieval at {0}: {1}")]
	HeaderRetrieval(BlockId<Block>, InnerError),

	#[error("header not found at {0}")]
	HeaderNotFound(BlockId<Block>),

	#[error("latest code not found")]
	LatestCodeNotFound,

	#[error("state retrieval at {0}: {1}")]
	StateRetrieval(BlockId<Block>, InnerError),

	#[error("block indexed body retrieval at {0}: {1}")]
	BlockIndexedBodyRetrieval(BlockId<Block>, InnerError),

	#[error("justifications retrieval at {0}: {1}")]
	JustificationsRetrieval(BlockId<Block>, InnerError),

	#[error("backend operation start at {0}: {1}")]
	BackendOperationStart(BlockId<Block>, InnerError),

	#[error("backend state operation start at {0}: {1}")]
	BackendStateOperationStart(BlockId<Block>, InnerError),

	#[error("backend operation reversal: {0}")]
	BackendOperationReversal(InnerError),

	#[error("backend operation commit at {0}: {1}")]
	BackendOperationCommit(BlockId<Block>, InnerError),

	#[error("state not available at {0}: {1}")]
	StateNotAvailable(BlockId<Block>, InnerError),

	#[error("block number retrieval at {0}: {1}")]
	BlockNumberRetrieval(BlockId<Block>, InnerError),

	#[error("block number not found at {0}")]
	BlockNumberNotFound(BlockId<Block>),

	#[error("externalities execution at {0:?}: {1}")]
	ExternalitiesExecution(Option<BlockId<Block>>, InnerError),

	#[error("storage update at {0}: {1}")]
	StorageUpdate(BlockId<Block>, InnerError),

	#[error("DB storage update at {0:?}: {1}")]
	DBStorageUpdate(Option<BlockId<Block>>, InnerError),

	#[error("transaction index update at {0}: {1}")]
	TransactionIndexUpdate(BlockId<Block>, InnerError),

	#[error("block data set at {0:?}: {1}")]
	BlockDataSet(Option<BlockId<Block>>, InnerError),

	#[error("extrinsic submission: {0}")]
	ExtrinsicSubmission(InnerError),

	#[error("factory initialization: {0}")]
	FactoryInitialization(InnerError),

	#[error("block proposal: {0}")]
	BlockProposal(InnerError),

	#[error("block importing: {0}")]
	BlockImporting(InnerError),
}

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash)]
pub enum Operation {
	Commit,
	DryRun,
}

#[derive(Clone)]
struct TransitionCache {
	auxilliary: Vec<StoragePair>,
}

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash)]
pub enum PoolState {
	Empty,
	Busy(usize),
}

pub struct Builder<Block, RtApi, Exec, B, C, A> {
	backend: Arc<B>,
	client: Arc<C>,
	pool: Arc<A>,
	executor: Exec,
	task_manager: TaskManager,
	cache: TransitionCache,
	_phantom: PhantomData<(Block, RtApi)>,
}

impl<Block, RtApi, Exec, B, C, A> Builder<Block, RtApi, Exec, B, C, A>
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
	for<'r> &'r C: BlockImport<Block, Transaction = TransactionFor<B, Block>>,
	A: TransactionPool<Block = Block, Hash = Block::Hash> + MaintainedTransactionPool + 'static,
{
	/// Create a new Builder with provided backend and client.
	pub fn new(
		client: Arc<C>,
		backend: Arc<B>,
		pool: Arc<A>,
		executor: Exec,
		task_manager: TaskManager,
	) -> Self {
		Builder {
			backend,
			client,
			pool,
			executor,
			task_manager,
			cache: TransitionCache {
				auxilliary: Vec::new(),
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
					"Could not retrieve latest header."
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
				"Could not begin backend operation at {}.",
				at,
			);
			Error::BackendOperationStart(at, e.into())
		})?;

		self.backend
			.begin_state_operation(&mut op, at)
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not begin backend state operation at {}.",
					at,
				);

				Error::BackendStateOperationStart(at, e.into())
			})?;

		let header = self.get_block_header(at)?;

		self.check_best_hash_and_revert(*header.number())?;

		self.mutate_normal(&mut op, changes, at)?;

		self.backend.commit_operation(op).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not commit backend operation at {}.",
				at,
			);

			Error::BackendOperationCommit(at, e.into())
		})
	}

	pub fn handle(&self) -> SpawnTaskHandle {
		self.task_manager.spawn_handle()
	}

	fn state_version(&self) -> StateVersion {
		let wasm = self.latest_code();
		let code_fetcher = sp_core::traits::WrappedRuntimeCode(wasm.as_slice().into());
		let runtime_code = sp_core::traits::RuntimeCode {
			code_fetcher: &code_fetcher,
			heap_pages: None,
			hash: {
				use std::hash::{Hash, Hasher};
				let mut state = DefaultHasher::new();
				wasm.hash(&mut state);
				state.finish().to_le_bytes().to_vec()
			},
		};
		let mut ext = sp_state_machine::BasicExternalities::new_empty(); // just to read runtime version.
		let runtime_version =
			RuntimeVersionOf::runtime_version(&self.executor, &mut ext, &runtime_code).unwrap();
		runtime_version.state_version()
	}

	pub fn with_state<R>(
		&self,
		op: Operation,
		at: Option<BlockId<Block>>,
		exec: impl FnOnce() -> R,
	) -> Result<R, Error<Block>> {
		let block = match at {
			Some(BlockId::Hash(req_at)) => req_at,
			Some(BlockId::Number(req_at)) => {
				self.backend.blockchain().hash(req_at).unwrap().unwrap()
			}
			_ => self.client.info().best_hash,
		};
		let state = self.backend.state_at(block);
		let at = sp_api::BlockId::Hash(block);

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
						"Could not begin backend operation at {}.",
						at,
					);

					Error::BackendOperationStart(at, e.into())
				})?;

				self.backend
					.begin_state_operation(&mut op, at)
					.map_err(|e| {
						tracing::error!(
							target = DEFAULT_BUILDER_LOG_TARGET,
							error = ?e,
							"Could not begin backend state operation at {}.",
							at,
						);

						Error::BackendStateOperationStart(at, e.into())
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
					self.mutate_genesis::<R>(&mut op, &state, exec)
				} else {
					// We need to unfinalize the latest block and re-import it again in order to
					// mutate it
					self.check_best_hash_and_revert(NumberFor::<Block>::one())?;

					let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);

					let (r, changes) = ext.execute_with_mut(exec).map_err(|e| {
						tracing::error!(
							target = DEFAULT_BUILDER_LOG_TARGET,
							error = ?e,
							"Could not execute externalities at {:?}.",
							at,
						);

						Error::ExternalitiesExecution(Some(at), e.into())
					})?;

					self.mutate_normal(&mut op, changes, at)?;

					Ok(r)
				}?;

				self.backend.commit_operation(op).map_err(|e| {
					tracing::error!(
						target = DEFAULT_BUILDER_LOG_TARGET,
						error = ?e,
						"Could not commit backend operation at {}.",
						at,
					);

					Error::BackendOperationCommit(at, e.into())
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

		let (r, changes) = ext.execute_with_mut(exec).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not execute externalities.",
			);

			Error::ExternalitiesExecution(Some(BlockId::Number(Zero::zero())), e.into())
		})?;

		let (_main_sc, _child_sc, _, tx, root, _tx_index) = changes.into_inner();

		op.update_db_storage(tx).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update DB storage."
			);

			Error::DBStorageUpdate(None, e.into())
		})?;

		op.update_storage(main_sc, child_sc).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update storage."
			);

			Error::StorageUpdate(at, e.into())
		})?;

		op.update_transaction_index(tx_index).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update transaction index."
			);

			Error::TransactionIndexUpdate(at, e.into())
		})?;

		let genesis_block = Block::new(
			Block::Header::new(
				Zero::zero(),
				<<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
					Vec::new(),
					self.state_version(),
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

			Error::BlockDataSet(None, e.into())
		})?;

		Ok(r)
	}

	fn mutate_normal<R>(
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

			Error::DBStorageUpdate(Some(at), e.into())
		})?;

		op.update_storage(main_sc, child_sc).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update storage."
			);

			Error::StorageUpdate(at, e.into())
		})?;

		op.update_transaction_index(tx_index).map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not update transaction index."
			);

			Error::TransactionIndexUpdate(at, e.into())
		})?;

		let block = match at {
			BlockId::Hash(req_at) => req_at,
			BlockId::Number(req_at) => self.backend.blockchain().hash(req_at).unwrap().unwrap(),
		};

		let body = chain_backend.body(block).map_err(|e| {
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

		// TODO(cdamian): Carried over from original PR.
		//
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
				"Could not set block data at {}.",
				at
			);

			Error::BlockDataSet(Some(at), e.into())
		})?;

		Ok(())
	}

	/// Append a given set of key-value-pairs into the builder cache
	pub fn append_transition(&mut self, trans: StoragePair) {
		self.cache.auxilliary.push(trans);
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
		params: BlockImportParams<Block, TransactionFor<B, Block>>,
	) -> Result<(), Error<Block>> {
		let prev_hash = self.latest_block();

		let ret = match futures::executor::block_on(
			self.client
				.as_ref()
				.import_block(params, Default::default()),
		)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not import block."
			);

			Error::BlockImporting(e.into())
		})? {
			ImportResult::Imported(_) => Ok(()),
			ImportResult::AlreadyInChain => Err(()),
			ImportResult::KnownBad => Err(()),
			ImportResult::UnknownParent => Err(()),
			ImportResult::MissingState => Err(()),
		};

		// Trigger pool maintenance
		//
		// We do not re-org and we do always finalize directly. So no actual
		// "routes" provided here.
		if ret.is_ok() {
			let best_hash = self.latest_block();
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

				Error::BackendOperationReversal(e.into())
			})?;
		}

		Ok(())
	}
}
