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

use std::{marker::PhantomData, str::FromStr, sync::Arc};

use sc_client_api::{backend, execution_extensions::ExecutionStrategies, UsageProvider};
use sc_executor::RuntimeVersionOf;
use sc_service::{
	ClientConfig, Configuration, KeystoreContainer, LocalCallExecutor, TFullBackend,
	TFullCallExecutor, TFullClient, TaskManager,
};
use sc_transaction_pool::{FullChainApi, FullPool, Options, RevalidationType};
use sp_api::{BlockT, ConstructRuntimeApi};
use sp_core::traits::{CodeExecutor, SpawnNamed};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::BuildStorage;
use tokio::runtime::Handle;

use crate::{provider::BackendProvider, GenesisState, Initiator};

/// A struct that holds configuration
/// options for a transaction pool.
pub struct PoolConfig {
	is_validator: bool,
	options: Options,
	revalidation: RevalidationType,
}

/// A structure that provides all necessary
/// configuration to instantiate the needed
/// structures for a core builder of fudge.
///
/// It implements `Initiator`.
pub struct Init<Block, RtApi, Exec>
where
	Block: BlockT,
{
	backend: Box<dyn BackendProvider<Block>>,
	genesis: Option<Box<dyn BuildStorage>>,
	handle: TaskManager,
	exec: Exec,
	pool_config: PoolConfig,
	/// Optional keystore that can be appended
	keystore: Option<SyncCryptoStorePtr>,
	/// Optional ClientConfig that can be appended
	client_config: Option<ClientConfig<Block>>,
	/// Optional ExecutionStrategies that can be appended
	execution_strategies: Option<ExecutionStrategies>,
	_phantom: PhantomData<(Block, RtApi, Exec)>,
}

impl<Block, RtApi, Exec> Init<Block, RtApi, Exec>
where
	Block: BlockT,
	Block::Hash: FromStr,
	RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
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
	/// 	_phantom: Default::default(),
	/// }
	/// ```
	///
	/// Every configuration field can be overwritten with the respective `with_*` method.
	pub fn new(backend: Box<dyn BackendProvider<Block>>, exec: Exec, handle: Handle) -> Self {
		Self {
			backend,
			genesis: None,
			handle: TaskManager::new(handle, None).unwrap(),
			exec,
			keystore: None,
			client_config: None,
			pool_config: PoolConfig {
				is_validator: true,
				options: Options::default(),
				revalidation: RevalidationType::Full,
			},
			execution_strategies: None,
			_phantom: Default::default(),
		}
	}

	/// Overwrites the used `ExecutionStrategies` that will be used when initiating the
	/// structs for a core builder.
	pub fn with_exec_strategies(&mut self, execution_strategies: ExecutionStrategies) -> &mut Self {
		self.execution_strategies = Some(execution_strategies);
		self
	}

	/// Overwrites the used genesis that will be used when initiating the
	/// structs for a core builder.
	pub fn with_genesis(&mut self, genesis: Box<dyn BuilStorage>) -> &mut Self {
		self.genesis = Some(genesis);
		self
	}

	/// Overwrites the used `ClientConfig` that will be used when initiating the
	/// structs for a core builder.
	pub fn with_config(&mut self, config: ClientConfig<Block>) -> &mut Self {
		self.client_config = Some(config);
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

	fn destruct<Backend>(
		self,
	) -> (
		Arc<Backend>,
		Box<dyn BuildStorage>,
		Exec,
		TaskManager,
		ExecutionStrategies,
		ClientConfig<Block>,
		Option<SyncCryptoStorePtr>,
		PoolConfig,
	)
	where
		Backend: backend::LocalBackend<Block> + 'static,
	{
		todo!()
	}
}

impl<Block, RtApi, Exec> Initiator<Block> for Init<Block, RtApi, Exec> {
	type Backend = BackendProvider<Block>::Backend;
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
		(),
	> {
		let (
			backend,
			genesis,
			executor,
			task_manager,
			execution_strategies,
			client_config,
			keystore,
			pool_config,
		) = self.destruct();

		let call_executor = LocalCallExecutor::new(
			backend.clone(),
			executor.clone(),
			Box::new(task_manager.spawn_handle()),
			client_config.clone(),
		)?;

		let extensions = sc_client_api::execution_extensions::ExecutionExtensions::new(
			execution_strategies,
			keystore,
			sc_offchain::OffchainDb::factory_from_backend(&*backend),
		);

		let client = Arc::new(sc_service::client::Client::<
			TFullBackend<Block>,
			TFullCallExecutor<Block, Exec>,
			Block,
			RtApi,
		>::new(
			backend.clone(),
			call_executor,
			&*genesis,
			None,
			None,
			extensions,
			None,
			None,
			client_config,
		)?);

		let pool = Arc::new(FullPool::<Block, C>::with_revalidation_type(
			pool_config.options,
			pool_config.is_validator.into(),
			Arc::new(FullChainApi::new(
				client.clone(),
				None,
				&manager.spawn_essential_handle(),
			)),
			None,
			pool_config.revalidation,
			manager.spawn_essential_handle(),
			client.usage_info().chain.best_number,
		));

		Ok((client, backend, pool, executor, task_manager))
	}
}

/// A structure that provides all necessary
/// configuration to instantiate the needed
/// structures for a core builder of fudge.
///
/// It implements `Initiator`. This
/// struct uses the `Configuration` struct
/// used by many services of actual Substrate nodes.
pub struct FromConfiguration<Block, RtApi, Exec, R> {
	exec: Exec,
	config: Configuration,
	keystore_receiver: R,
	pool_config: PoolConfig,
	_phantom: PhantomData<(Block, RtApi)>,
}

impl<Block, RtApi, Exec, R> FromConfiguration<Block, RtApi, Exec, R>
where
	R: FnOnce(KeyStoreContainer),
{
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
			keystore_receiver: |_| {},
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
	pub fn with_keystore_receiver(&mut self, receiver: R) -> &mut Self {
		self.keystore_receiver = receiver;
		self
	}
}

impl<Block, RtApi, Exec, R> Initiator<Block> for FromConfiguration<Block, RtApi, Exec, R> {
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
			sc_service::new_full_parts(&self.config, None, self.exec.clone())?;
		let client = Arc::new(client);

		let pool = Arc::new(FullPool::<Block, C>::with_revalidation_type(
			self.pool_config.options,
			self.pool_config.is_validator.into(),
			Arc::new(FullChainApi::new(
				client.clone(),
				None,
				&manager.spawn_essential_handle(),
			)),
			None,
			self.pool_config.revalidation,
			manager.spawn_essential_handle(),
			client.usage_info().chain.best_number,
		));

		self.keystore_receiver(keystore_container);
		Ok((client, backend, pool, self.exec, task_manager))
	}
}
