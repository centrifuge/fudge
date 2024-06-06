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

use std::{marker::PhantomData, path::PathBuf, sync::Arc};

use sc_client_db::{BlocksPruning, DatabaseSettings, DatabaseSource, PruningMode};
use sc_service::TFullBackend;
use sp_runtime::traits::Block as BlockT;

use crate::provider::BackendProvider;

/// The default canonicalization delay of the
/// backends that are instantiated here.
const CANONICALIZATION_DELAY: u64 = 4096;

/// The default cache size for a RocksDb
const ROCKS_DB_CACHE_SIZE: usize = 1024;

/// A struct holding all necessary information
/// to create a memory backend that implements
/// `sc_client_api::Backend`
pub struct MemDb<Block> {
	blocks_pruning: BlocksPruning,
	trie_cache: Option<usize>,
	state_pruning: Option<PruningMode>,
	require_create_flag: bool,
	canonicalization_delay: u64,
	_phantom: PhantomData<Block>,
}

impl<Block> MemDb<Block> {
	/// Creates a new `MemDb` with some sane
	/// defaults.
	///
	/// ```ignore
	/// Self {
	///       path,
	///       blocks_pruning: BlocksPruning::All,
	///       trie_cache: None,
	///       state_pruning: Some(PruningMode::ArchiveAll),
	///       require_create_flag: true,
	///       canonicalization_delay: 4096,
	///       _phantom: Default::default()
	/// }
	/// ```
	///
	/// Every default can be overwritten with the respective
	/// `with_*` method of the struct.
	pub fn new() -> Self {
		Self {
			blocks_pruning: BlocksPruning::KeepAll,
			trie_cache: None,
			state_pruning: Some(PruningMode::ArchiveAll),
			require_create_flag: true,
			canonicalization_delay: CANONICALIZATION_DELAY,
			_phantom: Default::default(),
		}
	}

	/// Overwrites current block pruning. The newly set `BlockPruning` will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_blocks_pruning(&mut self, pruning: BlocksPruning) -> &mut Self {
		self.blocks_pruning = pruning;
		self
	}

	/// Overwrites current trie cache. The newly set trie cache size will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_trie_cache(&mut self, trie_cache: usize) -> &mut Self {
		self.trie_cache = Some(trie_cache);
		self
	}

	/// Overwrites current trie cache to NONE. The newly set trie cache size will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn no_trie_cache(&mut self) -> &mut Self {
		self.trie_cache = None;
		self
	}

	/// Overwrites current state pruning mode. The newly set `PruningMode` will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_state_pruning(&mut self, pruning: PruningMode) -> &mut Self {
		self.state_pruning = Some(pruning);
		self
	}

	/// Overwrites current state pruning mode to NONE. The newly set `PruningMode` will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn no_state_pruning(&mut self) -> &mut Self {
		self.state_pruning = None;
		self
	}

	/// Overwrites current requiere create flag. The newly set flag will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_create_flag(&mut self, create: bool) -> &mut Self {
		self.require_create_flag = create;
		self
	}

	/// Overwrites current canonicalization delay. The newly set canonicalization delay will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_canonicalization_delay(&mut self, delay: u64) -> &mut Self {
		self.canonicalization_delay = delay;
		self
	}
}

impl<Block: BlockT> BackendProvider<Block> for MemDb<Block> {
	type Backend = TFullBackend<Block>;

	fn provide(&self) -> Result<Arc<Self::Backend>, sp_blockchain::Error> {
		let settings = DatabaseSettings {
			trie_cache_maximum_size: self.trie_cache,
			state_pruning: self.state_pruning.clone(),
			source: DatabaseSource::Custom {
				db: Arc::new(sp_database::MemDb::new()),
				require_create_flag: self.require_create_flag,
			},
			blocks_pruning: self.blocks_pruning,
		};
		sc_client_db::Backend::new(settings, self.canonicalization_delay)
			.map(|backend| Arc::new(backend))
	}
}

/// A struct holding all necessary information
/// to create a disk backend that implements
/// `sc_client_api::Backend`
pub struct DiskDb<Block> {
	path: PathBuf,
	blocks_pruning: BlocksPruning,
	trie_cache: Option<usize>,
	state_pruning: Option<PruningMode>,
	database_type: DiskDatabaseType,
	canonicalization_delay: u64,
	_phantom: PhantomData<Block>,
}

impl<Block> DiskDb<Block> {
	/// Creates a new `DiskDb` with some sane
	/// defaults.
	///
	/// ```ignore
	/// Self {
	///       path,
	///       blocks_pruning: BlocksPruning::All,
	///       trie_cache: None,
	///       state_pruning: Some(PruningMode::ArchiveAll),
	///       database_type: DiskDatabaseType::RocksDb {cache_size: 1024},
	///       canonicalization_delay: 4096,
	///       _phantom: Default::default()
	/// }
	/// ```
	///
	/// Every default can be overwritten with the respective
	/// `with_*` method of the struct.
	pub fn new(path: PathBuf) -> Self {
		Self {
			path,
			blocks_pruning: BlocksPruning::KeepAll,
			trie_cache: None,
			state_pruning: Some(PruningMode::ArchiveAll),
			database_type: DiskDatabaseType::RocksDb {
				cache_size: ROCKS_DB_CACHE_SIZE,
			},
			canonicalization_delay: CANONICALIZATION_DELAY,
			_phantom: Default::default(),
		}
	}

	/// Overwrites current block pruning. The newly set `BlockPruning` will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_blocks_pruning(&mut self, pruning: BlocksPruning) -> &mut Self {
		self.blocks_pruning = pruning;
		self
	}

	/// Overwrites current trie cache. The newly set trie cache size will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_trie_cache(&mut self, trie_cache: usize) -> &mut Self {
		self.trie_cache = Some(trie_cache);
		self
	}

	/// Overwrites current trie cache to NONE. The newly set trie cache size will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn no_trie_cache(&mut self) -> &mut Self {
		self.trie_cache = None;
		self
	}

	/// Overwrites current state pruning mode. The newly set `PruningMode` will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_state_pruning(&mut self, pruning: PruningMode) -> &mut Self {
		self.state_pruning = Some(pruning);
		self
	}

	/// Overwrites current state pruning mode to NONE. The newly set `PruningMode` will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn no_state_pruning(&mut self) -> &mut Self {
		self.state_pruning = None;
		self
	}

	/// Overwrites current database type. The newly set `DiskDatabaseType` will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_database_type(&mut self, database_type: DiskDatabaseType) -> &mut Self {
		self.database_type = database_type;
		self
	}

	/// Overwrites current canonicalization delay. The newly set canonicalization delay will be used
	/// when a backend is created via `BackendProvider::provide(&self)`
	pub fn with_canonicalization_delay(&mut self, delay: u64) -> &mut Self {
		self.canonicalization_delay = delay;
		self
	}
}

impl<Block: BlockT> BackendProvider<Block> for DiskDb<Block> {
	type Backend = TFullBackend<Block>;

	fn provide(&self) -> Result<Arc<Self::Backend>, sp_blockchain::Error> {
		let settings = DatabaseSettings {
			trie_cache_maximum_size: self.trie_cache,
			state_pruning: self.state_pruning.clone(),
			source: match self.database_type {
				DiskDatabaseType::RocksDb { cache_size } => DatabaseSource::RocksDb {
					path: self.path.clone(),
					cache_size,
				},
				DiskDatabaseType::ParityDb => DatabaseSource::ParityDb {
					path: self.path.clone(),
				},
			},
			blocks_pruning: self.blocks_pruning,
		};
		sc_client_db::Backend::new(settings, self.canonicalization_delay)
			.map(|backend| Arc::new(backend))
	}
}

/// Enum indicating which kind of disk
/// database should be used.
#[derive(Copy, Clone, Debug)]
pub enum DiskDatabaseType {
	RocksDb { cache_size: usize },
	ParityDb,
}
