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
use sc_transaction_pool::{FullChainApi, FullPool, RevalidationType};
use sp_api::{BlockT, ConstructRuntimeApi};
use sp_core::traits::{CodeExecutor, SpawnNamed};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::BuildStorage;
use tokio::runtime::Handle;

use crate::{provider::BackendProvider, GenesisState, Initiator};

pub struct Init<Block, RtApi, Exec>
where
	Block: BlockT,
{
	backend: Box<dyn BackendProvider<Block>>,
	genesis: Option<Box<dyn BuildStorage>>,
	handle: TaskManager,
	exec: Exec,
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
	pub fn new(backend: Box<dyn BackendProvider<Block>>, exec: Exec, handle: Handle) -> Self {
		Self {
			backend,
			genesis: None,
			handle: TaskManager::new(handle, None).unwrap(),
			exec,
			keystore: None,
			client_config: None,
			execution_strategies: None,
			_phantom: Default::default(),
		}
	}

	pub fn with_exec_strategies(&mut self, execution_strategies: ExecutionStrategies) {
		self.execution_strategies = Some(execution_strategies);
	}

	pub fn with_genesis(&mut self, genesis: Box<dyn BuilStorage>) {
		self.genesis = Some(genesis);
	}

	pub fn with_config(&mut self, config: ClientConfig<Block>) {
		self.client_config = Some(config);
	}

	pub fn with_keystore(&mut self, keystore: SyncCryptoStorePtr) {
		self.keystore = Some(keystore);
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
	type Executor = Exec;
	type Pool = FullPool<Block, Self::Client>;

	fn init(
		self,
	) -> Result<
		(
			TFullClient<Block, RtApi, Exec>,
			TFullBackend<Block>,
			FullPool<Block, TFullClient<Block, RtApi, Exec>>,
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
		) = self.destruct();

		let call_executor = sc_service::client::LocalCallExecutor::new(
			backend.clone(),
			executor.clone(),
			Box::new(task_manager.spawn_handle()),
			client_config.clone(),
		)
		.unwrap();

		let extensions = sc_client_api::execution_extensions::ExecutionExtensions::new(
			execution_strategies,
			keystore,
			sc_offchain::OffchainDb::factory_from_backend(&*backend),
		);

		let client = Arc::new(
			sc_service::client::Client::<
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
			)
			.map_err(|_| "err".to_string())
			.unwrap(),
		);

		let pool = Arc::new(FullPool::<Block, C>::with_revalidation_type(
			Default::default(),
			true.into(),
			Arc::new(FullChainApi::new(
				client.clone(),
				None,
				&manager.spawn_essential_handle(),
			)),
			None,
			RevalidationType::Full,
			manager.spawn_essential_handle(),
			client.usage_info().chain.best_number,
		));

		Ok((client, backend, pool, executor, task_manager))
	}
}

/*
Actually create Initiator froma Configuration and an executor

pub struct FromConfiguration<Block, RtApi, Exec, R> {
	exec: Exec,
	config: Configuration,
	keystore_receiver: R;
	_phantom: PhantomData<(Block, RtApi)>,
};

impl<Block, RtApi, Exec, R> FromConfiguration<Block, RtApi, Exec, R>
where
	R: FnOnce(KeyStoreContainer)
{
	pub fn new(exec: Exec, config: Configuration, keystore_receiver: R) -> Self {
		Self {
			exec,
			config,
			keystore_receiver,
			_phantom: Default::default()
		}
	 }
}

impl Initiator<Block, Exec, RtApi> for FromConfiguration<Block, RtApi, Exec> {
	type Backend = Arc<TFullBackend<Block>>;
	type Client = Arc<TFullClient<Block, RtApi, Exec>>;
	type Executor = Exec;
	type Pool = Arc<FullPool<Block, Self::Client>>;

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
		let (client, backend, keystore_container, task_manager) = sc_service::new_full_parts(self.config, None, self.exec.clone())
				.unwrap();
		let client = Arc::new(client);

		let pool = Arc::new(FullPool::<Block, C>::with_revalidation_type(
			Default::default(),
			true.into(),
			Arc::new(FullChainApi::new(
				client.clone(),
				None,
				&manager.spawn_essential_handle(),
			)),
			None,
			RevalidationType::Full,
			manager.spawn_essential_handle(),
			client.usage_info().chain.best_number,
		));

		self.keystore_receiver(keystore_container);
		Ok((client, backend, pool, self.exec, task_manager))
	}
}

 */
