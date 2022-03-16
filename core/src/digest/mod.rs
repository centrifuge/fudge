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

pub use babe::Digest as FudgeBabeDigest;
mod babe;

pub trait DigestCreator {
	fn create_digest(&self) -> Result<Digest, ()>;
}

pub trait DigestProvider {
	fn build_digest(&self) -> Result<Digest, ()>;
	fn append_digest(&self, digest: &mut Digest) -> Result<(), ()>;
}

impl<F> DigestCreator for F
where
	F: Fn() -> Result<Digest, ()> + Sync + Send,
{
	fn create_digest(&self) -> Result<Digest, ()> {
		(*self)()
	}
}

impl DigestCreator for Box<dyn DigestCreator + Send + Sync> {
	fn create_digest(&self) -> Result<Digest, ()> {
		(**self).create_digest()
	}
}
