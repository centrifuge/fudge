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

pub use externalities_provider::ExternalitiesProvider;
use sc_executor::RuntimeVersionOf;
use sc_service::{
	config::ExecutionStrategies, ClientConfig, Configuration, KeystoreContainer, TFullBackend,
	TFullCallExecutor, TFullClient, TaskManager,
};
use sp_api::{BlockT, ConstructRuntimeApi};
use sp_core::traits::{CodeExecutor, SpawnNamed};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::BuildStorage;
use sp_std::{marker::PhantomData, str::FromStr, sync::Arc};
use sp_storage::Storage;
use thiserror::Error;

pub use crate::provider::state_provider::DbOpen;
use crate::provider::{
	state_provider::StateProvider,
	Error::{StateProviderError, StorageBuilderError},
};

mod externalities_provider;
mod state_provider;

const DEFAULT_ENV_PROVIDER_LOG_TARGET: &str = "fudge-env";

#[derive(Error, Debug)]
pub enum Error {
	#[error("full node parts creation: {0}")]
	FullNodePartsCreation(Box<dyn std::error::Error>),

	#[error("local call executor creation: {0}")]
	LocalCallExecutorCreation(Box<dyn std::error::Error>),

	#[error("substrate client creation: {0}")]
	SubstrateClientCreation(Box<dyn std::error::Error>),

	#[error("state provider: {0}")]
	StateProviderError(Box<dyn std::error::Error>),

	#[error("storage builder: {0}")]
	StorageBuilderError(Box<dyn std::error::Error>),
}

pub struct EnvProvider<Block, RtApi, Exec>
where
	Block: BlockT,
{
	state: StateProvider<TFullBackend<Block>, Block>,
	_phantom: PhantomData<(Block, RtApi, Exec)>,
}

impl<Block, RtApi, Exec> EnvProvider<Block, RtApi, Exec>
where
	Block: BlockT,
	Block::Hash: FromStr,
	RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send,
	Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static,
{
	pub fn empty() -> Result<Self, Error> {
		Ok(Self {
			state: StateProvider::empty_default(None).map_err(|e| StateProviderError(e.into()))?,
			_phantom: Default::default(),
		})
	}

	pub fn with_code(code: &'static [u8]) -> Result<Self, Error> {
		Ok(Self {
			state: StateProvider::empty_default(Some(code))
				.map_err(|e| StateProviderError(e.into()))?,
			_phantom: Default::default(),
		})
	}

	pub fn from_spec(spec: &dyn BuildStorage) -> Result<Self, Error> {
		let storage = spec
			.build_storage()
			.map_err(|e| StorageBuilderError(Box::<dyn std::error::Error>::from(e)))?;

		Self::from_storage(storage)
	}

	pub fn from_config(
		config: &Configuration,
		exec: Exec,
	) -> Result<
		(
			TFullClient<Block, RtApi, Exec>,
			Arc<TFullBackend<Block>>,
			KeystoreContainer,
			TaskManager,
		),
		Error,
	> {
		sc_service::new_full_parts(config, None, exec).map_err(|e| {
			tracing::error!(
				target = DEFAULT_ENV_PROVIDER_LOG_TARGET,
				error = ?e,
				"Couldn't get full node parts."
			);

			Error::FullNodePartsCreation(e.into())
		})
	}

	pub fn from_storage(storage: Storage) -> Result<Self, Error> {
		Ok(Self {
			state: StateProvider::from_storage(storage)
				.map_err(|e| StateProviderError(e.into()))?,
			_phantom: Default::default(),
		})
	}

	pub fn from_db(open: DbOpen) -> Result<Self, Error> {
		Ok(Self {
			state: StateProvider::from_db(open).map_err(|e| StateProviderError(e.into()))?,
			_phantom: Default::default(),
		})
	}

	pub fn from_storage_with_code(storage: Storage, code: &'static [u8]) -> Result<Self, Error> {
		let mut state =
			StateProvider::empty_default(Some(code)).map_err(|e| StateProviderError(e.into()))?;

		state
			.insert_storage(storage)
			.map_err(|e| StateProviderError(e.into()))?;

		Ok(Self {
			state,
			_phantom: Default::default(),
		})
	}

	pub fn insert_storage(&mut self, storage: Storage) -> Result<&mut Self, Error> {
		self.state
			.insert_storage(storage)
			.map_err(|e| StateProviderError(e.into()))?;
		Ok(self)
	}

	pub fn init_default(
		self,
		exec: Exec,
		handle: Box<dyn SpawnNamed>,
	) -> Result<(TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>), Error> {
		self.init(exec, handle, None, None)
	}

	pub fn init_with_config(
		self,
		exec: Exec,
		handle: Box<dyn SpawnNamed>,
		config: ClientConfig<Block>,
	) -> Result<(TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>), Error> {
		self.init(exec, handle, None, Some(config))
	}

	pub fn init_full(
		self,
		exec: Exec,
		handle: Box<dyn SpawnNamed>,
		keystore: SyncCryptoStorePtr,
		config: ClientConfig<Block>,
	) -> Result<(TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>), Error> {
		self.init(exec, handle, Some(keystore), Some(config))
	}

	pub fn init_with_keystore(
		self,
		exec: Exec,
		handle: Box<dyn SpawnNamed>,
		keystore: SyncCryptoStorePtr,
	) -> Result<(TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>), Error> {
		self.init(exec, handle, Some(keystore), None)
	}

	fn init(
		self,
		exec: Exec,
		handle: Box<dyn SpawnNamed>,
		keystore: Option<SyncCryptoStorePtr>,
		config: Option<ClientConfig<Block>>,
	) -> Result<(TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>), Error> {
		let backend = self.state.backend();
		let config = config.clone().unwrap_or(Self::client_config());

		let executor = sc_service::client::LocalCallExecutor::new(
			backend.clone(),
			exec,
			handle,
			config.clone(),
		)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_ENV_PROVIDER_LOG_TARGET,
				error = ?e,
				"Couldn't get full node parts."
			);

			Error::LocalCallExecutorCreation(e.into())
		})?;

		// TODO: Execution strategies default is not right. Use always wasm instead
		let extensions = sc_client_api::execution_extensions::ExecutionExtensions::new(
			ExecutionStrategies::default(),
			keystore,
			sc_offchain::OffchainDb::factory_from_backend(&*backend),
		);

		// TODO: Client config pass?
		let client = sc_service::client::Client::<
			TFullBackend<Block>,
			TFullCallExecutor<Block, Exec>,
			Block,
			RtApi,
		>::new(
			backend.clone(),
			executor,
			&self.state,
			None,
			None,
			extensions,
			None,
			None,
			config.clone(),
		)
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_ENV_PROVIDER_LOG_TARGET,
				error = ?e,
				"Couldn't get full node parts."
			);

			Error::SubstrateClientCreation(e.into())
		})?;

		Ok((client, backend))
	}

	fn client_config() -> ClientConfig<Block> {
		ClientConfig::default()
	}
}
