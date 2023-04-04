pub use aura::Digest as FudgeAuraDigest;
pub use babe::Digest as FudgeBabeDigest;
use sp_inherents::InherentData;
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
use sp_runtime::Digest;

mod aura;
mod babe;
use sp_runtime::traits::Block;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
	#[error("couldn't retrieve timestamp inherent data: {0}")]
	TimestampInherentDataRetrieval(sp_inherents::Error),

	#[error("timestamp inherent data not found")]
	TimestampInherentDataNotFound,

	#[error("couldn't retrieve babe inherent data: {0}")]
	BabeInherentDataRetrieval(sp_inherents::Error),

	#[error("babe inherent data not found")]
	BabeInherentDataNotFound,
}

#[async_trait::async_trait]
pub trait DigestCreator<B>
where
	B: Block,
{
	async fn create_digest(
		&self,
		parent: B::Header,
		inherents: InherentData,
	) -> Result<Digest, Error>;
}

#[async_trait::async_trait]
pub trait DigestProvider<B: Block> {
	async fn build_digest(
		&self,
		parent: &B::Header,
		inherents: &InherentData,
	) -> Result<Digest, Error>;

	async fn append_digest(
		&self,
		digest: &mut Digest,
		parent: &B::Header,
		inherents: &InherentData,
	) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl<F, Fut, B> DigestCreator<B> for F
where
	B: Block,
	F: Fn(B::Header, InherentData) -> Fut + Sync + Send,
	Fut: std::future::Future<Output = Result<Digest, Error>> + Send + 'static,
{
	async fn create_digest(
		&self,
		parent: B::Header,
		inherents: InherentData,
	) -> Result<Digest, Error> {
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
	) -> Result<Digest, Error> {
		(**self).create_digest(parent, inherents).await
	}
}
