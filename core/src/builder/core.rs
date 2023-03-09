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

use crate::{provider::externalities::ExternalitiesProvider, types::StoragePair};

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

	pub fn latest_header(&self) -> Block::Header {
		self.backend
			.blockchain()
			.header(self.latest_block())
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
	) -> Result<R, String> {
		let block = match at {
			Some(BlockId::Hash(req_at)) => req_at,
			Some(BlockId::Number(req_at)) => {
				self.backend.blockchain().hash(req_at).unwrap().unwrap()
			}
			_ => self.client.info().best_hash,
		};
		let state = self.backend.state_at(block);
		let at = sp_api::BlockId::Hash(block);

		let state = state.map_err(|_| "State at INSERT_AT_HERE not available".to_string())?;

		match op {
			Operation::Commit => {
				let mut op = self
					.backend
					.begin_operation()
					.map_err(|_| "Unable to start state-operation on backend".to_string())?;
				self.backend.begin_state_operation(&mut op, block).unwrap();

				let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);
				let r = ext.execute_with(exec);

				if self
					.backend
					.blockchain()
					.block_number_from_id(&at)
					.unwrap()
					.unwrap() == Zero::zero()
				{
					self.mutate_genesis::<R>(&mut op, ext.drain(self.state_version()))
				} else {
					// We need to revert the latest block and re-import it again in order to
					// mutate it if it was already finalized
					let info = self.client.info();
					if info.best_hash == info.finalized_hash {
						self.backend
							.revert(NumberFor::<Block>::one(), true)
							.unwrap();
					}
					self.mutate_normal::<R>(&mut op, ext.drain(self.state_version()), at)
				}?;

				self.backend
					.commit_operation(op)
					.map_err(|_| "Unable to commit state-operation on backend".to_string())?;

				Ok(r)
			}
			Operation::DryRun => Ok(
				ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state).execute_with(exec),
			),
		}
	}

	fn mutate_genesis<R>(
		&self,
		op: &mut B::BlockImportOperation,
		changes: StorageChanges<<<B as sc_client_api::Backend<Block>>::State as StateBackend<HashFor<Block>>>::Transaction, HashFor<Block>>,
	) -> Result<(), String> {
		let (main_sc, child_sc, _, tx, root, tx_index) = changes.into_inner();

		op.update_db_storage(tx).unwrap();
		op.update_storage(main_sc, child_sc)
			.map_err(|_| "Updating storage not possible.")
			.unwrap();
		op.update_transaction_index(tx_index)
			.map_err(|_| "Updating transaction index not possible.")
			.unwrap();

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
		.map_err(|_| "Could not set block data".to_string())?;

		Ok(())
	}

	fn mutate_normal<R>(
		&self,
		op: &mut B::BlockImportOperation,
		changes: StorageChanges<<<B as sc_client_api::Backend<Block>>::State as StateBackend<HashFor<Block>>>::Transaction, HashFor<Block>>,
		at: BlockId<Block>,
	) -> Result<(), String> {
		let chain_backend = self.backend.blockchain();
		let block = match at {
			BlockId::Hash(req_at) => req_at,
			BlockId::Number(req_at) => self.backend.blockchain().hash(req_at).unwrap().unwrap(),
		};
		let mut header = chain_backend
			.header(block)
			.ok()
			.flatten()
			.expect("State is available. qed");

		let (main_sc, child_sc, _, tx, root, tx_index) = changes.into_inner();
		header.set_state_root(root);

		op.update_db_storage(tx).unwrap();
		op.update_storage(main_sc, child_sc)
			.map_err(|_| "Updating storage not possible.")
			.unwrap();
		op.update_transaction_index(tx_index)
			.map_err(|_| "Updating transaction index not possible.")
			.unwrap();

		let body = chain_backend.body(block).expect("State is available. qed.");
		let indexed_body = chain_backend
			.block_indexed_body(block)
			.expect("State is available. qed.");
		let justifications = chain_backend
			.justifications(block)
			.expect("State is available. qed.");

		op.set_block_data(
			header,
			body,
			indexed_body,
			justifications,
			NewBlockState::Final,
		)
		.unwrap();
		Ok(())
	}

	/// Append a given set of key-value-pairs into the builder cache
	pub fn append_transition(&mut self, trans: StoragePair) {
		self.cache.auxilliary.push(trans);
	}

	/// Caches a given extrinsic in the builder. The extrinsic will be
	pub fn append_extrinsic(&mut self, ext: Block::Extrinsic) -> Result<Block::Hash, ()> {
		let fut = self.pool.submit_one(
			&BlockId::Hash(self.client.info().best_hash),
			TransactionSource::External,
			ext,
		);
		futures::executor::block_on(fut).map_err(|_| ())
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
	) -> Proposal<Block, TransactionFor<B, Block>, StorageProof> {
		let mut factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
			handle,
			self.client.clone(),
			self.pool.clone(),
			None,
			None,
		);
		let header = self
			.backend
			.blockchain()
			.header(self.latest_block())
			.ok()
			.flatten()
			.expect("State is available. qed");
		let proposer = futures::executor::block_on(factory.init(&header)).unwrap();
		futures::executor::block_on(proposer.propose(inherents, digest, time, Some(limit)))
			.expect("Proposal failure")
	}

	/// Import a block, that has been previosuly build
	pub fn import_block(
		&mut self,
		params: BlockImportParams<Block, TransactionFor<B, Block>>,
	) -> Result<(), ()> {
		let prev_hash = self.latest_block();
		let ret = match futures::executor::block_on(
			self.client
				.as_ref()
				.import_block(params, Default::default()),
		)
		.unwrap()
		{
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

		ret
	}
}
