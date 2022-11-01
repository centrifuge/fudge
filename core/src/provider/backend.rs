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

pub struct MemDb;

impl StateProvider {
	pub fn from_db(open: DbOpen) -> Self {
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

	fn with_in_mem_db() -> Result<Self, ()> {
		let settings = DatabaseSettings {
			trie_cache_maximum_size: None,
			state_pruning: Some(PruningMode::ArchiveAll),
			source: DatabaseSource::Custom {
				db: Arc::new(MemDb::new()),
				require_create_flag: true,
			},
			blocks_pruning: BlocksPruning::All,
		};

		let backend =
			Arc::new(sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY).map_err(|_| ())?);

		Ok(Self {
			pseudo_genesis: Storage::default(),
		})
	}
}
