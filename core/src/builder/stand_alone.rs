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

use crate::digest::DigestCreator;
use crate::inherent::ArgsProvider;
use crate::{
	builder::core::{Builder, Operation},
	types::{Bytes, StoragePair},
};
use sc_client_api::{AuxStore, Backend as BackendT, BlockOf, HeaderBackend, UsageProvider};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TFullClient};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_consensus::{BlockOrigin, Proposal};
use sp_core::traits::CodeExecutor;
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::{marker::PhantomData, sync::Arc, time::Duration};

pub struct StandAloneBuilder<
	Block: BlockT,
	RtApi,
	Exec,
	CIDP,
	ExtraArgs,
	DP,
	B = Backend<Block>,
	C = TFullClient<Block, RtApi, Exec>,
> {
	builder: Builder<Block, RtApi, Exec, B, C>,
	cidp: CIDP,
	dp: DP,
	next: Option<Block>,
	imported_blocks: Vec<Block>,
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
	DP: DigestCreator<Block::Hash>,
	ExtraArgs: ArgsProvider<ExtraArgs>,
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
	pub fn new(
		handle: SpawnTaskHandle,
		backend: Arc<B>,
		client: Arc<C>,
		cidp: CIDP,
		dp: DP,
	) -> Self {
		Self {
			builder: Builder::new(backend, client),
			cidp,
			dp,
			next: None,
			imported_blocks: Vec::new(),
			handle,
			_phantom: Default::default(),
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

	pub fn append_xcm(&mut self, _xcm: Bytes) -> &mut Self {
		todo!()
	}

	pub fn append_xcms(&mut self, _xcms: Vec<Bytes>) -> &mut Self {
		todo!()
	}

	pub fn build_block(&mut self) -> &mut Self {
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

		let digest = self
			.with_state(|| futures::executor::block_on(self.dp.create_digest()).unwrap())
			.unwrap();

		// TODO: Might need proof and storage changes??
		let Proposal {
			block,
			proof: _proof,
			storage_changes: _changes,
		} = self.builder.build_block(
			self.handle.clone(),
			provider.create_inherent_data().unwrap(),
			digest,
			Duration::from_secs(60),
			6_000_000,
		);

		self.next = Some(block.clone());
		self.imported_blocks.push(block);
		self
	}

	pub fn import_block(&mut self) -> &mut Self {
		let block = self.next.take().unwrap();
		let mut params = BlockImportParams::new(
			BlockOrigin::ConsensusBroadcast,
			block.clone().deconstruct().0,
		);
		params.body = Some(block.clone().deconstruct().1);
		params.finalized = true;
		params.fork_choice = Some(ForkChoiceStrategy::Custom(true));

		self.builder.import_block(params).unwrap();
		self.imported_blocks.push(block);
		self
	}

	pub fn blocks(&self) -> Vec<Block> {
		self.imported_blocks.clone()
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
}
