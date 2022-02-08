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
	sp_std::{
		borrow::Borrow,
		ops::Deref,
		sync::{Mutex, MutexGuard},
	},
	traits::{CodeExecutor, ReadRuntimeVersion},
	Pair,
};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::{collections::HashMap, marker::PhantomData, path::PathBuf, sync::Arc};

mod builder;
mod externalities_provider;
mod state_provider;
#[cfg(test)]
mod tests;
mod traits;

use crate::builder::Operation;
use builder::Builder;
use externalities_provider::ExternalitiesProvider;
pub use state_provider::StateProvider;
use traits::AuthorityProvider;

pub type Bytes = Vec<u8>;

#[derive(Clone, Debug)]
pub struct StoragePair {
	key: Bytes,
	value: Bytes,
}

pub struct ParachainBuilder<
	Block,
	RtApi,
	Exec,
	B = Backend<Block>,
	C = TFullClient<Block, RtApi, Exec>,
> where
	B: BackendT<Block>,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ ProvideCache<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	builder: Arc<Mutex<Builder<Block, RtApi, Exec, B, C>>>,
}

impl<Block, RtApi, Exec, B, C> ParachainBuilder<Block, RtApi, Exec, B, C>
where
	B: BackendT<Block>,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ ProvideCache<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	pub fn new(state: StateProvider<B, Block>) -> Self {
		// TODO: State does provide backend
		todo!()
	}

	fn builder(&self) -> MutexGuard<Builder<Block, RtApi, Exec, B, C>> {
		self.builder.lock().unwrap()
	}

	fn mut_builder(&mut self) -> MutexGuard<Builder<Block, RtApi, Exec, B, C>> {
		self.builder.lock().unwrap()
	}

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
		todo!()
	}

	pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
		todo!()
	}

	pub fn append_transition(&mut self, aux: (Bytes, Option<Bytes>)) -> &mut Self {
		todo!()
	}

	pub fn append_transitions(&mut self, auxs: Vec<(Bytes, Option<Bytes>)>) -> &mut Self {
		todo!()
	}

	pub fn append_xcm(&mut self, xcm: Bytes) -> &mut Self {
		todo!()
	}

	pub fn append_xcms(&mut self, xcms: Vec<Bytes>) -> &mut Self {
		todo!()
	}

	pub fn build_block<RelayBlock, RelayRtApi, RelayExec, RelayB, RelayC>(
		&mut self,
		relay: &mut RelaychainBuilder<RelayBlock, RelayRtApi, RelayExec, RelayB, RelayC>,
	) -> &mut Self
	where
		RelayB: BackendT<RelayBlock>,
		RelayBlock: BlockT,
		RelayRtApi: ConstructRuntimeApi<RelayBlock, RelayC> + Send,
		RelayExec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
		RelayC::Api: BlockBuilder<RelayBlock> + ApiExt<RelayBlock>,
		RelayC: 'static
			+ ProvideRuntimeApi<RelayBlock>
			+ BlockOf
			+ ProvideCache<RelayBlock>
			+ Send
			+ Sync
			+ AuxStore
			+ UsageProvider<RelayBlock>
			+ HeaderBackend<RelayBlock>
			+ BlockImport<RelayBlock>
			+ CallApiAt<RelayBlock>,
	{
		// TODO: Builds a block in companion with the relay-chain. This should be used
		//       in order to actually test xcms, block production, etc.
		todo!()
	}

	pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder().with_state(Operation::DryRun, None, exec)
	}

	pub fn with_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder().with_state(Operation::DryRun, Some(at), exec)
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder().with_state(Operation::Commit, None, exec)
	}

	pub fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder().with_state(Operation::Commit, Some(at), exec)
	}

	/// Store the provided authorites in the Builder cache.
	pub fn swap_authorities<P: AuthorityProvider>(&mut self) -> &mut Self {
		<P as AuthorityProvider>::block_production().into_iter().for_each(|pair| {
			self.builder().append_transition(pair);
		});

		<P as AuthorityProvider>::misc().into_iter().for_each(|pair| {
			self.builder().append_transition(pair);
		});

		self
	}
}

pub struct RelaychainBuilder<
	Block,
	RtApi,
	Exec,
	B = Backend<Block>,
	C = TFullClient<Block, RtApi, Exec>,
> where
	B: BackendT<Block>,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ ProvideCache<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	builder: Arc<Mutex<Builder<Block, RtApi, Exec, B, C>>>,
}

impl<Block, RtApi, Exec, B, C> RelaychainBuilder<Block, RtApi, Exec, B, C>
where
	B: BackendT<Block>,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	C::Api: BlockBuilder<Block> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ ProvideCache<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	pub fn new(state: StateProvider<B, Block>) -> Self {
		// TODO: State does provide backend
		todo!()
	}

	fn builder(&self) -> MutexGuard<Builder<Block, RtApi, Exec, B, C>> {
		self.builder.lock().unwrap()
	}

	fn mut_builder(&mut self) -> MutexGuard<Builder<Block, RtApi, Exec, B, C>> {
		self.builder.lock().unwrap()
	}

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
		todo!()
	}

	pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
		todo!()
	}

	pub fn append_transition(&mut self, aux: (Bytes, Option<Bytes>)) -> &mut Self {
		todo!()
	}

	pub fn append_transitions(&mut self, auxs: Vec<(Bytes, Option<Bytes>)>) -> &mut Self {
		todo!()
	}

	pub fn append_xcm(&mut self, xcm: Bytes) -> &mut Self {
		todo!()
	}

	pub fn append_xcms(&mut self, xcms: Vec<Bytes>) -> &mut Self {
		todo!()
	}

	pub fn build_block(&mut self) -> &mut Self {
		// TODO: build a block with the given caches without the relay-chain.
		todo!()
	}

	pub fn with_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder().with_state(Operation::DryRun, None, exec)
	}

	pub fn with_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder().with_state(Operation::DryRun, Some(at), exec)
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder().with_state(Operation::Commit, None, exec)
	}

	pub fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder().with_state(Operation::Commit, Some(at), exec)
	}

	/// Store the provided authorites in the Builder cache.
	pub fn swap_authorities<P: AuthorityProvider>(&mut self) -> &mut Self {
		<P as AuthorityProvider>::block_production().into_iter().for_each(|pair| {
			self.mut_builder().append_transition(pair);
		});

		<P as AuthorityProvider>::misc().into_iter().for_each(|pair| {
			self.mut_builder().append_transition(pair);
		});

		self
	}
}
