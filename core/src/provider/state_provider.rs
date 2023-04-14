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

use std::path::PathBuf;

use sc_client_api::Backend;
use sc_client_db::{BlocksPruning, DatabaseSettings, DatabaseSource};
use sc_service::PruningMode;
use sp_core::storage::well_known_keys::CODE;
use sp_database::MemDb;
use sp_runtime::{traits::Block as BlockT, BuildStorage};
use sp_std::{marker::PhantomData, sync::Arc};
use sp_storage::Storage;
use thiserror::Error;

const DEFAULT_STATE_PROVIDER_LOG_TARGET: &str = "fudge-state";

#[derive(Error, Debug)]
pub enum Error {
	#[error("incompatible child info update")]
	IncompatibleChildInfoUpdate,

	#[error("DB backend creation: {0}")]
	DBBackendCreation(Box<dyn std::error::Error>),
}

pub const CANONICALIZATION_DELAY: u64 = 4096;

pub struct StateProvider<B, Block> {
	backend: Arc<B>,
	pseudo_genesis: Storage,
	_phantom: PhantomData<Block>,
}

impl<B, Block> StateProvider<B, Block>
where
	Block: BlockT,
	B: Backend<Block>,
{
	pub fn insert_storage(&mut self, storage: Storage) -> Result<&mut Self, Error> {
		let Storage {
			top,
			children_default: _children_default,
		} = storage;

		self.pseudo_genesis.top.extend(top.into_iter());

		for (k, other_map) in children_default.iter() {
			let k = k.clone();
			if let Some(map) = self.pseudo_genesis.children_default.get_mut(&k) {
				map.data
					.extend(other_map.data.iter().map(|(k, v)| (k.clone(), v.clone())));
				if !map.child_info.try_update(&other_map.child_info) {
					return Err(Error::IncompatibleChildInfoUpdate);
				}
			} else {
				self.pseudo_genesis
					.children_default
					.insert(k, other_map.clone());
			}
		}

		Ok(self)
	}

	pub fn backend(&self) -> Arc<B> {
		self.backend.clone()
	}
}

impl<Block> StateProvider<sc_client_db::Backend<Block>, Block>
where
	Block: BlockT,
{
	pub fn from_db(open: DbOpen) -> Result<Self, Error> {
		let settings = match open {
			DbOpen::FullConfig(settings) => settings,
			DbOpen::SparseConfig {
				path,
				state_pruning,
			} => DatabaseSettings {
				trie_cache_maximum_size: None,
				state_pruning: Some(state_pruning),
				source: DatabaseSource::RocksDb {
					path: path,
					cache_size: 1024,
				},
				blocks_pruning: BlocksPruning::All,
			},
			DbOpen::Default(path) => DatabaseSettings {
				trie_cache_maximum_size: None,
				state_pruning: None,
				source: DatabaseSource::RocksDb {
					path: path,
					cache_size: 1024,
				},
				blocks_pruning: BlocksPruning::All,
			},
			blocks_pruning: BlocksPruning::All,
		};

		let backend = Arc::new(
			sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY).map_err(|e| {
				tracing::error!(
					target = DEFAULT_STATE_PROVIDER_LOG_TARGET,
					error = ?e,
					"Could not create DB backend."
				);

				Error::DBBackendCreation(e.into())
			})?,
		);

		Ok(Self {
			backend,
			pseudo_genesis: Storage::default(),
			_phantom: Default::default(),
		})
	}

	pub fn from_spec() -> Self {
		todo!()
	}

	pub fn from_storage(storage: Storage) -> Result<Self, Error> {
		let mut provider = StateProvider::empty_default(None)?;
		provider.insert_storage(storage)?;
		Ok(provider)
	}

	pub fn empty_default(code: Option<&[u8]>) -> Result<Self, Error> {
		let mut provider = StateProvider::with_in_mem_db()?;

		let mut storage = Storage::default();
		if let Some(code) = code {
			storage.top.insert(CODE.to_vec(), code.to_vec());
		}

		provider.insert_storage(storage)?;

		Ok(provider)
	}

	fn with_in_mem_db() -> Result<Self, Error> {
		let settings = DatabaseSettings {
			trie_cache_maximum_size: None,
			state_pruning: Some(PruningMode::ArchiveAll),
			source: DatabaseSource::Custom {
				db: Arc::new(MemDb::new()),
				require_create_flag: true,
			},
			blocks_pruning: BlocksPruning::All,
		};

		let backend = Arc::new(
			sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY).map_err(|e| {
				tracing::error!(
					target = DEFAULT_STATE_PROVIDER_LOG_TARGET,
					error = ?e,
					"Could not create DB backend."
				);

				Error::DBBackendCreation(e.into())
			})?,
		);

		Ok(Self {
			backend,
			pseudo_genesis: Storage::default(),
			_phantom: Default::default(),
		})
	}
}

impl<B, Block> BuildStorage for StateProvider<B, Block>
where
	Block: BlockT,
	B: Backend<Block>,
{
	fn build_storage(&self) -> Result<Storage, String> {
		Ok(self.pseudo_genesis.clone())
	}

	fn assimilate_storage(&self, _storage: &mut Storage) -> Result<(), String> {
		todo!()
	}
}
