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

pub use constants::*;
pub use types::*;

/// Common types for all runtimes
pub mod types {
	use frame_support::traits::EitherOfDiverse;
	use frame_system::EnsureRoot;
	use sp_runtime::{
		traits::{BlakeTwo256, IdentifyAccount, Verify},
		OpaqueExtrinsic,
	};

	// Ensure that origin is either Root or fallback to use EnsureOrigin `O`
	pub type EnsureRootOr<O> = EitherOfDiverse<EnsureRoot<AccountId>, O>;

	/// An index to a block.
	pub type BlockNumber = u32;

	/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
	pub type Signature = sp_runtime::MultiSignature;

	/// Some way of identifying an account on the chain. We intentionally make it equivalent
	/// to the public key of our transaction signing scheme.
	pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

	/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
	/// never know...
	pub type AccountIndex = u32;

	/// The address format for describing accounts.
	pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

	/// Balance of an account.
	pub type Balance = u128;

	/// IBalance is the signed version of the Balance for orml tokens
	pub type IBalance = i128;

	/// Index of a transaction in the chain.
	pub type Index = u32;

	/// A hash of some data used by the chain.
	pub type Hash = sp_core::H256;

	/// A generic block for the node to use, as we can not commit to
	/// a specific Extrinsic format at this point. Runtimes will ensure
	/// Extrinsic are correctly decoded.
	pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;

	/// Block header type as expected by this runtime.
	pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;

	/// Aura consensus authority.
	pub type AuraId = sp_consensus_aura::sr25519::AuthorityId;

	/// Moment type
	pub type Moment = u64;
}

/// Common constants for all runtimes
pub mod constants {
	use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};
	use sp_runtime::Perbill;

	use super::types::{Balance, BlockNumber};

	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pallet_timestamp` which is in turn picked up by `pallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLISECS_PER_BLOCK: u64 = 12000;
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	// Time is measured by number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	/// Milliseconds per day
	pub const MILLISECS_PER_DAY: u64 = SECONDS_PER_DAY * 1000;

	// Seconds units
	pub const SECONDS_PER_MINUTE: u64 = 60;
	pub const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * 60;
	pub const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * 24;
	pub const SECONDS_PER_YEAR: u64 = SECONDS_PER_DAY * 365;

	/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
	/// used to limit the maximal weight of a single extrinsic.
	pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
	/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
	/// Operational  extrinsics.
	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

	/// We allow for 0.5 seconds of compute with a 6 second average block time.
	pub const MAXIMUM_BLOCK_WEIGHT: Weight =
		Weight::from_ref_time(WEIGHT_REF_TIME_PER_SECOND).saturating_div(2);

	pub const MICRO_CUR: Balance = 1_000_000_000_000; // 10−6 	0.000001
	pub const MILLI_CUR: Balance = 1_000 * MICRO_CUR; // 10−3 	0.001
	pub const CENTI_CUR: Balance = 10 * MILLI_CUR; // 10−2 	0.01
	pub const CUR: Balance = 100 * CENTI_CUR;
}
