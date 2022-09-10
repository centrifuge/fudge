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
use sc_client_api::{
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, HeaderBackend, UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TFullClient, TaskManager};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, Proposal};
use sp_core::traits::CodeExecutor;
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, BlockIdTo},
};
use sp_state_machine::StorageProof;
use sp_std::{marker::PhantomData, sync::Arc, time::Duration};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;

use crate::{
	builder::core::{Builder, Operation},
	digest::DigestCreator,
	inherent::ArgsProvider,
	types::StoragePair,
	PoolState,
};

pub struct StandAloneBuilder<
	Block: BlockT,
	RtApi,
	Exec,
	CIDP,
	ExtraArgs,
	DP,
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
	builder: Builder<Block, RtApi, Exec, B, C>,
	cidp: CIDP,
	dp: DP,
	next: Option<(Block, StorageProof)>,
	imports: Vec<(Block, StorageProof)>,
	handle: SpawnTaskHandle,
	_phantom: PhantomData<ExtraArgs>,
}

impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
	StandAloneBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C>
where
	B: BackendT<Block> + 'static,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, C> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
	CIDP: CreateInherentDataProviders<Block, ExtraArgs> + Send + Sync + 'static,
	CIDP::InherentDataProviders: Send,
	DP: DigestCreator<Block>,
	ExtraArgs: ArgsProvider<ExtraArgs>,
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
	pub fn new(manager: &TaskManager, backend: Arc<B>, client: Arc<C>, cidp: CIDP, dp: DP) -> Self {
		Self {
			builder: Builder::new(backend, client, manager),
			cidp,
			dp,
			next: None,
			imports: Vec::new(),
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

	/* TODO: Implement this
	 pub fn append_xcm(&mut self, _xcm: Bytes) -> &mut Self {
		todo!()
	}

	pub fn append_xcms(&mut self, _xcms: Vec<Bytes>) -> &mut Self {
		todo!()
	}
	 */

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
		let inherents = provider.create_inherent_data().unwrap();
		let digest = self
			.with_state(|| {
				futures::executor::block_on(self.dp.create_digest(parent, inherents.clone()))
					.unwrap()
			})
			.unwrap();

		let Proposal { block, proof, .. } = self.builder.build_block(
			self.handle.clone(),
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
		self.imports.push((block, proof));
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
