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

use sp_api::{BlockT, NumberFor};
use sp_core::Hasher;
use sp_runtime::traits::Header;
use sp_state_machine::{Backend, ChangesTrieBlockNumber, Ext, InMemoryChangesTrieStorage, OverlayedChanges, StorageChanges, StorageTransactionCache};
use std::panic::{AssertUnwindSafe, UnwindSafe};
use sp_externalities::Externalities;

pub struct ExternalitiesProvider<'a, H, Block, B>
where
	H: Hasher,
	H::Out: codec::Codec + Ord + 'static,
	B: Backend<H>,
	Block: BlockT,
{
	overlay: OverlayedChanges,
	// TODO: Do we need an offchain-db here?
	//offchain_db: TestPersistentOffchainDB,
	storage_transaction_cache:
		StorageTransactionCache<<B as Backend<H>>::Transaction, H, NumberFor<Block>>,
	backend: &'a B,
}

impl<'a, H, Block, B> ExternalitiesProvider<'a, H, Block, B>
where
	H: Hasher,
	H::Out: codec::Codec + Ord + 'static,
	B: Backend<H>,
	Block: BlockT,
{
	pub fn new(backend: &'a B) -> Self {
		Self {
			backend,
			storage_transaction_cache: StorageTransactionCache::default(),
			overlay: OverlayedChanges::default(),
		}
	}

	/// Get externalities implementation.
	pub fn ext(&mut self) -> Ext<H, NumberFor<Block>, B> {
		Ext::new(&mut self.overlay, &mut self.storage_transaction_cache, &self.backend, None, None)
	}
	/*
	/// Create a new instance of `TestExternalities` with storage.
	pub fn new(storage: Storage) -> Self {
		Self::new_with_code(&[], storage)
	}

	/// New empty test externalities.
	pub fn new_empty() -> Self {
		Self::new_with_code(&[], Storage::default())
	}

	/// Create a new instance of `TestExternalities` with code and storage.
	pub fn new_with_code(code: &[u8], mut storage: Storage) -> Self {
		let mut overlay = OverlayedChanges::default();
		let changes_trie_config = storage
			.top
			.get(CHANGES_TRIE_CONFIG)
			.and_then(|v| Decode::decode(&mut &v[..]).ok());
		overlay.set_collect_extrinsics(changes_trie_config.is_some());

		assert!(storage.top.keys().all(|key| !is_child_storage_key(key)));
		assert!(storage.children_default.keys().all(|key| is_child_storage_key(key)));

		storage.top.insert(CODE.to_vec(), code.to_vec());

		let mut extensions = Extensions::default();
		extensions.register(TaskExecutorExt::new(TaskExecutor::new()));

		let offchain_db = TestPersistentOffchainDB::new();

		TestExternalities {
			overlay,
			offchain_db,
			changes_trie_config,
			extensions,
			changes_trie_storage: ChangesTrieInMemoryStorage::new(),
			backend: storage.into(),
			storage_transaction_cache: Default::default(),
		}
	}

	/// Returns the overlayed changes.
	pub fn overlayed_changes(&self) -> &OverlayedChanges {
		&self.overlay
	}

	/// Move offchain changes from overlay to the persistent store.
	pub fn persist_offchain_overlay(&mut self) {
		self.offchain_db.apply_offchain_changes(self.overlay.offchain_drain_committed());
	}

	/// A shared reference type around the offchain worker storage.
	pub fn offchain_db(&self) -> TestPersistentOffchainDB {
		self.offchain_db.clone()
	}

	/// Insert key/value into backend
	pub fn insert(&mut self, k: StorageKey, v: StorageValue) {
		self.backend.insert(vec![(None, vec![(k, Some(v))])]);
	}

	/// Registers the given extension for this instance.
	pub fn register_extension<E: Any + Extension>(&mut self, ext: E) {
		self.extensions.register(ext);
	}

	/// Get mutable reference to changes trie storage.
	pub fn changes_trie_storage(&mut self) -> &mut ChangesTrieInMemoryStorage<H, N> {
		&mut self.changes_trie_storage
	}

	/// Return a new backend with all pending changes.
	///
	/// In contrast to [`commit_all`](Self::commit_all) this will not panic if there are open
	/// transactions.
	pub fn as_backend(&self) -> B {
		let top: Vec<_> =
			self.overlay.changes().map(|(k, v)| (k.clone(), v.value().cloned())).collect();
		let mut transaction = vec![(None, top)];

		for (child_changes, child_info) in self.overlay.children() {
			transaction.push((
				Some(child_info.clone()),
				child_changes.map(|(k, v)| (k.clone(), v.value().cloned())).collect(),
			))
		}

		self.backend.update(transaction)
	}
	*/

	/// Execute the given closure while `self` is set as externalities.
	///
	/// Returns the result of the given closure.
	pub fn execute_with<R>(&mut self, execute: impl FnOnce() -> R) -> R {
		let mut ext = self.ext();
		sp_externalities::set_and_run_with_externalities(&mut ext, execute)
	}

	pub fn execute_with_mut<R>(&mut self, execute: impl FnOnce() -> R) -> (R, StorageChanges<B::Transaction, H, NumberFor<Block>>) {
		let parent_hash = self.overlay.storage_root(self.backend, &mut self.storage_transaction_cache);

		let mut ext = self.ext();
		ext.storage_start_transaction();
		let r = sp_externalities::set_and_run_with_externalities(&mut ext, execute);
		// TODO: Handle unwrap
		ext.storage_commit_transaction().unwrap();

		(r, self.overlay.drain_storage_changes::<B, H, NumberFor<Block>>(self.backend, None, parent_hash, &mut self.storage_transaction_cache).unwrap())
	}

	/// Execute the given closure while `self` is set as externalities.
	///
	/// Returns the result of the given closure, if no panics occured.
	/// Otherwise, returns `Err`.
	pub fn execute_with_safe<R>(
		&mut self,
		f: impl FnOnce() -> R + UnwindSafe,
	) -> Result<R, String> {
		let mut ext = AssertUnwindSafe(self.ext());
		std::panic::catch_unwind(move || {
			sp_externalities::set_and_run_with_externalities(&mut *ext, f)
		})
		.map_err(|e| format!("Closure panicked: {:?}", e))
	}
}
