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
	prelude::{AccountId32, Parachain, XcmError},
	v4::{
		Asset, AssetId, Instruction, InteriorLocation, Junctions::X1, Location, NetworkId,
		XcmContext,
	},
};
use codec::{Decode, Encode};
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::traits::TransformOrigin;
use frame_support::traits::{Nothing, ProcessMessageError};
use pallet_xcm::TestWeightInfo;
use polkadot_parachain_primitives::primitives::Sibling;
use scale_info::TypeInfo;
use sp_core::Get;
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;
use xcm_builder::{
	AccountId32Aliases, EnsureXcmOrigin, FixedWeightBounds, FrameTransactionalProcessor,
	ParentIsPreset, SiblingParachainConvertsVia, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::{
	traits::{Properties, ShouldExecute, TransactAsset, WeightTrader},
	AssetsInHolding,
};
use xcm_primitives::{UtilityAvailableCalls, UtilityEncodeCall, XcmTransact};

use super::*;

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfLocation: Location = Location::new(1, X1([Parachain(ParachainInfo::get().into())].into()));

	pub UniversalLocation: InteriorLocation = [Parachain(ParachainInfo::parachain_id().into())].into();

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
	type TransactionalProcessor = FrameTransactionalProcessor;
	type UniversalAliases = Nothing;
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = XcmRouter;
}

pub struct TestBarrier;

impl ShouldExecute for TestBarrier {
	fn should_execute<RuntimeCall>(
		_origin: &Location,
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

parameter_types! {
	pub const RelayChain: Location = Location::parent();

	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(RelayChain::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: Balance = 300_000_000;
	/// The fee per byte
	pub const ByteFee: Balance = 1_000_000;
}

pub type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	ByteFee,
	XcmpQueue,
>;

parameter_types! {
	pub const HeapSize: u32 = 1024;
	pub const MaxStale: u32 = 2;
	pub const ServiceWeight: Option<Weight> = Some(Weight::from_parts(100, 100));
}

pub struct MessageQueueWeightInfo;

// As of this version, the current implementation of WeightInfo for (), has weights that are bigger than 0.
impl pallet_message_queue::WeightInfo for MessageQueueWeightInfo {
	fn ready_ring_knit() -> Weight {
		Weight::zero()
	}

	fn ready_ring_unknit() -> Weight {
		Weight::zero()
	}

	fn service_queue_base() -> Weight {
		Weight::zero()
	}

	fn service_page_base_completion() -> Weight {
		Weight::zero()
	}

	fn service_page_base_no_completion() -> Weight {
		Weight::zero()
	}

	fn service_page_item() -> Weight {
		Weight::zero()
	}

	fn bump_service_head() -> Weight {
		Weight::zero()
	}

	fn reap_page() -> Weight {
		Weight::zero()
	}

	fn execute_overweight_page_removed() -> Weight {
		Weight::zero()
	}

	fn execute_overweight_page_updated() -> Weight {
		Weight::zero()
	}
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = MessageQueueWeightInfo;
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	type QueueChangeHandler = ();
	type QueuePausedQuery = ();
	type HeapSize = HeapSize;
	type MaxStale = MaxStale;
	type ServiceWeight = ServiceWeight;
}

pub struct ParaIdToSibling;
impl sp_runtime::traits::Convert<ParaId, AggregateMessageOrigin> for ParaIdToSibling {
	fn convert(para_id: ParaId) -> AggregateMessageOrigin {
		AggregateMessageOrigin::Sibling(para_id)
	}
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = ();
	type MaxInboundSuspended = ();
	type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
	type RuntimeEvent = RuntimeEvent;
	type VersionWrapper = PolkadotXcm;
	type WeightInfo = ();
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
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
	pub MaxFee: Asset = (Location::parent(), 1_000_000_000_000u128).into();
}
pub type MaxHrmpRelayFee = xcm_builder::Case<MaxFee>;

impl pallet_xcm_transactor::Config for Runtime {
	type AccountIdToLocation = AccountIdToMultiLocation<AccountId>;
	type AssetTransactor = DummyAssetTransactor;
	type Balance = Balance;
	type BaseXcmWeight = BaseXcmWeight;
	type CurrencyId = AssetId;
	type CurrencyIdToLocation = CurrencyIdConvert;
	type DerivativeAddressRegistrationOrigin = EnsureRoot<AccountId>;
	type HrmpManipulatorOrigin = EnsureRoot<AccountId>;
	type HrmpOpenOrigin = EnsureRoot<AccountId>;
	type MaxHrmpFee = MaxHrmpRelayFee;
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
		_what: &Asset,
		_who: &Location,
		_maybe_context: Option<&XcmContext>,
	) -> Result<AssetsInHolding, XcmError> {
		Ok(AssetsInHolding::default())
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
		_payment: AssetsInHolding,
		_context: &XcmContext,
	) -> Result<AssetsInHolding, XcmError> {
		Ok(AssetsInHolding::default())
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
	fn destination(self) -> Location {
		Default::default()
	}
}

pub struct AccountIdToMultiLocation<AccountId>(PhantomData<AccountId>);
impl<AccountId> Convert<AccountId, Location> for AccountIdToMultiLocation<AccountId>
where
	AccountId: Into<[u8; 32]> + Clone,
{
	fn convert(account: AccountId) -> Location {
		X1([AccountId32 {
			network: None,
			id: account.into(),
		}]
		.into())
		.into()
	}
}

pub struct CurrencyIdConvert;

impl Convert<AssetId, Option<Location>> for CurrencyIdConvert {
	fn convert(_id: AssetId) -> Option<Location> {
		Some(Location::here())
	}
}

pub type XcmWeigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
