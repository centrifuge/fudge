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

use std::{error::Error as StdError, marker::PhantomData, sync::Arc};

use sc_block_builder::BlockBuilderApi;
use sc_client_api::{
	AuxStore, Backend as BackendT, BlockBackend, BlockOf, HeaderBackend, UsageProvider,
};
use sc_consensus::BlockImport;
use sc_executor::{RuntimeVersionOf, WasmExecutor};
use sc_service::{
	ClientConfig, GenesisBlockBuilder, LocalCallExecutor, TFullBackend, TFullClient, TaskManager,
};
use sc_transaction_pool_api::{MaintainedTransactionPool, TransactionPool};
use sp_api::{CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_core::traits::{CodeExecutor, SpawnNamed};
use sp_runtime::traits::{Block as BlockT, BlockIdTo};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use thiserror::Error;

pub mod backend;
pub mod externalities;
pub mod initiator;
pub mod state;

pub type InnerError = Box<dyn StdError>;

pub trait Initiator<Block: BlockT>
where
	for<'r> &'r Self::Client: BlockImport<Block>,
{
	type Api: BlockBuilder<Block> + BlockBuilderApi<Block> + TaggedTransactionQueue<Block>;
	type Client: 'static
		+ ProvideRuntimeApi<Block, Api = Self::Api>
		+ BlockOf
		+ BlockBackend<Block>
		+ BlockIdTo<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>;
	type Backend: 'static + BackendT<Block>;
	type Pool: 'static
		+ TransactionPool<Block = Block, Hash = Block::Hash>
		+ MaintainedTransactionPool;
	type Executor: 'static + CodeExecutor + RuntimeVersionOf + Clone;
	type Error: 'static + StdError;

	fn init(
		self,
	) -> Result<
		(
			Arc<Self::Client>,
			Arc<Self::Backend>,
			Arc<Self::Pool>,
			Self::Executor,
			TaskManager,
		),
		Self::Error,
	>;
}

pub trait BackendProvider<Block: BlockT> {
	type Backend: 'static + BackendT<Block>;

	fn provide(&self) -> Result<Arc<Self::Backend>, sp_blockchain::Error>;
}

pub trait ClientProvider<Block: BlockT> {
	type Api: BlockBuilder<Block> + BlockBuilderApi<Block> + TaggedTransactionQueue<Block>;
	type Backend: 'static + BackendT<Block>;
	type Client: 'static
		+ ProvideRuntimeApi<Block, Api = Self::Api>
		+ BlockOf
		+ BlockBackend<Block>
		+ BlockIdTo<Block>
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ BlockImport<Block>
		+ CallApiAt<Block>;
	type Exec: CodeExecutor + RuntimeVersionOf + 'static;
	type Error: 'static + StdError;

	fn provide(
		&self,
		config: ClientConfig<Block>,
		genesis_block_builder: GenesisBlockBuilder<Block, Self::Backend, Self::Exec>,
		backend: Arc<Self::Backend>,
		exec: LocalCallExecutor<Block, Self::Backend, Self::Exec>,
		spawn_handle: Box<dyn SpawnNamed>,
	) -> Result<Arc<Self::Client>, Self::Error>;
}

pub struct DefaultClient<Block, RtApi, Exec>(PhantomData<(Block, RtApi, Exec)>);

impl<Block, RtApi, Exec> DefaultClient<Block, RtApi, Exec> {
	pub fn new() -> Self {
		Self(Default::default())
	}
}

/// HostFunctions that do not include benchmarking specific host functions
pub type TWasmExecutor = WasmExecutor<sp_io::SubstrateHostFunctions>;

const DEFAULT_CLIENT_PROVIDER_LOG_TARGET: &str = "fudge-client-provider";

#[derive(Error, Debug)]
pub enum Error {
	#[error("full client creation: {0}")]
	FullClientCreation(InnerError),
}

impl<Block, RtApi, Exec> ClientProvider<Block> for DefaultClient<Block, RtApi, Exec>
where
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send + Sync + 'static,
	<RtApi as ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>>>::RuntimeApi:
		TaggedTransactionQueue<Block> + BlockBuilderApi<Block>,
	Exec: CodeExecutor + RuntimeVersionOf,
{
	type Api = <TFullClient<Block, RtApi, Exec> as ProvideRuntimeApi<Block>>::Api;
	type Backend = TFullBackend<Block>;
	type Client = TFullClient<Block, RtApi, Exec>;
	type Error = Error;
	type Exec = Exec;

	fn provide(
		&self,
		config: ClientConfig<Block>,
		genesis_block_builder: GenesisBlockBuilder<Block, Self::Backend, Self::Exec>,
		backend: Arc<Self::Backend>,
		exec: LocalCallExecutor<Block, Self::Backend, Self::Exec>,
		spawn_handle: Box<dyn SpawnNamed>,
	) -> Result<Arc<Self::Client>, Self::Error> {
		TFullClient::new(
			backend.clone(),
			exec,
			spawn_handle,
			genesis_block_builder,
			None,
			None,
			None,
			None,
			config,
		)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_CLIENT_PROVIDER_LOG_TARGET,
				error = ?e,
				"Could not create full client."
			);

			Error::FullClientCreation(e.into())
		})
		.map(|client| Arc::new(client))
	}
}
