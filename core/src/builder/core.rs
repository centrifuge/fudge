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

use crate::{provider::ExternalitiesProvider, StoragePair};

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash)]
pub enum Operation {
	Commit,
	DryRun,
}

#[derive(Clone)]
pub struct TransitionCache {
	auxilliary: Vec<StoragePair>,
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
			backend: backend,
			client: client,
			pool: pool,
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

	pub fn commit_storage_changes(
		&mut self,
		changes: StorageChanges<B::State, Block>,
	) -> Result<(), ()> {
		let at = BlockId::Hash(self.client.info().best_hash);
		let mut op = self.backend.begin_operation().unwrap();
		self.backend.begin_state_operation(&mut op, at).unwrap();
		let info = self.client.info();
		if info.best_hash == info.finalized_hash {
			self.backend
				.revert(NumberFor::<Block>::one(), true)
				.unwrap();
		}
		self.mutate_normal(&mut op, changes, at).unwrap();
		self.backend.commit_operation(op).unwrap();

		Ok(())
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
					let mut ext = ExternalitiesProvider::<HashFor<Block>, B::State>::new(&state);
					let (r, changes) = ext.execute_with_mut(exec);
					self.mutate_normal(&mut op, changes, at).unwrap();

					Ok(r)
				};

				self.backend
					.commit_operation(op)
					.map_err(|_| "Unable to commit state-operation on backend".to_string())?;

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

	fn mutate_normal(
		&self,
		op: &mut B::BlockImportOperation,
		changes: StorageChanges<B::State, Block>,
		at: BlockId<Block>,
	) -> Result<(), String> {
		let chain_backend = self.backend.blockchain();
		let mut header = chain_backend
			.header(at)
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
			.header(BlockId::Hash(self.latest_block()))
			.ok()
			.flatten()
			.expect("State is available. qed");
		let proposer = futures::executor::block_on(factory.init(&header)).unwrap();
		futures::executor::block_on(proposer.propose(inherents, digest, time, Some(limit))).unwrap()
	}

	/// Import a block, that has been previosuly build
	pub fn import_block(
		&mut self,
		params: BlockImportParams<Block, C::Transaction>,
	) -> Result<(), ()> {
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
		.unwrap()
		{
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

		ret
	}
}
