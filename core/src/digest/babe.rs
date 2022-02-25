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
use sp_timestamp::Timestamp;

pub struct Digest;

impl Digest {
	pub fn pre_digest(timestamp: Timestamp, slot_duration: u64) -> PreDigest {
		let slot_wrap =
			sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_duration(
				timestamp,
				std::time::Duration::from_secs(slot_duration),
			);

		PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
			authority_index: 0,
			slot: slot_wrap.slot(),
		})
	}
}
