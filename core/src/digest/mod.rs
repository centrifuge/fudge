pub use aura::Digest as FudgeAuraDigest;
pub use babe::Digest as FudgeBabeDigest;
use sp_inherents::InherentData;
use sp_runtime::traits::Block;
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
use sp_runtime::{Digest, DigestItem};
use thiserror::Error;

mod aura;
mod babe;

type InnerError = Box<dyn std::error::Error>;

#[derive(Error, Debug)]
pub enum Error {
	#[error("slot duration retrieval: {0}")]
	SlotDurationRetrieval(InnerError),

	#[error("timestamp inherent data retrieval: {0}")]
	TimestampInherentDataRetrieval(InnerError),

	#[error("timestamp inherent data not found")]
	TimestampInherentDataNotFound,

	#[error("babe inherent data retrieval: {0}")]
	BabeInherentDataRetrieval(InnerError),

	#[error("babe inherent data not found")]
	BabeInherentDataNotFound,
}

#[async_trait::async_trait]
pub trait DigestCreator<B>
where
	B: Block,
{
	async fn create_digest(&self, parent: B::Header, inherents: InherentData)
		-> Result<Digest, ()>;
}

#[async_trait::async_trait]
pub trait DigestProvider<B: Block> {
	fn digest(&self, parent: B::Header, inherents: &InherentData) -> Result<DigestItem, Error>;

	async fn build_digest(
		&self,
		parent: B::Header,
		inherents: &InherentData,
	) -> Result<Digest, Error> {
		Ok(Digest {
			logs: vec![self.digest(parent, inherents)?],
		})
	}

	async fn append_digest(
		&self,
		parent: B::Header,
		digest: &mut Digest,
		inherents: &InherentData,
	) -> Result<(), Error> {
		digest.push(self.digest(parent, inherents)?);
		Ok(())
	}
}

#[async_trait::async_trait]
impl<F, Fut, B> DigestCreator<B> for F
where
	B: Block,
	F: Fn(B::Header, InherentData) -> Fut + Sync + Send,
	Fut: std::future::Future<Output = Result<Digest, ()>> + Send + 'static,
{
	async fn create_digest(
		&self,
		parent: B::Header,
		inherents: InherentData,
	) -> Result<Digest, ()> {
		(*self)(parent, inherents).await
	}
}

#[async_trait::async_trait]
impl<B> DigestCreator<B> for Box<dyn DigestCreator<B> + Send + Sync>
where
	B: Block,
{
	async fn create_digest(
		&self,
		parent: B::Header,
		inherents: InherentData,
	) -> Result<Digest, ()> {
		(**self).create_digest(parent, inherents).await
	}
}
