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


use crate::StoragePair;

///! myChain traits and possible default implementations

/// A trait that can be implemented by chains to provide their own authority that
/// they want to swap all the time when using mychain.
pub trait AuthorityProvider {
	fn block_production() -> Vec<StoragePair>;

	fn misc() -> Vec<StoragePair>;
}

pub struct DefaultAuthorityProvider;
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
