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
use crate::builder::parachain::FudgeParaBuild;
use polkadot_primitives::v1::{
	BackedCandidate, InherentData as ParachainsInherentData, PARACHAINS_INHERENT_IDENTIFIER,
};
use sp_inherents::{Error, InherentData, InherentDataProvider, InherentIdentifier};
use sp_runtime::traits::Header;
use sp_std::sync::Arc;

pub struct DummyInherent<HDR> {
	parent_head: HDR,
}

impl<HDR: Header> DummyInherent<HDR> {
	pub fn new(parent_head: HDR) -> Self {
		Self { parent_head }
	}
}

#[async_trait::async_trait]
impl<HDR: Header> InherentDataProvider for DummyInherent<HDR> {
	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		let inherent = ParachainsInherentData::<HDR> {
			bitfields: vec![],
			backed_candidates: vec![],
			disputes: vec![],
			parent_header: self.parent_head.clone(),
		};

		inherent_data.put_data(PARACHAINS_INHERENT_IDENTIFIER, &inherent)
	}

	async fn try_handle_error(
		&self,
		_identifier: &InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), Error>> {
		todo!()
	}
}

pub struct Inherent<HDR: Header, Client> {
	para_builds: Vec<FudgeParaBuild>,
	backed_candidates: Vec<BackedCandidate<HDR::Hash>>,
	parent_head: HDR,
	relay: Arc<Client>,
}

impl<HDR: Header, Client> Inherent<HDR, Client> {
	pub fn new(
		relay: Arc<Client>,
		parent_head: HDR,
		para_builds: Vec<FudgeParaBuild>,
		backed_candidates: Vec<BackedCandidate<HDR::Hash>>,
	) -> Self {
		Self {
			relay,
			para_builds,
			backed_candidates,
			parent_head,
		}
	}
}

#[async_trait::async_trait]
impl<HDR, Client> InherentDataProvider for Inherent<HDR, Client>
where
	HDR: Header,
	Client: Sync + Send,
{
	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		// TODO: - From `FudgeParaBuild` -> Generate new backed_candidates
		//       - From `BackedCandidate` -> Generate new bitfields

		let inherent = ParachainsInherentData::<HDR> {
			bitfields: vec![],
			backed_candidates: vec![],
			disputes: vec![],
			parent_header: self.parent_head.clone(),
		};

		inherent_data.put_data(PARACHAINS_INHERENT_IDENTIFIER, &inherent)
	}

	async fn try_handle_error(
		&self,
		_identifier: &InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), Error>> {
		todo!()
	}
}
