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

use pallet_babe;
use sp_consensus_babe::digests::{PreDigest, SecondaryPlainPreDigest};
pub struct Digest;

impl Digest {
	pub fn pre_digest<Runtime: pallet_babe::Config>() -> PreDigest {
		let mut slot = pallet_babe::Pallet::<Runtime>::current_slot();
		slot = slot + 1;

		PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
			authority_index: 0,
			slot,
		})
	}
}
