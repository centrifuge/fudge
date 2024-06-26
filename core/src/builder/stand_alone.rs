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
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, HeaderBackend,
	UsageProvider,
};
use sc_client_db::Backend;
use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
use sc_executor::RuntimeVersionOf;
use sc_service::{SpawnTaskHandle, TFullClient};
use sc_transaction_pool::FullPool;
use sc_transaction_pool_api::{MaintainedTransactionPool, TransactionPool};
use sp_api::{CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
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
	builder::core::{Builder, InnerError, Operation, PoolState},
	digest::DigestCreator,
	inherent::ArgsProvider,
	provider::Initiator,
	types::StoragePair,
};

const DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET: &str = "fudge-standalone-chain";

#[derive(Error, Debug)]
pub enum Error {
	#[error("core builder: {0}")]
	CoreBuilder(InnerError),

	#[error("initiator: {0}")]
	Initiator(InnerError),

	#[error("inherent data providers creation: {0}")]
	InherentDataProvidersCreation(Box<dyn std::error::Error + Send + Sync>),

	#[error("inherent data creation: {0}")]
	InherentDataCreation(InnerError),

	#[error("digest creation: {0}")]
	DigestCreation(InnerError),

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
	A = FullPool<Block, C>,
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
	A: TransactionPool<Block = Block, Hash = Block::Hash> + MaintainedTransactionPool + 'static,
{
	builder: Builder<Block, RtApi, Exec, B, C, A>,
	cidp: CIDP,
	dp: DP,
	next: Option<(Block, StorageProof)>,
	imports: Vec<(Block, StorageProof)>,
	handle: SpawnTaskHandle,
	_phantom: PhantomData<ExtraArgs>,
}

impl<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C, A>
	StandAloneBuilder<Block, RtApi, Exec, CIDP, ExtraArgs, DP, B, C, A>
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
	for<'r> &'r C: BlockImport<Block>,
	A: TransactionPool<Block = Block, Hash = Block::Hash> + MaintainedTransactionPool + 'static,
{
	pub fn new<I, F>(initiator: I, setup: F) -> Result<Self, Error>
	where
		I: Initiator<Block, Api = C::Api, Client = C, Backend = B, Pool = A, Executor = Exec>,
		F: FnOnce(Arc<C>) -> (CIDP, DP),
	{
		let (client, backend, pool, executor, task_manager) = initiator.init().map_err(|e| {
			tracing::error!(
				target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not initialize."
			);

			Error::Initiator(e.into())
		})?;

		let (cidp, dp) = setup(client.clone());

		let handle = task_manager.spawn_handle();

		Ok(Self {
			builder: Builder::new(client, backend, pool, executor, task_manager),
			cidp,
			dp,
			next: None,
			imports: Vec::new(),
			handle,
			_phantom: Default::default(),
		})
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

		let inherents =
			futures::executor::block_on(provider.create_inherent_data()).map_err(|e| {
				tracing::error!(
					target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
					error = ?e,
					"Could not create inherent data."
				);

				Error::InherentDataCreation(e.into())
			})?;

		let parent = self.builder.latest_header().map_err(|e| {
			tracing::error!(
				target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
				error = ?e,
				"Could not retrieve latest header."
			);

			Error::CoreBuilder(e.into())
		})?;

		let digest = self.with_state(|| {
			futures::executor::block_on(self.dp.create_digest(parent, inherents.clone())).map_err(
				|e| {
					tracing::error!(
						target = DEFAULT_STANDALONE_CHAIN_BUILDER_LOG_TARGET,
						error = ?e,
						"Could not create digest."
					);

					Error::DigestCreation(e.into())
				},
			)
		})??;

		let Proposal { block, proof, .. } = self
			.builder
			.build_block(
				self.builder.handle(),
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
		let (block, proof) = self.next.take().ok_or_else(|| {
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
