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
	execution_extensions::ExecutionStrategies, AuxStore, Backend, BlockBackend, BlockOf,
	UsageProvider,
};
use sc_consensus::BlockImport;
use sc_executor::{RuntimeVersionOf, WasmExecutionMethod, WasmExecutor};
use sc_service::{
	ClientConfig, Configuration, KeystoreContainer, LocalCallExecutor, TFullBackend, TFullClient,
	TaskManager,
};
use sc_transaction_pool::{FullChainApi, FullPool, Options, RevalidationType};
use sp_api::{ApiExt, BlockT, CallApiAt, ConstructRuntimeApi, ProvideRuntimeApi};
use sp_core::traits::CodeExecutor;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::{traits::BlockIdTo, BuildStorage};
use sp_storage::Storage;
use sp_transaction_pool::runtime_api::TaggedTransactionQueue;
use tokio::runtime::Handle;

use crate::{
	provider::{BackendProvider, ClientProvider, DefaultClient, TWasmExecutor},
	Initiator, MemDb,
};

/// A struct that holds configuration
/// options for a transaction pool.
pub struct PoolConfig {
	is_validator: bool,
	options: Options,
	revalidation: RevalidationType,
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
		WasmExecutor::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
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
		WasmExecutor::new(WasmExecutionMethod::Interpreted, Some(8), 8, None, 2),
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
	/// Optional keystore that can be appended
	keystore: Option<SyncCryptoStorePtr>,
	/// Optional ClientConfig that can be appended
	client_config: ClientConfig<Block>,
	/// Optional ExecutionStrategies that can be appended
	execution_strategies: ExecutionStrategies,
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
{
	/// Creates a new `Init` instance with some sane defaults:
	///
	/// ```ignore
	/// Self {
	/// 	backend,
	/// 	genesis: None,
	/// 	handle: TaskManager::new(handle, None).unwrap(),
	/// 	exec,
	/// 	keystore: None,
	/// 	client_config: None,
	/// 	pool_config: PoolConfig {
	/// 		is_validator: true,
	/// 		options: Options::default(),
	/// 		revalidation: RevalidationType::Full,
	/// 	},
	/// 	execution_strategies: None,
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
			exec,
			keystore: None,
			client_config: ClientConfig::default(),
			pool_config: PoolConfig {
				is_validator: true,
				options: Options::default(),
				revalidation: RevalidationType::Full,
			},
			execution_strategies: ExecutionStrategies::default(),
		}
	}

	/// Overwrites the used `ExecutionStrategies` that will be used when initiating the
	/// structs for a core builder.
	pub fn with_exec_strategies(&mut self, execution_strategies: ExecutionStrategies) -> &mut Self {
		self.execution_strategies = execution_strategies;
		self
	}

	/// Overwrites the used genesis that will be used when initiating the
	/// structs for a core builder.
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

	/// Overwrites the used keystore pointer that will be used when initiating the
	/// structs for a core builder.
	pub fn with_keystore(&mut self, keystore: SyncCryptoStorePtr) -> &mut Self {
		self.keystore = Some(keystore);
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
{
	type Api = CP::Api;
	type Backend = CP::Backend;
	type Client = CP::Client;
	type Error = sp_blockchain::Error;
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
		sp_blockchain::Error,
	> {
		let task_manager = TaskManager::new(self.handle, None).unwrap();
		let backend = self.backend_provider.provide().unwrap();
		let call_executor = LocalCallExecutor::new(
			backend.clone(),
			self.exec.clone(),
			Box::new(task_manager.spawn_handle()),
			self.client_config.clone(),
		)
		.unwrap();
		let client = self
			.client_provider
			.provide(
				self.client_config,
				self.genesis,
				self.execution_strategies,
				self.keystore,
				backend.clone(),
				call_executor,
			)
			.unwrap();

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
	type Error = sp_blockchain::Error;
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
			sc_service::new_full_parts(&self.config, None, self.exec.clone()).unwrap(); // TODO NEED own error type
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
		));

		(self.keystore_receiver)(keystore_container);
		Ok((client, backend, pool, self.exec, task_manager))
	}
}
