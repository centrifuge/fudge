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

use sc_client_api::Backend;
use sc_client_db::{DatabaseSettings, DatabaseSource, KeepBlocks};
use sc_service::PruningMode;
use sp_core::storage::well_known_keys::CODE;
use sp_database::MemDb;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::BuildStorage;
use sp_std::{marker::PhantomData, sync::Arc};
use sp_storage::Storage;
use std::path::PathBuf;

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
	pub fn insert_storage(&mut self, storage: Storage) -> &mut Self {
		let Storage {
			top,
			children_default: _children_default,
		} = storage;

		self.pseudo_genesis.top.extend(top.into_iter());
		// TODO: Not sure how to handle childrens here?
		// self.pseudo_genesis.children_default.
		self
	}

	pub fn backend(&self) -> Arc<B> {
		self.backend.clone()
	}
}

impl<Block> StateProvider<sc_client_db::Backend<Block>, Block>
where
	Block: BlockT,
{
	/// Note: When using this option, it will only be
	pub fn from_db(path: PathBuf) -> Self {
		// TODO: Maybe allow to set these settings
		let settings = DatabaseSettings {
			state_cache_size: 0,
			state_cache_child_ratio: None,
			state_pruning: Some(PruningMode::ArchiveAll),
			source: DatabaseSource::RocksDb {
				path: path.clone(),
				cache_size: 0,
			},
			keep_blocks: KeepBlocks::All,
			// transaction_storage: TransactionStorageMode::BlockBody,
		};

		let backend = Arc::new(
			sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY)
				.map_err(|_| ())
				.unwrap(),
		);

		Self {
			backend,
			pseudo_genesis: Storage::default(),
			_phantom: Default::default(),
		}
	}

	pub fn from_spec() -> Self {
		todo!()
	}

	pub fn from_storage(storage: Storage) -> Self {
		let mut provider = StateProvider::empty_default(None);
		provider.insert_storage(storage);
		provider
	}

	pub fn empty_default(code: Option<&[u8]>) -> Self {
		// TODO: Handle unwrap
		let mut provider = StateProvider::with_in_mem_db().unwrap();

		let mut storage = Storage::default();
		if let Some(code) = code {
			storage.top.insert(CODE.to_vec(), code.to_vec());
		}

		provider.insert_storage(storage);
		provider
	}

	fn with_in_mem_db() -> Result<Self, ()> {
		// TODO: Maybe allow to set these settings
		let settings = DatabaseSettings {
			state_cache_size: 0,
			state_cache_child_ratio: None,
			state_pruning: Some(PruningMode::ArchiveAll),
			source: DatabaseSource::Custom {
				db: Arc::new(MemDb::new()),
				require_create_flag: false,
			},
			keep_blocks: KeepBlocks::All,
			// transaction_storage: TransactionStorageMode::BlockBody,
		};

		let backend =
			Arc::new(sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY).map_err(|_| ())?);
		//TODO(nuno): ^ this here is failing and the error is being silenced

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
