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
use sc_client_api::{AuxStore, Backend};
use sc_client_db::{DatabaseSettings, DatabaseSource, KeepBlocks, TransactionStorageMode};
use sc_service::PruningMode;
use sp_api::BlockId;
use sp_database::{Database, MemDb};
use sp_runtime::traits::Block as BlockT;
use sp_storage::Storage;
use sp_std::{marker::PhantomData, sync::Arc};
use sp_core::storage::well_known_keys::CODE;
use sp_runtime::BuildStorage;

pub const CANONICALIZATION_DELAY: u64 = 4096;

pub struct StateProvider<B, Block> {
	backend: Arc<B>,
	_phantom: PhantomData<Block>,
}

impl<B, Block> StateProvider<B, Block>
where
	Block: BlockT,
	B: Backend<Block>,
{
	pub fn from_storage(storage: Storage) -> Self {
		todo!();
	}

	pub fn from_db() -> Self {
		todo!()
	}

	pub fn from_spec() -> Self {
		todo!()
	}

	pub fn insert_storage(&mut self, storage: Storage) -> &mut Self {
		// Bascially iterate over top and bottom and use aux storage. Not sure though, if the database takes this correctly
		todo!()
	}

	pub fn backend(&self) -> Arc<B> {
		self.backend.clone()
	}
}

impl<Block> StateProvider<sc_client_db::Backend<Block>, Block>
where
	Block: BlockT,
{
	pub fn empty_default(code: Option<&[u8]>) -> Self {
		// TODO: Handle unwrap
		let mut provider = StateProvider::with_InMemDb().unwrap();

		if let Some(code) = code {
			// TODO: Insert code here...
			//provider.backend.insert_aux(CODE, code);
		}

        /*
		let genesis_storage =
			build_genesis_storage.build_storage().map_err(sp_blockchain::Error::Storage)?;
		let genesis_state_version =
			Self::resolve_state_version_from_wasm(&genesis_storage, &executor)?;
		let mut op = backend.begin_operation()?;
		let state_root =
			op.set_genesis_state(genesis_storage, !config.no_genesis, genesis_state_version)?;
		let genesis_block = genesis::construct_genesis_block::<Block>(state_root.into());
		info!(
				"🔨 Initializing Genesis block/state (state: {}, header-hash: {})",
				genesis_block.header().state_root(),
				genesis_block.header().hash()
			);
		// Genesis may be written after some blocks have been imported and finalized.
		// So we only finalize it when the database is empty.
		let block_state = if info.best_hash == Default::default() {
			NewBlockState::Final
		} else {
			NewBlockState::Normal
		};
		op.set_block_data(
			genesis_block.deconstruct().0,
			Some(vec![]),
			None,
			None,
			block_state,
		)?;
		backend.commit_operation(op)?;
		*/

		provider
	}

	fn with_InMemDb() -> Result<Self, ()> {
		// TODO: Maybe allow to set these settings
		let settings = DatabaseSettings {
			state_cache_size: 0,
			state_cache_child_ratio: None,
			state_pruning: PruningMode::ArchiveAll,
			source: DatabaseSource::Custom(Arc::new(MemDb::new())),
			keep_blocks: KeepBlocks::All,
			transaction_storage: TransactionStorageMode::BlockBody,
		};

		let backend = Arc::new(sc_client_db::Backend::new(settings, CANONICALIZATION_DELAY).map_err(|_| ())?);

		Ok(Self { backend, _phantom: Default::default() })
	}
}

impl<B, Block> BuildStorage for StateProvider<B, Block>
where
	Block: BlockT,
	B: Backend<Block>,
{
	fn build_storage(&self) -> Result<Storage, String> {
		//TODO
		Ok(Storage::default())
	}

	fn assimilate_storage(&self, storage: &mut Storage) -> Result<(), String> {
		todo!()
	}
}