// Copyright 2021 Centrifuge Foundation (centrifuge.io).
//
// This file is part of the Centrifuge chain project.
// Centrifuge is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version (see http://www.gnu.org/licenses).
// Centrifuge is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use sc_executor::RuntimeVersionOf;
use sc_service::{ChainSpec, ClientConfig, Configuration, KeystoreContainer, LocalCallExecutor, TaskManager, TFullBackend, TFullCallExecutor, TFullClient};
use sc_service::config::ExecutionStrategies;
use sp_api::{BlockT, ConstructRuntimeApi};
use sp_core::traits::{CodeExecutor, SpawnNamed};
use sp_runtime::BuildStorage;
use sp_std::marker::PhantomData;
use sp_std::sync::Arc;
use sp_storage::Storage;
pub use externalities_provider::ExternalitiesProvider;
use crate::provider::state_provider::StateProvider;
use sp_keystore::SyncCryptoStorePtr;
use sp_std::str::FromStr;

mod externalities_provider;
mod state_provider;

pub struct EnvProvider<Block, RtApi, Exec>
    where
        Block: BlockT,
        Block::Hash: FromStr,
        RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send,
        Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static
{
    state: StateProvider<TFullBackend<Block>, Block>,
    _phantom: PhantomData<(Block, RtApi, Exec)>
}

impl<Block, RtApi, Exec> EnvProvider<Block, RtApi, Exec>
    where
        Block: BlockT,
        Block::Hash: FromStr,
        RtApi: ConstructRuntimeApi<Block, TFullClient<Block, RtApi, Exec>> + Send,
        Exec: CodeExecutor + RuntimeVersionOf + Clone + 'static
{
    pub fn empty() -> Self {
        Self {
            state: StateProvider::empty_default(None),
            _phantom: Default::default(),
        }
    }

    pub fn with_code(code: &'static [u8]) -> Self {
        Self {
            state: StateProvider::empty_default(Some(code)),
            _phantom: Default::default(),
        }
    }

    pub fn from_spec(spec: &dyn BuildStorage) -> Self {
        let storage = spec.build_storage().unwrap();
        Self::from_storage(storage)
    }

    pub fn into_from_config(config: &Configuration, exec: Exec) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>, KeystoreContainer, TaskManager) {
        //TODO: Handle unwrap
       sc_service::new_full_parts(config, None, exec).map_err(|_| "err".to_string()).unwrap()
    }

    pub fn from_storage(storage: Storage) -> Self {
        Self {
            state: StateProvider::from_storage(storage),
            _phantom: Default::default()
        }
    }

    pub fn from_storage_with_code(storage: Storage, code: &'static [u8]) -> Self {
        let mut state = StateProvider::empty_default(Some(code));
        state.insert_storage(storage);

        Self {
            state,
            _phantom: Default::default()
        }
    }

    pub fn insert_storage(&mut self, storage: Storage) -> &mut Self {
        self.state.insert_storage(storage);
        self
    }

    pub fn init_default(self, exec: Exec, handle: Box<dyn SpawnNamed>,) ->  (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
        self.init(exec, handle, None, None)
    }

    pub fn init_with_config(self, exec: Exec, handle: Box<dyn SpawnNamed>, config: ClientConfig<Block>) ->  (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
        self.init(exec,  handle, None, Some(config))
    }

    pub fn init_full(self, exec: Exec, handle: Box<dyn SpawnNamed>, keystore: SyncCryptoStorePtr, config: ClientConfig<Block>) ->  (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
        self.init(exec, handle,Some(keystore), Some(config))
    }

    pub fn init_with_keystore(self, exec: Exec, handle: Box<dyn SpawnNamed>, keystore: SyncCryptoStorePtr) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
        self.init(exec, handle, Some(keystore), None)
    }

    fn init(self, exec: Exec, handle: Box<dyn SpawnNamed>, keystore: Option<SyncCryptoStorePtr>, config: Option<ClientConfig<Block>>) -> (TFullClient<Block, RtApi, Exec>, Arc<TFullBackend<Block>>) {
        let backend = self.state.backend();
        let mut config = config.clone().unwrap_or(Default::default());
        config.no_genesis = true;
        // TODO: Handle unwrap
        let executor = sc_service::client::LocalCallExecutor::new(
            backend.clone(),
            exec,
            handle,
            config.clone()
        ).unwrap();

        // TODO: Execution strategies default is not right. Use always wasm instead
        let extensions = sc_client_api::execution_extensions::ExecutionExtensions::new(
            ExecutionStrategies::default(),
            keystore,
            sc_offchain::OffchainDb::factory_from_backend(&*backend),
        );

        // TODO: Client config pass?
        // Client<TFullBackend<TBl>, TFullCallExecutor<TBl, TExec>, TBl, TRtApi>;
        let client = sc_service::client::Client::<TFullBackend<Block>, TFullCallExecutor<Block, Exec>, Block, RtApi>::new(
            backend.clone(),
            executor,
            &self.state,
            None,
            None,
            extensions,
            None,
            None,
            config.clone()
        ).map_err(|_| "err".to_string()).unwrap();

        (client, backend)
    }
}