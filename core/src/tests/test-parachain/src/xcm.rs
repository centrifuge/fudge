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

use ::xcm::{
	latest::{Instruction, MultiLocation},
	v3::NetworkId,
};
use frame_support::traits::Nothing;
use pallet_xcm::TestWeightInfo;
use sp_core::ConstU32;
use xcm_builder::{EnsureXcmOrigin, FixedWeightBounds, SignedToAccountId32};
use xcm_executor::traits::ShouldExecute;

use super::*;

/////////// XCM Config

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();

	pub UniversalLocation: ::xcm::v3::InteriorMultiLocation = ::xcm::v3::Junctions::X2(
		::xcm::v3::Junction::GlobalConsensus(RelayNetwork::get()),
		::xcm::v3::Junction::Parachain(ParachainInfo::parachain_id().into())
	);

	pub const UnitWeightCost: ::xcm::v3::Weight = ::xcm::v3::Weight::from_ref_time(200_000_000);
	pub const MaxInstructions: u32 = 100;
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetExchanger = ();
	type AssetLocker = ();
	type AssetTransactor = ();
	type AssetTrap = PolkadotXcm;
	type Barrier = TestBarrier;
	type CallDispatcher = RuntimeCall;
	type FeeManager = ();
	type IsReserve = ();
	type IsTeleporter = ();
	type MaxAssetsIntoHolding = ConstU32<64>;
	type MessageExporter = ();
	type OriginConverter = ();
	type PalletInstancesInfo = crate::AllPalletsWithSystem;
	type ResponseHandler = PolkadotXcm;
	type RuntimeCall = RuntimeCall;
	type SafeCallFilter = Everything;
	type SubscriptionService = PolkadotXcm;
	type Trader = ();
	type UniversalAliases = Nothing;
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = XcmRouter;
}

pub struct TestBarrier;

impl ShouldExecute for TestBarrier {
	fn should_execute<RuntimeCall>(
		_origin: &MultiLocation,
		_instructions: &mut [Instruction<RuntimeCall>],
		_max_weight: Weight,
		_weight_credit: &mut Weight,
	) -> Result<(), ()> {
		Ok(())
	}
}

pub type XcmRouter = (
	// Use UMP to communicate with the relay chain
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// Use XCMP to communicate with sibling parachains
	XcmpQueue,
);

/////////// Other pallets

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = ();
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type PriceForSiblingDelivery = ();
	type RuntimeEvent = RuntimeEvent;
	type VersionWrapper = PolkadotXcm;
	type WeightInfo = ();
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl pallet_xcm::Config for Runtime {
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = crate::Balances;
	type CurrencyMatcher = ();
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type MaxLockers = ConstU32<8>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type SovereignAccountOf = ();
	type TrustedLockers = ();
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type WeightInfo = TestWeightInfo;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Everything;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
