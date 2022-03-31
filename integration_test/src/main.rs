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

use centrifuge_runtime::{Block as PBlock, RuntimeApi as PRtApi};
use fudge::{
	digest::DigestCreator,
	inherent::{
		CreateInherentDataProviders, FudgeDummyInherentRelayParachain, FudgeInherentParaParachain,
		FudgeInherentTimestamp,
	},
	ParachainBuilder, RelaychainBuilder,
};
use polkadot_core_primitives::{Block as RBlock, Header as RHeader};
use polkadot_runtime::{Runtime as RRuntime, RuntimeApi as RRtApi};

type RCidp = Box<
	dyn CreateInherentDataProviders<
		RBlock,
		(),
		InherentDataProviders = (
			FudgeInherentTimestamp,
			sp_consensus_babe::inherents::InherentDataProvider,
			sp_authorship::InherentDataProvider<RHeader>,
			FudgeDummyInherentRelayParachain<RHeader>,
		),
	>,
>;
type PCidp = Box<
	dyn CreateInherentDataProviders<
		PBlock,
		(),
		InherentDataProviders = (
			FudgeInherentTimestamp,
			sp_consensus_babe::inherents::InherentDataProvider,
			FudgeInherentParaParachain,
		),
	>,
>;
type Dp = Box<dyn DigestCreator + Send + Sync>;

fn main() {}

const PARA_ID: u32 = 2002u32;
#[fudge::companion]
struct TestEnv {
	#[fudge::parachain(PARA_ID)]
	centrifuge: ParachainBuilder<PBlock, PRtApi, PCidp, Dp>,
	#[fudge::parachain(2000u32)]
	sibling: ParachainBuilder<PBlock, PRtApi, PCidp, Dp>,
	#[fudge::relaychain]
	polkadot: RelaychainBuilder<RBlock, RRtApi, RRuntime, RCidp, Dp>,
}
