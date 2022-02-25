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

pub use babe::Digest as BabeDigest;
mod babe;

#[async_trait::async_trait]
pub trait DigestCreator<Hash> {
	async fn create_digest(&self) -> Result<Digest<Hash>, ()>;
}

#[async_trait::async_trait]
pub trait DigestProvider<Hash> {
	async fn build_digest(&self) -> Result<Digest<Hash>, ()>;
	async fn append_digest(&self, digest: &mut Digest<Hash>) -> Result<(), ()>;
}

#[async_trait::async_trait]
impl<F, Fut, Hash> DigestCreator<Hash> for F
where
	F: Fn() -> Fut + Sync + Send,
	Fut: std::future::Future<Output = Result<Digest<Hash>, ()>> + Send + 'static,
{
	async fn create_digest(&self) -> Result<Digest<Hash>, ()> {
		(*self)().await
	}
}

#[async_trait::async_trait]
impl<Hash> DigestCreator<Hash> for Box<dyn DigestCreator<Hash> + Send + Sync> {
	async fn create_digest(&self) -> Result<Digest<Hash>, ()> {
		(*self).create_digest().await
	}
}
