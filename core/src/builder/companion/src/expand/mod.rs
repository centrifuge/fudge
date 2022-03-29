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
	let parachains = parachain::helper::parachains(&def);
	let parachain_names = parachain::helper::parachain_names(&def);
	let parachain_ids = parachain::helper::parachain_ids(&def);
	let relay_chain_name = def.relaychain.name;

	let ts = quote! {
		pub type FudgeResult<T> = std::result::Result<T, ()>;
		pub enum Chain {
			Relay,
			Para(u32)
		}

		#vis struct #name {
			#(
				#parachains,
			)*

			#relay_chain_name: (),
		}

		impl #name {
			pub fn new() -> FudgeResult<()> {
				todo!();
				// pass builder as parameters
				//
				// Onboard parachains
			}

			pub fn with_state<R>(&self, chain: Chain, exec: impl FnOnce() -> R,) -> FudgeResult<()> {
				match chain {
					Chain::Relay => self.#relay_chain_name.with_state(exec),
					Chain::Para(id) => match id {
						#(
							#parachain_ids => self.#parachain_names.with_state(exec).map_err(|_| ()).map(|_| ()),
						)*
						_ => Err(())
					}
				}
			}

			pub fn with_mut_state<R>(&self,  chain: Chain, exec: impl FnOnce() -> R) -> FudgeResult<()> {
				match chain {
					Chain::Relay => self.#relay_chain_name.with_mut_state(exec),
					Chain::Para(id) => match id {
						#(
							#parachain_ids => self.#parachain_names.with_mut_state(exec).map_err(|_| ()).map(|_| ()),
						)*
						_ => Err(())
					}
				}
			}

			pub fn evolve(&mut self) -> FudgeResult<()> {
				self.#relay_chain_name.build_block().map_err(|_| ()).map(|_| ())?;
				self.#relay_chain_name.import_block().map_err(|_| ()).map(|_| ())?;

				#(
					self.#parachain_names.build_block().map_err(|_| ()).map(|_| ())?;
					self.#parachain_names.import_block().map_err(|_| ()).map(|_| ())?;
					let para = FudgeParaChain {
						id: #parachain_ids,
						head: self.#parachain_names.head(),
						code: self.#parachain_names.code(),
					};
					self.#relay_chain_name.onboard_para(para)?;
				)*

				self.#relay_chain_name.build_block().map_err(|_| ()).map(|_| ())?;
				self.#relay_chain_name.import_block().map_err(|_| ()).map(|_| ())?;
			}
		}
	};

	Ok(ts)
}
