#![feature(associated_type_bounds)]
extern crate sc_client_api;
extern crate sc_client_db;
extern crate sc_consensus;
extern crate sc_service;
extern crate sp_api;
extern crate sp_consensus;
extern crate sp_runtime;

// Copyright 2021 Centrifuge Foundation (centrifuge.io).
//
// This file is part of the Centrifuge chain project.
// Centrifuge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).
// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
use crate::provider::ExternalitiesProvider;
use sc_client_api::{
	blockchain::ProvideCache, AuxStore, Backend as BackendT, BlockOf, CallExecutor, HeaderBackend,
	UsageProvider,
};
use sc_client_db::{Backend, DatabaseSettings, DatabaseSource, RefTrackingState};
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{LocalCallExecutor, TFullClient};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, Core as CoreApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, CanAuthorWith, Error as ConsensusError};
use sp_core::{
	traits::{CodeExecutor, ReadRuntimeVersion},
	Pair,
};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::{collections::HashMap, marker::PhantomData, path::PathBuf, sync::Arc};
use sp_std::time::Duration;

use self::{sc_client_api::ClientImportOperation, sp_api::HashFor};
use super::{traits::AuthorityProvider, Bytes, StoragePair};
use self::sp_consensus::InherentData;
use self::sp_runtime::{Digest, DigestItem};
use self::sp_runtime::traits::DigestFor;

pub enum Operation {
	Commit,
	DryRun,
}

pub struct TransitionCache<Block: BlockT> {
	extrinsics: Vec<Block::Extrinsic>,
	auxilliary: Vec<StoragePair>,
}

pub struct Builder<Block, RtApi, Exec, B = Backend<Block>, C = TFullClient<Block, RtApi, Exec>>
where
	B: BackendT<Block>,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	backend: Arc<B>,
	client: Arc<C>,
	cache: TransitionCache<Block>,
	blocks: Vec<BlockImportParams<Block, <C as BlockImport<Block>>::Transaction>>,
	_phantom: PhantomData<(Block, RtApi, Exec)>,
}

impl<Block, RtApi, Exec, B, C> Builder<Block, RtApi, Exec, B, C>
where
	B: BackendT<Block>,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	/// Create a new Builder with provided backend and client.
	pub fn new(backend: Arc<B>, client: C) -> Self {
		Builder {
			backend: backend,
			client: Arc::new(client),
			cache: TransitionCache { extrinsics: Vec::new(), auxilliary: Vec::new() },
			blocks: Vec::new(),
			_phantom: PhantomData::default(),
		}
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
		let mut ext = ExternalitiesProvider::<HashFor<Block>, Block, B::State>::new(&state);

		match op {
			// TODO: Does this actually commit changes to the underlying DB?
			Operation::Commit => {
				let inner = || {
					let _import_lock = self.backend.get_import_lock().write();

					let mut op = self
						.backend
						.begin_operation()
						.map_err(|_| "Unable to start state-operation on backend".to_string())?;

					self.backend.begin_state_operation(&mut op, at);
					let r = ext.execute_with(exec);

					self.backend
						.commit_operation(op)
						.map_err(|_| "Unable to commit state-operation on backend".to_string())?;

					Ok(r)
				};
				let result = inner();
				// TODO: The parity client does this. I am not sure, why. At least the guard is
				// dropped here
				//*self.backend.importing_block.write() = None;

				result
			},
			// TODO: Does this actually NOT change the state?
			Operation::DryRun => Ok(ext.execute_with(exec)),
		}
	}

	/// Append a given set of key-value-pairs into the builder cache
	pub fn append_transition(&mut self, trans: StoragePair) -> &mut Self {
		self.cache.auxilliary.push(trans);
		self
	}

	/// Caches a given extrinsic in the builder. The extrinsic will be
	pub fn append_extrinsic(&mut self, ext: Block::Extrinsic) -> &mut Self {
		self.cache.extrinsics.push(ext);
		self
	}

	/// Create a block from a given state of the Builder.
	///
	/// Multiple calls of this will result in building multiple blocks, where each block
	/// is the child of the previously build block (or the last block of the fetched state).
	pub fn build_block(&mut self, inherents: InherentData, digest: DigestFor<Block>, time: Duration, limit: usize) -> &mut Self {
		todo!()

		// TODO: Uses the cached transitions and extrinsics and creates a BlockImportParams struct

		// TODO: Rough overview
		//   - Create the `BlockImportParams` struct E.g
		//		// NOTE: Is this the correct BlockOrigin? We want it to be stored as "checked"
		//     let mut import = BlockImportParams::new(BlockOrigin::ConsensusBroadcast, header);
		//     import.body = Some(self.cache.extrinsics);
		//     import.aux = self.cache.auxilliary;
		//     import.finalized = true;
		//     import.fork_choice = Some(ForkChoiceStrategy::Custom(true));
		//
		//      - We need to pass the new changes via `Aux`-field
		// 	 - Stores struct in builder
		//   - Cleans up TransitionCache
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
