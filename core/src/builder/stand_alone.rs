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
const DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET: &str = "fudge-standalone-chain";

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
use thiserror::Error;

use crate::{
	builder::core::{Builder, InnerError, Operation},
	digest::DigestCreator,
	inherent::ArgsProvider,
	types::StoragePair,
	PoolState,
};

#[derive(Error, Debug)]
pub enum Error {
	#[error("core builder error: {0}")]
	CoreBuilder(InnerError),

	#[error("couldn't create inherent data providers: {0}")]
	InherentDataProvidersCreation(Box<dyn std::error::Error + Send + Sync>),

	#[error("couldn't create inherent data: {0}")]
	InherentDataCreation(InnerError),

	#[error("couldn't create digest")]
	DigestCreation,

	#[error("next block not found")]
	NextBlockNotFound,
}

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

	pub fn append_extrinsic(&mut self, xt: Block::Extrinsic) -> Result<Block::Hash, Error> {
		self.builder
			.append_extrinsic(xt)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	pub fn append_extrinsics(
		&mut self,
		xts: Vec<Block::Extrinsic>,
	) -> Result<Vec<Block::Hash>, Error> {
		xts.into_iter().fold(Ok(Vec::new()), |hashes, xt| {
			let mut hashes = hashes?;

			let block_hash = self
				.builder
				.append_extrinsic(xt)
				.map_err(|e| Error::CoreBuilder(e.into()))?;

			hashes.push(block_hash);

			Ok(hashes)
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

	pub fn build_block(&mut self) -> Result<Block, Error> {
		assert!(self.next.is_none());

		let provider =
			self.with_state(|| {
				futures::executor::block_on(self.cidp.create_inherent_data_providers(
					self.builder.latest_block(),
					ExtraArgs::extra(),
				))
				.map_err(|e| {
					tracing::error!(
						target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
						error = ?e,
						"Could not create inherent data providers."
					);

					Error::InherentDataProvidersCreation(e)
				})
			})??;

		let inherents = provider.create_inherent_data().map_err(|e| {
			tracing::error!(
				target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not create inherent data providers."
			);

			Error::InherentDataCreation(e.into())
		})?;

		let digest = self.with_state(|| {
			futures::executor::block_on(self.dp.create_digest(inherents.clone())).map_err(|_| {
				tracing::error!(
					target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
					"Could not create inherent data providers."
				);

				Error::DigestCreation
			})
		})??;

		let Proposal { block, proof, .. } = self
			.builder
			.build_block(
				self.handle.clone(),
				inherents,
				digest,
				Duration::from_secs(60),
				6_000_000,
			)
			.map_err(|e| Error::CoreBuilder(e.into()))?;

		self.next = Some((block.clone(), proof));

		Ok(block)
	}

	pub fn import_block(&mut self) -> Result<(), Error> {
		let (block, proof) = self.next.take().ok_or({
			tracing::error!(
				target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
				"Next block not found."
			);

			Error::NextBlockNotFound
		})?;
		let (header, body) = block.clone().deconstruct();
		let mut params = BlockImportParams::new(BlockOrigin::NetworkInitialSync, header);
		params.body = Some(body);
		params.finalized = true;
		params.fork_choice = Some(ForkChoiceStrategy::Custom(true));

		self.builder
			.import_block(params)
			.map_err(|e| Error::CoreBuilder(e.into()))?;

		self.imports.push((block, proof));
		Ok(())
	}

	pub fn imports(&self) -> Vec<(Block, StorageProof)> {
		self.imports.clone()
	}

	pub fn with_state<R>(&self, exec: impl FnOnce() -> R) -> Result<R, Error> {
		self.builder
			.with_state(Operation::DryRun, None, exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	pub fn with_state_at<R>(
		&self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, Error> {
		self.builder
			.with_state(Operation::DryRun, Some(at), exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	pub fn with_mut_state<R>(&mut self, exec: impl FnOnce() -> R) -> Result<R, Error> {
		assert!(self.next.is_none());

		self.builder
			.with_state(Operation::Commit, None, exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}

	/// Mutating past states not supported yet...
	fn with_mut_state_at<R>(
		&mut self,
		at: BlockId<Block>,
		exec: impl FnOnce() -> R,
	) -> Result<R, Error> {
		assert!(self.next.is_none());

		self.builder
			.with_state(Operation::Commit, Some(at), exec)
			.map_err(|e| Error::CoreBuilder(e.into()))
	}
}
