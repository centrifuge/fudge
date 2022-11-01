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

//! A module concerned with providing a data structure that can
//! be used to provide a genesis state for a builder.

use std::path::PathBuf;

use sc_client_api::Backend;
use sc_client_db::{BlocksPruning, DatabaseSettings, DatabaseSource};
use sc_service::PruningMode;
use sp_core::storage::well_known_keys::CODE;
use sp_database::MemDb;
use sp_runtime::{traits::Block as BlockT, BuildStorage};
use sp_std::{marker::PhantomData, sync::Arc};
use sp_storage::Storage;

/// Helping struct to ease provisioning of a
/// genesis state.
pub struct StateProvider {
	pseudo_genesis: Storage,
}

impl StateProvider {
	/// Anything that implements `trait BuildStorage` can appends its state to the
	/// existing `StateProvider`.
	///
	/// **panics+* upon providing wrongly formatted child storage items.
	pub fn insert_storage(&mut self, storage: impl BuildStorage) -> &mut Self {
		assert!(storage.assimilate_storage(&mut self.pseudo_genesis).is_ok());
		self
	}

	/// Generates a truly empty `StateProvider`
	pub fn empty() -> Self {
		Self {
			pseudo_genesis: Storage::default(),
		}
	}

	/// Builds a `StorageProvider` from anything that implements
	/// `trait BuildStorage`
	///
	/// E.g.: This could be used to a spec of a chain to build the
	///       correct genesis state for a chain. All specs implement
	///       `trait BuildStorage`
	/// (See: https://github.com/paritytech/substrate/blob/0d64ba4268106fffe430d41b541c1aeedd4f8da5/client/chain-spec/src/chain_spec.rs#L111-L141)
	pub fn from_storage(storage: impl BuildStorage) -> Self {
		let mut provider = StateProvider::empty();
		provider.insert_storage(storage);
		provider
	}

	/// Creates a new instance of `StorageBuilder`.
	///
	/// As instantiating an actual client needs in most cases
	/// an existing wasm blob to be present, this method enforces
	/// providing the code.
	///
	/// Developers opting out of this should use `StateProvider::empty()`.
	pub fn new(code: &[u8]) -> Self {
		let mut storage = Storage::default();
		storage.top.insert(CODE.to_vec(), code.to_vec());

		let mut provider = StateProvider::empty();
		provider.insert_storage(storage);
		provider
	}
}

impl BuildStorage for StateProvider {
	fn build_storage(&self) -> Result<Storage, String> {
		Ok(self.pseudo_genesis.clone())
	}

	fn assimilate_storage(&self, storage: &mut Storage) -> Result<(), String> {
		self.pseudo_genesis.assimilate_storage(storage)
	}
}
