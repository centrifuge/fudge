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

//! The module provides a ways and means to interact with
//! and provide externalities.

use std::{
	any::Any,
	panic::{AssertUnwindSafe, UnwindSafe},
};

use sp_core::Hasher;
use sp_externalities::Externalities;
use sp_state_machine::{Backend, Ext, OverlayedChanges, StorageChanges};
use sp_storage::StateVersion;
use thiserror::Error;

const DEFAULT_EXTERNALITIES_PROVIDER_LOG_TARGET: &str = "fudge-externalities";

#[derive(Error, Debug)]
pub enum Error {
	#[error("storage transaction commit error")]
	StorageTransactionCommit,

	#[error("storage changes draining: {0}")]
	StorageChangesDraining(Box<dyn std::error::Error>),

	#[error("externalities set and run: {0:?}")]
	ExternalitiesSetAndRun(Box<dyn Any + Send + 'static>),
}

/// Provides a simple and secure way to execute code
/// in an externalities provided environment.
///
/// The struct solely needs something that implements `trait sp_state_machine::Backend`.
/// From there on
pub struct ExternalitiesProvider<'a, H, B>
where
	H: Hasher,
	H::Out: codec::Codec + Ord + 'static,
	B: Backend<H>,
{
	overlay: OverlayedChanges<H>,
	backend: &'a B,
}

impl<'a, H, B> ExternalitiesProvider<'a, H, B>
where
	H: Hasher,
	H::Out: codec::Codec + Ord + 'static,
	B: Backend<H>,
{
	/// Create a new `ExternalitiesProvider`.
	pub fn new(backend: &'a B) -> Self {
		Self {
			backend,
			overlay: OverlayedChanges::default(),
		}
	}

	/// Get externalities implementation.
	pub fn ext(&mut self) -> Ext<H, B> {
		Ext::new(
			&mut self.overlay,
			&self.backend,
			None,
		)
	}

	/*
	/// Create a new instance_id of `TestExternalities` with storage.
	pub fn new(storage: Storage) -> Self {
		Self::new_with_code(&[], storage)
	}

	/// New empty test externalities.
	pub fn new_empty() -> Self {
		Self::new_with_code(&[], Storage::default())
	}

	/// Create a new instance_id of `TestExternalities` with code and storage.
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

	/// Registers the given extension for this instance_id.
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

	pub fn execute_with_mut<R>(
		&mut self,
		execute: impl FnOnce() -> R,
		version: StateVersion,
	) -> Result<(R, StorageChanges<H>), Error> {
		let _parent_hash = self.overlay.storage_root(
			self.backend,
			version,
		);

		let mut ext = self.ext();
		ext.storage_start_transaction();
		let r = sp_externalities::set_and_run_with_externalities(&mut ext, execute);

		ext.storage_commit_transaction().map_err(|_| {
			tracing::error!(
				target = DEFAULT_EXTERNALITIES_PROVIDER_LOG_TARGET,
				"Could not commit storage transaction."
			);

			Error::StorageTransactionCommit
		})?;

		Ok((
			r,
			self.overlay
				.drain_storage_changes::<B>(
					self.backend,
					version,
				)
				.map_err(|e| {
					tracing::error!(
						target = DEFAULT_EXTERNALITIES_PROVIDER_LOG_TARGET,
						error = ?e,
						"Could not drain storage changes."
					);

					Error::StorageChangesDraining(Box::<dyn std::error::Error>::from(e))
				})?,
		))
	}

	/// Execute the given closure while `self` is set as externalities.
	///
	/// Returns the result of the given closure, if no panics occured.
	/// Otherwise, returns `Err`.
	#[allow(dead_code)]
	pub fn execute_with_safe<R>(&mut self, f: impl FnOnce() -> R + UnwindSafe) -> Result<R, Error> {
		let mut ext = AssertUnwindSafe(self.ext());
		std::panic::catch_unwind(move || {
			sp_externalities::set_and_run_with_externalities(&mut *ext, f)
		})
		.map_err(|e| {
			tracing::error!(
				target = DEFAULT_EXTERNALITIES_PROVIDER_LOG_TARGET,
				error = ?e,
				"Could not set and run externalities."
			);

			Error::ExternalitiesSetAndRun(e)
		})
	}
}
