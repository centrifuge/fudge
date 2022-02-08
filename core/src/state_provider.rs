use std::marker::PhantomData;
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
use sc_client_api::Backend;
use sc_client_db::{DatabaseSettings, DatabaseSource, KeepBlocks, TransactionStorageMode};
use sc_service::PruningMode;
use sp_api::BlockId;
use sp_core::sp_std::sync::Arc;
use sp_database::{Database, MemDb};
use sp_runtime::traits::Block as BlockT;
use sp_state_machine::MemoryDB;

pub struct StateProvider<B, Block> {
	backend: B,
	_phantom: PhantomData<Block>,
}

impl<B, Block> StateProvider<B, Block>
where
	Block: BlockT,
	B: Backend<Block>,
{
	pub fn from_genesis() -> Self {
		todo!();
	}

	pub fn from_db() -> Self {
		todo!()
	}

	pub fn from_spec() -> Self {
		todo!()
	}
}

impl<Block> StateProvider<sc_client_db::Backend<Block>, Block>
where
	Block: BlockT,
{
	pub fn empty(code: Option<&[u8]>) -> Self {
		let mut provider = StateProvider::default();
		let state = provider.backend.state_at(BlockId::Hash(Default::default()));
		todo!();
		provider
	}
}

impl<Block> Default for StateProvider<sc_client_db::Backend<Block>, Block>
where
	Block: BlockT,
{
	fn default() -> Self {
		// TODO: Maybe allow to set these settings
		let settings = DatabaseSettings {
			state_cache_size: 0,
			state_cache_child_ratio: None,
			state_pruning: PruningMode::ArchiveAll,
			source: DatabaseSource::Custom(Arc::new(MemDb::new())),
			keep_blocks: KeepBlocks::All,
			transaction_storage: TransactionStorageMode::BlockBody,
		};

		// TODO: What is the right canoncicalizatio_delay here
		// TOOD: Unwrap here safely somehow? Mabye default is not the right impl for this. Haha.
		let backend = sc_client_db::Backend::new(settings, 0).unwrap();

		Self { backend, _phantom: Default::default() }
	}
}
