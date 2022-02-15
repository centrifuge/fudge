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
	blockchain::ProvideCache, AuxStore, Backend as BackendT, BlockOf, HeaderBackend,
	UsageProvider,
};
use sc_client_db::Backend;
use sc_executor::RuntimeVersionOf;
use sc_service::{Configuration, TFullClient};
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, Core as CoreApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, CanAuthorWith, Error as ConsensusError};
use sp_core::{
	traits::{CodeExecutor, ReadRuntimeVersion},
	Pair,
};
use sp_inherents::InherentDataProvider;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::{marker::PhantomData, sync::Arc};
use builder::{Builder,Operation};

mod builder;
mod provider;
#[cfg(test)]
mod tests;
mod traits;

pub use provider::EnvProvider;
pub use traits::AuthorityProvider;

pub type Bytes = Vec<u8>;

#[derive(Clone, Debug)]
pub struct StoragePair {
	key: Bytes,
	value: Option<Bytes>,
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
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	builder: Builder<Block, RtApi, Exec, B, C>
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
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
{
	pub fn new(backend: Arc<B>, client: C) -> Self {
		Self {
			builder: Builder::new(backend, client)
		}
	}

	pub fn from_config(config: &Configuration, exec: Exec) -> Self {
		todo!()
	}

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
		self.builder.append_extrinsic(xt);
		self
	}

	pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
		xts.into_iter().for_each(|xt| {
			self.builder.append_extrinsic(xt);
		});
		self
	}

	pub fn append_transition(&mut self, aux: StoragePair) -> &mut Self {
		self.builder.append_transition(aux);
		self
	}

	pub fn append_transitions(&mut self, auxs: Vec<StoragePair>) -> &mut Self {
		auxs.into_iter().for_each(|aux| {
			self.builder.append_transition(aux);
		});
		self
	}

	pub fn append_xcm(&mut self, xcm: Bytes) -> &mut Self {
		todo!()
	}

	pub fn append_xcms(&mut self, xcms: Vec<Bytes>) -> &mut Self {
		todo!()
	}

	pub fn build_block<RelayBlock, RelayRtApi, RelayExec, RelayI, RelayB, RelayC>(
		&mut self,
		relay: &mut RelaychainBuilder<RelayBlock, RelayRtApi, RelayExec, RelayI, RelayB, RelayC>,
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
		RelayI: InherentDataProvider
	{
		// TODO: Builds a block in companion with the relay-chain. This should be used
		//       in order to actually test xcms, block production, etc.
		todo!()
	}

	pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, None, exec)
	}

	pub fn with_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, Some(at), exec)
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder.with_state(Operation::Commit, None, exec)
	}

	pub fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder.with_state(Operation::Commit, Some(at), exec)
	}

	/// Store the provided authorites in the Builder cache.
	pub fn swap_authorities<P: AuthorityProvider>(&mut self) -> &mut Self {
		<P as AuthorityProvider>::block_production().into_iter().for_each(|pair| {
			self.builder.append_transition(pair);
		});

		<P as AuthorityProvider>::misc().into_iter().for_each(|pair| {
			self.builder.append_transition(pair);
		});

		self
	}
}

pub struct RelaychainBuilder<
	Block,
	RtApi,
	Exec,
	I,
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
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>,
	I: InherentDataProvider
{
	builder: Builder<Block, RtApi, Exec, B, C>,
	_phantom: PhantomData<I>
}

impl<Block, RtApi, Exec, I, B, C> RelaychainBuilder<Block, RtApi, Exec, I, B, C>
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
	I: InherentDataProvider
{
	pub fn new(backend: Arc<B>, client: C) -> Self {
		Self {
			builder: Builder::new(backend, client),
			_phantom: Default::default()
		}
	}

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> &mut Self {
		self.builder.append_extrinsic(xt);
		self
	}

	pub fn append_extrinsics(&mut self, xts: Vec<Block::Extrinsic>) -> &mut Self {
		xts.into_iter().for_each(|xt| {
			self.builder.append_extrinsic(xt);
		});
		self
	}

	pub fn append_transition(&mut self, aux: StoragePair) -> &mut Self {
		self.builder.append_transition(aux);
		self
	}

	pub fn append_transitions(&mut self, auxs: Vec<StoragePair>) -> &mut Self {
		auxs.into_iter().for_each(|aux| {
			self.builder.append_transition(aux);
		});
		self
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

	/*
	pub fn build_block_with_limits(&mut self, weight: , time: ) -> &mut Self {
		todo!()
	}
	*/

	pub fn with_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, None, exec)
	}

	pub fn with_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder.with_state(Operation::DryRun, Some(at), exec)
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, String> {
		self.builder.with_state(Operation::Commit, None, exec)
	}

	pub fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, String> {
		self.builder.with_state(Operation::Commit, Some(at), exec)
	}

	/// Store the provided authorites in the Builder cache.
	pub fn swap_authorities<P: AuthorityProvider>(&mut self) -> &mut Self {
		<P as AuthorityProvider>::block_production().into_iter().for_each(|pair| {
			self.builder.append_transition(pair);
		});

		<P as AuthorityProvider>::misc().into_iter().for_each(|pair| {
			self.builder.append_transition(pair);
		});

		self
	}
}
