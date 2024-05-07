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
	prelude::{AccountId32, XcmError, X1},
	v3::{AssetId, Instruction, MultiAsset, MultiLocation, NetworkId, XcmContext},
};
use codec::{Decode, Encode};
use frame_support::traits::{Nothing, ProcessMessageError};
use pallet_xcm::TestWeightInfo;
use polkadot_parachain_primitives::primitives::Sibling;
use scale_info::TypeInfo;
use sp_core::{ConstU32, Get};
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;
use xcm_builder::{
	AccountId32Aliases, EnsureXcmOrigin, FixedWeightBounds, ParentIsPreset,
	SiblingParachainConvertsVia, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::{
	traits::{Properties, ShouldExecute, TransactAsset, WeightTrader},
	Assets,
};
use xcm_primitives::{UtilityAvailableCalls, UtilityEncodeCall, XcmTransact};

use super::*;

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfLocation: MultiLocation = MultiLocation::new(1, ::xcm::v3::Junctions::X1(::xcm::v3::Junction::Parachain(ParachainInfo::get().into())));

	pub UniversalLocation: ::xcm::v3::InteriorMultiLocation = ::xcm::v3::Junctions::X2(
		::xcm::v3::Junction::GlobalConsensus(RelayNetwork::get()),
		::xcm::v3::Junction::Parachain(ParachainInfo::parachain_id().into())
	);

	pub const UnitWeightCost: ::xcm::v3::Weight = ::xcm::v3::Weight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Aliasers = ();
	type AssetClaims = PolkadotXcm;
	type AssetExchanger = ();
	type AssetLocker = ();
	type AssetTransactor = DummyAssetTransactor;
	type AssetTrap = PolkadotXcm;
	type Barrier = TestBarrier;
	type CallDispatcher = RuntimeCall;
	type FeeManager = ();
	type IsReserve = ();
	type IsTeleporter = ();
	type MaxAssetsIntoHolding = ConstU32<64>;
	type MessageExporter = ();
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type PalletInstancesInfo = crate::AllPalletsWithSystem;
	type ResponseHandler = PolkadotXcm;
	type RuntimeCall = RuntimeCall;
	type SafeCallFilter = Everything;
	type SubscriptionService = PolkadotXcm;
	type Trader = DummyWeightTrader;
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
		_properties: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		Ok(())
	}
}

pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
);

pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// If we receive a MultiLocation of type AccountId32 that is within Centrifuge,
	// just alias it to a local [AccountId].
	AccountId32Aliases<RelayNetwork, AccountId>,
);

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
	type AdminOrigin = EnsureRoot<AccountId>;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<8>;
	type RemoteLockConsumerIdentifier = ();
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

parameter_types! {
	pub const BaseXcmWeight: ::xcm::v3::Weight = ::xcm::v3::Weight::from_parts(100_000_000, 0);
	pub MaxHrmpRelayFee: ::xcm::v3::MultiAsset = (MultiLocation::parent(), 1_000_000_000_000u128).into();
}

impl pallet_xcm_transactor::Config for Runtime {
	type AccountIdToMultiLocation = AccountIdToMultiLocation<AccountId>;
	type AssetTransactor = DummyAssetTransactor;
	type Balance = Balance;
	type BaseXcmWeight = BaseXcmWeight;
	type CurrencyId = AssetId;
	type CurrencyIdToMultiLocation = CurrencyIdConvert;
	type DerivativeAddressRegistrationOrigin = EnsureRoot<AccountId>;
	type HrmpManipulatorOrigin = EnsureRoot<AccountId>;
	type MaxHrmpFee = xcm_builder::Case<MaxHrmpRelayFee>;
	type ReserveProvider = xcm_primitives::AbsoluteAndRelativeReserve<SelfLocation>;
	type RuntimeEvent = RuntimeEvent;
	type SelfLocation = SelfLocation;
	type SovereignAccountDispatcherOrigin = EnsureRoot<AccountId>;
	type Transactor = NullTransactor;
	type UniversalLocation = UniversalLocation;
	type Weigher = XcmWeigher;
	type WeightInfo = ();
	type XcmSender = XcmRouter;
}

pub struct DummyAssetTransactor;

impl TransactAsset for DummyAssetTransactor {
	fn withdraw_asset(
		_what: &MultiAsset,
		_who: &MultiLocation,
		_maybe_context: Option<&XcmContext>,
	) -> Result<Assets, XcmError> {
		Ok(Assets::default())
	}
}

pub struct DummyWeightTrader;
impl WeightTrader for DummyWeightTrader {
	fn new() -> Self {
		DummyWeightTrader
	}

	fn buy_weight(
		&mut self,
		_weight: Weight,
		_payment: Assets,
		_context: &XcmContext,
	) -> Result<Assets, XcmError> {
		Ok(Assets::default())
	}
}

#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
pub struct NullTransactor {}

impl UtilityEncodeCall for NullTransactor {
	fn encode_call(self, _call: UtilityAvailableCalls) -> Vec<u8> {
		vec![]
	}
}

impl XcmTransact for NullTransactor {
	fn destination(self) -> MultiLocation {
		Default::default()
	}
}

pub struct AccountIdToMultiLocation<AccountId>(PhantomData<AccountId>);
impl<AccountId> Convert<AccountId, MultiLocation> for AccountIdToMultiLocation<AccountId>
where
	AccountId: Into<[u8; 32]> + Clone,
{
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 {
			network: None,
			id: account.into(),
		})
		.into()
	}
}

pub struct CurrencyIdConvert;

impl Convert<AssetId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(_id: AssetId) -> Option<MultiLocation> {
		Some(MultiLocation::here())
	}
}

pub type XcmWeigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
