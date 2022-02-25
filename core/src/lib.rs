#![feature(associated_type_bounds)]
// TODO: Remove before release
#![allow(dead_code)]
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

// Library exports
pub use builder::{parachain::ParachainBuilder, stand_alone::StandAloneBuilder};
pub use provider::EnvProvider;
pub use types::{Bytes, StoragePair};

mod builder;
mod inherent;
mod provider;
#[cfg(test)]
mod tests;

mod types {
	pub type Bytes = Vec<u8>;

	#[derive(Clone, Debug)]
	pub struct StoragePair {
		key: Bytes,
		value: Option<Bytes>,
	}
}

pub mod traits {
	///! myChain traits and possible default implementations
	use super::StoragePair;

	/// A unification trait that must be super trait of all
	/// providers that inject keys, values
	///
	/// Traits using this as super trait should call their respective
	/// calls to provide key-values inside the `injections` call+
	// TODO: Implement for tuples.
	pub trait InjectionProvider {
		fn injections() -> Vec<StoragePair>;
	}

	/// A trait that can be implemented by chains to provide their own authority that
	/// they want to swap all the time when using mychain.
	pub trait AuthorityProvider: InjectionProvider {
		fn block_production() -> Vec<StoragePair>;

		fn misc() -> Vec<StoragePair>;
	}

	pub struct DefaultAuthorityProvider;

	impl InjectionProvider for DefaultAuthorityProvider {
		fn injections() -> Vec<StoragePair> {
			let mut pairs = DefaultAuthorityProvider::block_production();
			pairs.extend_from_slice(DefaultAuthorityProvider::misc().as_slice());
			pairs
		}
	}

	impl AuthorityProvider for DefaultAuthorityProvider {
		fn block_production() -> Vec<StoragePair> {
			todo!()

			// TODO: Implement default block prtoduction locations. Probably session + basic_authorship
			// storage and maybe the default `OnNewSession`
		}

		fn misc() -> Vec<StoragePair> {
			Vec::new()
		}
	}
}
