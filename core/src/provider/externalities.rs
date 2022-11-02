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

use std::panic::{AssertUnwindSafe, UnwindSafe};

use sp_core::Hasher;
use sp_externalities::Externalities;
use sp_state_machine::{Backend, Ext, OverlayedChanges, StorageChanges, StorageTransactionCache};
use sp_storage::StateVersion;

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
	overlay: OverlayedChanges,
	storage_transaction_cache: StorageTransactionCache<<B as Backend<H>>::Transaction, H>,
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
			storage_transaction_cache: StorageTransactionCache::default(),
			overlay: OverlayedChanges::default(),
		}
	}

	/// Get externalities implementation.
	pub fn ext(&mut self) -> Ext<H, B> {
		Ext::new(
			&mut self.overlay,
			&mut self.storage_transaction_cache,
			&self.backend,
			None,
		)
	}

	/// Drains the overlay changes into a `StorageChanges` struct. Leaving an empty overlay
	/// in place.
	///
	/// This can be used to retain changes that should be commited to an underlying database or
	/// to reset the overlay.
	pub fn drain(&mut self, state_version: StateVersion) -> StorageChanges<B::Transaction, H> {
		self.overlay
			.drain_storage_changes::<B, H>(
				self.backend,
				&mut self.storage_transaction_cache,
				state_version,
			)
			.expect("Drain  storage changes implementation does not return result but fails. Qed.")
	}

	/// Execute some code in an externalities provided environment.
	///
	/// Panics are NOT catched.
	pub fn execute_with<R>(&mut self, execute: impl FnOnce() -> R) -> R {
		let mut ext = self.ext();
		ext.storage_start_transaction();
		let r = sp_externalities::set_and_run_with_externalities(&mut ext, execute);
		ext.storage_commit_transaction().expect(
			"Started a transaction above. Runtime takes care of opening and closing transactions correctly too. Qed.",
		);
		r
	}

	/// Execute the given closure while `self` is set as externalities.
	///
	/// Returns the result of the given closure, if no panics occured.
	/// Otherwise, returns `Err`.
	pub fn execute_with_safe<R>(
		&mut self,
		execute: impl FnOnce() -> R + UnwindSafe,
	) -> Result<R, String> {
		let mut ext = AssertUnwindSafe(self.ext());
		std::panic::catch_unwind(move || {
			ext.storage_start_transaction();
			let r = sp_externalities::set_and_run_with_externalities(&mut *ext, execute);
			ext.storage_commit_transaction().expect(
				"Started a transaction above. Runtime takes care of opening and closing transactions correctly too. Qed.",
			);
			r
		})
		.map_err(|e| format!("Closure panicked: {:?}", e))
	}
}
