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

use codec::Encode;
use sp_consensus_babe::{
	digests::{PreDigest, SecondaryPlainPreDigest},
	inherents::BabeInherentData,
	BABE_ENGINE_ID,
};
use sp_inherents::InherentData;
use sp_runtime::{traits::Block as BlockT, Digest as SPDigest, DigestItem};
use sp_std::marker::PhantomData;

use crate::digest::{DigestProvider, Error};

const DEFAULT_DIGEST_BABE_LOG_TARGET: &str = "fudge-digest-babe";

pub struct Digest<B> {
	_phantom: PhantomData<B>,
}

impl<B> Digest<B> {
	pub fn new() -> Self {
		Self {
			_phantom: Default::default(),
		}
	}

	fn digest(&self, inherents: &InherentData) -> Result<DigestItem, Error> {
		let slot = inherents
			.babe_inherent_data()
			.map_err(|e| {
				tracing::error!(
					target = DEFAULT_DIGEST_BABE_LOG_TARGET,
					error = ?e,
					"Couldn't retrieve babe inherent data."
				);

				Error::BabeInherentDataRetrieval(e)
			})?
			.ok_or({
				tracing::error!(
					target = DEFAULT_DIGEST_BABE_LOG_TARGET,
					"Babe inherent data not found."
				);

				Error::BabeInherentDataNotFound
			})?;

		let predigest = PreDigest::SecondaryPlain(SecondaryPlainPreDigest {
			authority_index: 0,
			slot,
		});

		Ok(DigestItem::PreRuntime(BABE_ENGINE_ID, predigest.encode()))
	}
}

#[async_trait::async_trait]
impl<B> DigestProvider<B> for Digest<B>
where
	B: BlockT,
{
	async fn build_digest(
		&self,
		_parent: &B::Header,
		inherents: &InherentData,
	) -> Result<SPDigest, Error> {
		Ok(SPDigest {
			logs: vec![self.digest(inherents)?],
		})
	}

	async fn append_digest(
		&self,
		digest: &mut SPDigest,
		_parent: &B::Header,
		inherents: &InherentData,
	) -> Result<(), Error> {
		digest.push(self.digest(inherents)?);
		Ok(())
	}
}
