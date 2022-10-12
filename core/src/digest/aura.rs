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

use sc_client_api::{AuxStore, UsageProvider};
use sp_api::ProvideRuntimeApi;
use sp_consensus_aura::{
	digests::CompatibleDigestItem,
	sr25519::{AuthorityId, AuthoritySignature},
	AuraApi, Slot, SlotDuration,
};
use sp_inherents::InherentData;
use sp_runtime::{traits::Block, Digest as SPDigest, DigestItem};
use sp_std::marker::PhantomData;
use sp_timestamp::TimestampInherentData;

use crate::digest::DigestProvider;

pub struct Digest<B, C> {
	slot_duration: SlotDuration,
	_phantom: PhantomData<(B, C)>,
}

impl<B, C> Digest<B, C>
where
	B: Block,
	C: AuxStore + ProvideRuntimeApi<B> + UsageProvider<B>,
	C::Api: AuraApi<B, AuthorityId>,
{
	pub fn new(client: &C) -> Self {
		let slot_duration = sc_consensus_aura::slot_duration(client)
			.expect("slot_duration is always present; qed.");

		Self {
			slot_duration,
			_phantom: Default::default(),
		}
	}
}

impl<B, C> Digest<B, C>
where
	B: Block,
{
	fn digest(&self, inherents: &InherentData) -> Result<DigestItem, ()> {
		let timestamp = inherents
			.timestamp_inherent_data()
			.map_err(|_| ())?
			.expect("Timestamp is always present; qed");

		// we always calculate the new slot number based on the current time-stamp and the slot
		// duration.
		Ok(
			<DigestItem as CompatibleDigestItem<AuthoritySignature>>::aura_pre_digest(
				Slot::from_timestamp(timestamp, self.slot_duration),
			),
		)
	}
}

#[async_trait::async_trait]
impl<B, C> DigestProvider<B> for Digest<B, C>
where
	B: Block,
	C: std::marker::Sync,
{
	async fn build_digest(
		&self,
		_parent: &B::Header,
		inherents: &InherentData,
	) -> Result<SPDigest, ()> {
		Ok(SPDigest {
			logs: vec![self.digest(inherents)?],
		})
	}

	async fn append_digest(
		&self,
		digest: &mut SPDigest,
		_parent: &B::Header,
		inherents: &InherentData,
	) -> Result<(), ()> {
		digest.push(self.digest(inherents)?);
		Ok(())
	}
}
