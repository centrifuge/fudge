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
use polkadot_primitives::v1::{
	InherentData as ParachainsInherentData, PARACHAINS_INHERENT_IDENTIFIER,
};
use sp_inherents::{Error, InherentData, InherentDataProvider, InherentIdentifier};
use sp_runtime::traits::Header;

pub struct Inherent<HDR: Header>(ParachainsInherentData<HDR>);

impl<HDR: Header> Inherent<HDR> {
	pub fn new(parent: HDR) -> Self {
		Inherent(ParachainsInherentData::<HDR> {
			bitfields: vec![],
			backed_candidates: vec![],
			disputes: vec![],
			parent_header: parent,
		})
	}
}

#[async_trait::async_trait]
impl<HDR: Header> InherentDataProvider for Inherent<HDR> {
	fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		inherent_data.put_data(PARACHAINS_INHERENT_IDENTIFIER, &self.0);

		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), Error>> {
		todo!()
	}
}
