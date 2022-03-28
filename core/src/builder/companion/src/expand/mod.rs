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
use crate::parse::parachain;
use crate::parse::CompanionDef;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Result;

pub fn expand(def: CompanionDef) -> Result<TokenStream> {
	let vis = def.vis.to_token_stream();
	let name = def.companion_name.to_token_stream();
	let parachain_names = parachain::helper::parachain_names(&def);
	let parachains = parachain::helper::parachains(&def);
	let relay_chain_name = def.relaychain.name;

	let ts = quote! {
		pub type FudgeResult<T> = std::result::Result<T, ()>;

		#vis struct #name {
			#(
				#parachain_names: #parachains,
			)*

			#relay_chain_name: (),
		}

		impl #name {
			pub fn with_state(&self) -> FudgeResult<()> {
				todo!()
			}

			pub fn with_mut_state(&self) -> FudgeResult<()> {
				todo!()
			}

			pub fn evolve(&mut self) -> FudgeResult<()> {
				todo!()
			}
		}
	};

	Ok(ts)
}
