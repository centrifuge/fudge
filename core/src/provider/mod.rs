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

use std::{error::Error, marker::PhantomData, sync::Arc};

use sc_block_builder::{BlockBuilderApi, BlockBuilderProvider};
use sc_client_api::{
	execution_extensions::ExecutionStrategies, AuxStore, Backend as BackendT, Backend,
	BlockBackend, BlockOf, HeaderBackend, TransactionFor, UsageProvider,
};
use sc_consensus::BlockImport;
use sc_executor::{RuntimeVersionOf, WasmExecutor};
use sc_service::{ClientConfig, LocalCallExecutor, TFullBackend, TFullClient, TaskManager};
use sc_transaction_pool_api::{MaintainedTransactionPool, TransactionPool};
use sp_api::{ApiExt, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_core::traits::CodeExecutor;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::{
	traits::{Block as BlockT, BlockIdTo},
	BuildStorage,
};
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;

pub mod backend;
pub mod externalities;
pub mod initiator;
pub mod state;

pub trait Initiator<Block: BlockT>
where
	for<'r> &'r Self::Client:
		BlockImport<Block, Transaction = TransactionFor<Self::Backend, Block>>,
{
	type Api: BlockBuilder<Block>
		+ ApiExt<Block, StateBackend = <Self::Backend as BackendT<Block>>::State>
		+ BlockBuilderApi<Block>
		+ TaggedTransactionQueue<Block>;
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
		+ CallApiAt<Block>
		+ BlockBuilderProvider<Self::Backend, Block, Self::Client>;
	type Backend: 'static + BackendT<Block>;
	type Pool: 'static
		+ TransactionPool<Block = Block, Hash = Block::Hash>
		+ MaintainedTransactionPool;
	type Executor: 'static + CodeExecutor + RuntimeVersionOf + Clone;
	type Error: 'static + Error;

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
	type Api: BlockBuilder<Block>
		+ ApiExt<Block, StateBackend = <Self::Backend as BackendT<Block>>::State>
		+ BlockBuilderApi<Block>
		+ TaggedTransactionQueue<Block>;
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
		+ CallApiAt<Block>
		+ BlockBuilderProvider<Self::Backend, Block, Self::Client>;
	type Exec: CodeExecutor + RuntimeVersionOf + 'static;

	fn provide(
		&self,
		config: ClientConfig<Block>,
		genesis: Box<dyn BuildStorage>,
		execution_strategies: ExecutionStrategies,
		keystore: Option<SyncCryptoStorePtr>,
		backend: Arc<Self::Backend>,
		exec: LocalCallExecutor<Block, Self::Backend, Self::Exec>,
	) -> Result<Arc<Self::Client>, ()>;
}

pub struct DefaultClient<Block, RtApi, Exec>(PhantomData<(Block, RtApi, Exec)>);

impl<Block, RtApi, Exec> DefaultClient<Block, RtApi, Exec> {
	pub fn new() -> Self {
		Self(Default::default())
	}
}

/// HostFunctions that do not include benchmarking specific host functions
#[cfg(not(feature = "runtime-benchmarks"))]
pub type TWasmExecutor = WasmExecutor<sp_io::SubstrateHostFunctions>;

/// Host functions that include benchmarking specific functionalities
#[cfg(feature = "runtime-benchmarks")]
pub type TWasmExecutor = WasmExecutor<
	sc_executor::sp_wasm_interface::ExtendedHostFunctions<
		sp_io::SubstrateHostFunctions,
		frame_benchmarking::benchmarking::HostFunctions,
	>,
>;

impl<Block, RtApi, Exec> ClientProvider<Block> for DefaultClient<Block, RtApi, Exec>
where
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send + Sync + 'static,
	<RtApi as ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>>>::RuntimeApi:
		TaggedTransactionQueue<Block>
			+ BlockBuilderApi<Block>
			+ ApiExt<Block, StateBackend = <TFullBackend<Block> as Backend<Block>>::State>,
	Exec: CodeExecutor + RuntimeVersionOf,
{
	type Api = <TFullClient<Block, RtApi, Exec> as ProvideRuntimeApi<Block>>::Api;
	type Backend = TFullBackend<Block>;
	type Client = TFullClient<Block, RtApi, Exec>;
	type Exec = Exec;

	fn provide(
		&self,
		config: ClientConfig<Block>,
		genesis: Box<dyn BuildStorage>,
		execution_strategies: ExecutionStrategies,
		keystore: Option<SyncCryptoStorePtr>,
		backend: Arc<Self::Backend>,
		exec: LocalCallExecutor<Block, Self::Backend, Self::Exec>,
	) -> Result<Arc<Self::Client>, ()> {
		TFullClient::new(
			backend.clone(),
			exec,
			&(*genesis),
			None,
			None,
			sc_client_api::execution_extensions::ExecutionExtensions::new(
				execution_strategies,
				keystore,
				sc_offchain::OffchainDb::factory_from_backend(&*backend),
			),
			None,
			None,
			config,
		)
		.map_err(|_| ())
		.map(|client| Arc::new(client))
	}
}
