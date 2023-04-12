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
pub use aura::Digest as FudgeAuraDigest;
pub use babe::Digest as FudgeBabeDigest;
use sp_inherents::InherentData;
use sp_runtime::{traits::Block, Digest, DigestItem};
use thiserror::Error;

mod aura;
mod babe;

#[derive(Error, Debug)]
pub enum Error {
	#[error("timestamp inherent data retrieval: {0}")]
	TimestampInherentDataRetrieval(sp_inherents::Error),

	#[error("timestamp inherent data not found")]
	TimestampInherentDataNotFound,

	#[error("babe inherent data retrieval: {0}")]
	BabeInherentDataRetrieval(sp_inherents::Error),

	#[error("babe inherent data not found")]
	BabeInherentDataNotFound,
}

#[async_trait::async_trait]
pub trait DigestCreator<B>
where
	B: Block,
{
	async fn create_digest(&self, inherents: InherentData) -> Result<Digest, Error>;
}

#[async_trait::async_trait]
pub trait DigestProvider<B: Block> {
	fn digest(&self, inherents: &InherentData) -> Result<DigestItem, Error>;

	async fn build_digest(&self, inherents: &InherentData) -> Result<Digest, Error> {
		Ok(Digest {
			logs: vec![self.digest(inherents)?],
		})
	}

	async fn append_digest(
		&self,
		digest: &mut Digest,
		inherents: &InherentData,
	) -> Result<(), Error> {
		digest.push(self.digest(inherents)?);
		Ok(())
	}
}

#[async_trait::async_trait]
impl<F, Fut, B> DigestCreator<B> for F
where
	B: Block,
	F: Fn(InherentData) -> Fut + Sync + Send,
	Fut: std::future::Future<Output = Result<Digest, Error>> + Send + 'static,
{
	async fn create_digest(&self, inherents: InherentData) -> Result<Digest, Error> {
		(*self)(inherents).await
	}
}

#[async_trait::async_trait]
impl<B> DigestCreator<B> for Box<dyn DigestCreator<B> + Send + Sync>
where
	B: Block,
{
	async fn create_digest(&self, inherents: InherentData) -> Result<Digest, Error> {
		(**self).create_digest(inherents).await
	}
}
