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

//! The module provides a struct that implements `trait Initiator`.
//! Builders will expect something that implements this trait in
//! order to retrieve a `client` and a `backend`.

use std::{marker::PhantomData, sync::Arc};

use polkadot_cli::service::HeaderBackend;
use sc_block_builder::{BlockBuilderApi, BlockBuilderProvider};
use sc_client_api::{
	execution_extensions::{ExecutionExtensions, ExecutionStrategies},
	AuxStore, Backend, BlockBackend, BlockOf, TransactionFor, UsageProvider,
};
use sc_consensus::BlockImport;
use sc_executor::{
	sp_wasm_interface::HostFunctions, HeapAllocStrategy, RuntimeVersionOf, WasmExecutor,
};
use sc_service::{
	ClientConfig, Configuration, GenesisBlockBuilder, KeystoreContainer, LocalCallExecutor,
	TFullBackend, TFullClient, TaskManager,
};
use sc_transaction_pool::{FullChainApi, FullPool, Options, RevalidationType};
use sp_api::{ApiExt, BlockT, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_blockchain::{Error as BlockChainError, HeaderMetadata};
use sp_core::traits::{CodeExecutor, ReadRuntimeVersion};
use sp_runtime::{traits::BlockIdTo, BuildStorage};
use sp_storage::Storage;
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use thiserror::Error;
use tokio::runtime::Handle;

use crate::provider::{
	backend::MemDb, BackendProvider, ClientProvider, DefaultClient, Initiator, InnerError,
	TWasmExecutor,
};

const DEFAULT_INIT_LOG_TARGET: &str = "fudge-init";

#[derive(Error, Debug)]
pub enum Error {
	#[error("task manager creation: {0}")]
	TaskManagerCreation(InnerError),

	#[error("backend provider: {0}")]
	BackendProvider(InnerError),

	#[error("call executor creation: {0}")]
	CallExecutorCreation(InnerError),

	#[error("genesis block builder creation: {0}")]
	GenesisBlockBuilderCreation(InnerError),

	#[error("client provider: {0}")]
	ClientProvider(InnerError),

	#[error("full parts creation: {0}")]
	FullPartsCreation(InnerError),
}

/// A struct that holds configuration
/// options for a transaction pool.
pub struct PoolConfig {
	is_validator: bool,
	options: Options,
	revalidation: RevalidationType,
}

fn build_wasm_executor<H>() -> WasmExecutor<H>
where
	H: HostFunctions + Send + Sync,
{
	let heap_alloc_strategy = HeapAllocStrategy::Static { extra_pages: 8 };
	WasmExecutor::<H>::builder()
		.with_max_runtime_instances(8)
		.with_runtime_cache_size(2)
		.with_onchain_heap_alloc_strategy(heap_alloc_strategy)
		.with_offchain_heap_alloc_strategy(heap_alloc_strategy)
		.build()
}

pub fn default_with<Block, RtApi, BP>(
	handle: Handle,
	backend: BP,
) -> Init<Block, DefaultClient<Block, RtApi, TWasmExecutor>, BP>
where
	BP: BackendProvider<Block, Backend = TFullBackend<Block>>,
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, TWasmExecutor>>
		+ Send
		+ Sync
		+ 'static,
	<RtApi as ConstructRuntimeApi<Block, TFullClient<Block, RtApi, TWasmExecutor>>>::RuntimeApi:
		TaggedTransactionQueue<Block>
			+ BlockBuilderApi<Block>
			+ ApiExt<Block, StateBackend = <TFullBackend<Block> as Backend<Block>>::State>,
{
	Init::new(
		backend,
		DefaultClient::<Block, RtApi, TWasmExecutor>::new(),
		build_wasm_executor(),
		handle,
	)
}

pub fn default<Block, RtApi>(
	handle: Handle,
) -> Init<Block, DefaultClient<Block, RtApi, TWasmExecutor>, MemDb<Block>>
where
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, TWasmExecutor>>
		+ Send
		+ Sync
		+ 'static,
	<RtApi as ConstructRuntimeApi<Block, TFullClient<Block, RtApi, TWasmExecutor>>>::RuntimeApi:
		TaggedTransactionQueue<Block>
			+ BlockBuilderApi<Block>
			+ ApiExt<Block, StateBackend = <TFullBackend<Block> as Backend<Block>>::State>,
{
	Init::new(
		MemDb::new(),
		DefaultClient::<Block, RtApi, TWasmExecutor>::new(),
		build_wasm_executor(),
		handle,
	)
}

/// A structure that provides all necessary
/// configuration to instantiate the needed
/// structures for a core builder of fudge.
///
/// It implements `Initiator`.
pub struct Init<Block, CP, BP>
where
	Block: BlockT,
	CP: ClientProvider<Block>,
{
	backend_provider: BP,
	client_provider: CP,
	genesis: Box<dyn BuildStorage>,
	handle: Handle,
	exec: CP::Exec,
	pool_config: PoolConfig,
	/// Optional ClientConfig that can be appended
	client_config: ClientConfig<Block>,
	execution_extensions: ExecutionExtensions<Block>,
}

impl<Block, CP, BP> Init<Block, CP, BP>
where
	Block: BlockT,
	CP: ClientProvider<Block>,
	BP: BackendProvider<Block, Backend = CP::Backend>,
	CP::Backend: Backend<Block> + 'static,
	CP::Client: 'static
		+ ProvideRuntimeApi<Block, Api = CP::Api>
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
		+ BlockBuilderProvider<CP::Backend, Block, CP::Client>,
	CP::Exec: Clone + ReadRuntimeVersion,
	for<'r> &'r CP::Client: BlockImport<Block, Transaction = TransactionFor<CP::Backend, Block>>,
{
	/// Creates a new `Init` instance with some sane defaults:
	///
	/// ```ignore
	/// Self {
	/// 	backend_provider,
	/// 	client_provider,
	/// 	genesis: Box::new(Storage::default()),
	/// 	handle: handle,
	/// 	exec,
	/// 	client_config: ClientConfig::default(),
	/// 	pool_config: PoolConfig {
	/// 		is_validator: true,
	/// 		options: Options::default(),
	/// 		revalidation: RevalidationType::Full,
	/// 	},
	/// 	execution_extensions: ExecutionExtensions::default(),
	/// }
	/// ```
	///
	/// Every configuration field can be overwritten with the respective `with_*` method.
	pub fn new(backend_provider: BP, client_provider: CP, exec: CP::Exec, handle: Handle) -> Self {
		Self {
			backend_provider,
			client_provider,
			genesis: Box::new(Storage::default()),
			handle,
			exec: exec.clone(),
			client_config: ClientConfig::default(),
			pool_config: PoolConfig {
				is_validator: true,
				options: Options::default(),
				revalidation: RevalidationType::Full,
			},
			execution_extensions: ExecutionExtensions::new(
				ExecutionStrategies::default(),
				None,
				None,
				Arc::new(exec),
			),
		}
	}

	/// Overwrites the used `ExecutionExtensions` that will be used when initiating the
	/// structs for a core builder.
	pub fn with_exec_extensions(
		&mut self,
		execution_extensions: ExecutionExtensions<Block>,
	) -> &mut Self {
		self.execution_extensions = execution_extensions;
		self
	}

	/// Overwrites the `GenesisBlockBuilder` with a new one that contains the provided genesis.
	pub fn with_genesis(&mut self, genesis: Box<dyn BuildStorage>) -> &mut Self {
		self.genesis = genesis;
		self
	}

	/// Overwrites the used `ClientConfig` that will be used when initiating the
	/// structs for a core builder.
	pub fn with_config(&mut self, config: ClientConfig<Block>) -> &mut Self {
		self.client_config = config;
		self
	}

	/// Overwrites the used `PoolConfig` that will be used when initiating the
	/// structs for a core builder.
	pub fn with_pool_config(&mut self, pool_config: PoolConfig) -> &mut Self {
		self.pool_config = pool_config;
		self
	}
}

impl<Block, CP, BP> Initiator<Block> for Init<Block, CP, BP>
where
	Block: BlockT,
	CP: ClientProvider<Block>,
	BP: BackendProvider<Block, Backend = CP::Backend>,
	CP::Backend: Backend<Block> + 'static,
	CP::Client: 'static
		+ ProvideRuntimeApi<Block, Api = CP::Api>
		+ HeaderMetadata<Block, Error = BlockChainError>
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
		+ BlockBuilderProvider<CP::Backend, Block, CP::Client>,
	for<'r> &'r CP::Client: BlockImport<Block, Transaction = TransactionFor<CP::Backend, Block>>,
{
	type Api = CP::Api;
	type Backend = CP::Backend;
	type Client = CP::Client;
	type Error = Error;
	type Executor = CP::Exec;
	type Pool = FullPool<Block, Self::Client>;

	fn init(
		self,
	) -> Result<
		(
			Arc<CP::Client>,
			Arc<CP::Backend>,
			Arc<FullPool<Block, CP::Client>>,
			CP::Exec,
			TaskManager,
		),
		Error,
	> {
		let task_manager = TaskManager::new(self.handle, None).map_err(|e| {
			tracing::error!(
				target = DEFAULT_INIT_LOG_TARGET,
				error = ?e,
				"Could not create task manager."
			);

			Error::TaskManagerCreation(e.into())
		})?;

		let backend = self.backend_provider.provide().map_err(|e| {
			tracing::error!(
				target = DEFAULT_INIT_LOG_TARGET,
				error = ?e,
				"Could not provide backed."
			);

			Error::BackendProvider(e.into())
		})?;

		let call_executor = LocalCallExecutor::new(
			backend.clone(),
			self.exec.clone(),
			self.client_config.clone(),
			self.execution_extensions,
		)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_INIT_LOG_TARGET,
				error = ?e,
				"Could not create call executor."
			);

			Error::CallExecutorCreation(e.into())
		})?;

		let genesis_block_builder = GenesisBlockBuilder::new(
			&(*self.genesis),
			!self.client_config.no_genesis,
			backend.clone(),
			self.exec.clone(),
		)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_INIT_LOG_TARGET,
				error = ?e,
				"Could not create genesis block builder."
			);

			Error::GenesisBlockBuilderCreation(e.into())
		})?;

		let client = self
			.client_provider
			.provide(
				self.client_config,
				genesis_block_builder,
				backend.clone(),
				call_executor,
				Box::new(task_manager.spawn_handle()),
			)
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_INIT_LOG_TARGET,
					error = ?e,
					"Could not provide client."
				);

				Error::ClientProvider(e.into())
			})?;

		let pool = Arc::new(FullPool::<Block, CP::Client>::with_revalidation_type(
			self.pool_config.options,
			self.pool_config.is_validator.into(),
			Arc::new(FullChainApi::new(
				client.clone(),
				None,
				&task_manager.spawn_essential_handle(),
			)),
			None,
			self.pool_config.revalidation,
			task_manager.spawn_essential_handle(),
			client.usage_info().chain.best_number,
			client.usage_info().chain.best_hash,
			client.usage_info().chain.finalized_hash,
		));

		Ok((client, backend, pool, self.exec, task_manager))
	}
}

/// A structure that provides all necessary
/// configuration to instantiate the needed
/// structures for a core builder of fudge.
///
/// It implements `Initiator`. This
/// struct uses the `Configuration` struct
/// used by many services of actual Substrate nodes.
pub struct FromConfiguration<Block, RtApi, Exec> {
	exec: Exec,
	config: Configuration,
	keystore_receiver: Box<dyn FnOnce(KeystoreContainer)>,
	pool_config: PoolConfig,
	_phantom: PhantomData<(Block, RtApi)>,
}

impl<Block, RtApi, Exec> FromConfiguration<Block, RtApi, Exec> {
	/// Creates a new instance of `FromConfiguration` with some
	/// sane defaults.
	///
	/// ```ignore
	/// Self {
	/// 	exec,
	/// 	config,
	/// 	keystore_receiver: |_| {},
	/// 	pool_config: PoolConfig {
	/// 		is_validator: true,
	/// 		options: Options::default(),
	/// 		revalidation: RevalidationType::Full,
	/// 	},
	/// 	_phantom: Default::default(),
	/// }
	/// ```
	/// The given defaults can be overwritten with the
	/// respective `with_*` methods.
	pub fn new(exec: Exec, config: Configuration) -> Self {
		Self {
			exec,
			config,
			keystore_receiver: Box::new(|_| {}),
			pool_config: PoolConfig {
				is_validator: true,
				options: Options::default(),
				revalidation: RevalidationType::Full,
			},
			_phantom: Default::default(),
		}
	}

	/// Overwrites the used `PoolConfig` that will be used when initiating the
	/// structs for a core builder.
	pub fn with_pool_config(&mut self, pool_config: PoolConfig) -> &mut Self {
		self.pool_config = pool_config;
		self
	}

	/// Overwrites the used keystore receiver that will be used when initiating the
	/// structs for a core builder.
	pub fn with_keystore_receiver<R>(&mut self, receiver: R) -> &mut Self
	where
		R: FnOnce(KeystoreContainer) + 'static,
	{
		self.keystore_receiver = Box::new(receiver);
		self
	}
}

impl<Block, RtApi, Exec> Initiator<Block> for FromConfiguration<Block, RtApi, Exec>
where
	Block: BlockT,
	RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send + Sync + 'static,
	<RtApi as ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>>>::RuntimeApi:
		TaggedTransactionQueue<Block>
			+ BlockBuilderApi<Block>
			+ ApiExt<Block, StateBackend = <TFullBackend<Block> as Backend<Block>>::State>,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
{
	type Api = RtApi::RuntimeApi;
	type Backend = TFullBackend<Block>;
	type Client = TFullClient<Block, RtApi, Exec>;
	type Error = Error;
	type Executor = Exec;
	type Pool = FullPool<Block, Self::Client>;

	fn init(
		self,
	) -> Result<
		(
			Arc<TFullClient<Block, RtApi, Exec>>,
			Arc<TFullBackend<Block>>,
			Arc<FullPool<Block, TFullClient<Block, RtApi, Exec>>>,
			Exec,
			TaskManager,
		),
		Self::Error,
	> {
		let (client, backend, keystore_container, task_manager) =
			sc_service::new_full_parts(&self.config, None, self.exec.clone()).map_err(|e| {
				tracing::error!(
					target = DEFAULT_INIT_LOG_TARGET,
					error = ?e,
					"Could not create full parts."
				);

				Error::FullPartsCreation(e.into())
			})?;

		let client = Arc::new(client);

		let pool = Arc::new(FullPool::<Block, Self::Client>::with_revalidation_type(
			self.pool_config.options,
			self.pool_config.is_validator.into(),
			Arc::new(FullChainApi::new(
				client.clone(),
				None,
				&task_manager.spawn_essential_handle(),
			)),
			None,
			self.pool_config.revalidation,
			task_manager.spawn_essential_handle(),
			client.usage_info().chain.best_number,
			client.usage_info().chain.best_hash,
			client.usage_info().chain.finalized_hash,
		));

		(self.keystore_receiver)(keystore_container);
		Ok((client, backend, pool, self.exec, task_manager))
	}
}
