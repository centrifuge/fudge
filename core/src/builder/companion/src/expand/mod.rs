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
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, ToTokens};
use syn::Result as SynResult;

fn get_fudge_crate() -> SynResult<TokenStream> {
	let found_crate = crate_name("fudge").map_err(|e| {
		syn::Error::new(
			Span::call_site(),
			"Crate fudge must be present for companion macro.",
		)
	})?;

	let ts = match found_crate {
		FoundCrate::Itself => quote!(crate),
		FoundCrate::Name(name) => {
			let ident = Ident::new(&name, Span::call_site());
			quote!( #ident )
		}
	};

	Ok(ts)
}

pub fn expand(def: CompanionDef) -> SynResult<TokenStream> {
	let fudge_crate = get_fudge_crate()?;
	let vis = def.vis.to_token_stream();
	let name = def.companion_name.to_token_stream();
	let parachains = parachain::helper::parachains(&def);
	let parachain_types = parachain::helper::parachain_types(&def);
	let parachain_names = parachain::helper::parachain_names(&def);
	let parachain_ids = parachain::helper::parachain_ids(&def);
	let relay_chain_name = def.relaychain.name.to_token_stream();
	let relay_chain = def.relaychain.builder.to_token_stream();
	let relay_vis = def.relaychain.vis.to_token_stream();

	let ts = quote! {
		use #fudge_crate::primitives::{Chain as _hidden_Chain, ParaId as _hidden_ParaId, FudgeParaChain as _hidden_FudgeParaChain};

		#vis struct #name {
			#(
				#parachains,
			)*

			#relay_vis #relay_chain_name: #relay_chain,
		}

		impl #name {
			pub fn new(#relay_chain_name: (), #(#parachain_names: #parachain_types,)*) -> Result<Self, ()> {
				let companion = Self {
					#relay_chain_name,
					#(#parachain_names,)*
				};

				#(
					let para = _hidden_FudgeParaChain {
						id: _hidden_ParaId::from(#parachain_ids),
						head: companion.#parachain_names.head(),
						code: companion.#parachain_names.code(),
					};
					companion.#relay_chain_name.onboard_para(para).map_err(|_| ()).map(|_| ())?;

				)*

				Ok(companion)
			}

			pub fn with_state<R>(&self, chain: _hidden_Chain, exec: impl FnOnce() -> R,) -> Result<(), ()> {
				match chain {
					_hidden_Chain::Relay => self.#relay_chain_name.with_state(exec),
					_hidden_Chain::Para(id) => match id {
						#(
							#parachain_ids => self.#parachain_names.with_state(exec).map_err(|_| ()).map(|_| ()),
						)*
						_ => Err(())
					}
				}
			}

			pub fn with_mut_state<R>(&self,  chain: _hidden_Chain, exec: impl FnOnce() -> R) -> Result<(), ()> {
				match chain {
					_hidden_Chain::Relay => self.#relay_chain_name.with_mut_state(exec),
					_hidden_Chain::Para(id) => match id {
						#(
							#parachain_ids => self.#parachain_names.with_mut_state(exec).map_err(|_| ()).map(|_| ()),
						)*
						_ => Err(())
					}
				}
			}

			pub fn evolve(&mut self) -> Result<(), ()> {
				self.#relay_chain_name.build_block().map_err(|_| ()).map(|_| ())?;
				self.#relay_chain_name.import_block().map_err(|_| ()).map(|_| ())?;

				#(
					self.#parachain_names.build_block().map_err(|_| ()).map(|_| ())?;
					self.#parachain_names.import_block().map_err(|_| ()).map(|_| ())?;
				)*

				self.#relay_chain_name.build_block().map_err(|_| ()).map(|_| ())?;
				self.#relay_chain_name.import_block().map_err(|_| ()).map(|_| ())?;

				#(
					let para = _hidden_FudgeParaChain {
						id: _hidden_ParaId::from(#parachain_ids),
						head: self.#parachain_names.head(),
						code: self.#parachain_names.code(),
					};
					self.#relay_chain_name.onboard_para(para).map_err(|_| ()).map(|_| ())?;
				)*
			}
		}
	};

	Ok(ts)
}
