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

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, ToTokens};
use syn::Result as SynResult;

use crate::parse::{parachain, CompanionDef};

pub fn expand(def: CompanionDef) -> SynResult<TokenStream> {
	let fudge_crate = get_crate_with_name("fudge")?;
	let fudge_core_crate = get_crate_with_name("fudge-core")?;
	let tracing_crate = get_crate_with_name("sp-tracing")?;
	let codec_crate = get_crate_with_name("parity-scale-codec")?;
	let sp_std_crate = get_crate_with_name("sp-std")?;
	let thiserror_crate = get_crate_with_name("thiserror")?;

	let vis = def.vis.to_token_stream();
	let name = def.companion_name.to_token_stream();
	let parachains = parachain::helper::parachains(&def);
	let parachain_types = parachain::helper::parachain_types(&def);
	let parachain_names = parachain::helper::parachain_names(&def);
	let parachain_ids = parachain::helper::parachain_ids(&def);
	let relay_chain_name = def.relaychain.name.to_token_stream();
	let relay_chain = def.relaychain.builder.to_token_stream();
	let relay_vis = def.relaychain.vis.to_token_stream();
	let others_names: Vec<TokenStream> = def
		.others
		.iter()
		.map(|field| {
			field
				.ident
				.as_ref()
				.expect("Fudge parser ensures named fields.")
				.clone()
				.to_token_stream()
		})
		.collect();
	let others_types: Vec<TokenStream> = def
		.others
		.iter()
		.map(|field| field.ty.clone().to_token_stream())
		.collect();
	let others_vis: Vec<TokenStream> = def
		.others
		.iter()
		.map(|field| field.vis.clone().to_token_stream())
		.collect();

	let ts = quote! {
		use #fudge_core_crate::builder::parachain::FudgeCollator;
		use #fudge_core_crate::builder::parachain::Error as ParachainError;
		use #fudge_core_crate::builder::relay_chain::Error as RelayChainError;
		use #fudge_core_crate::builder::stand_alone::Error as StandaloneError;
		use #fudge_crate::primitives::{Chain as _hidden_Chain, ParaId as _hidden_ParaId, FudgeParaChain as _hidden_FudgeParaChain};
		use #codec_crate::Decode as __hidden_Decode;
		use #tracing_crate as __hidden_tracing;
		use #sp_std_crate::sync::{Arc, Mutex};
		use #thiserror_crate::Error;

		pub type InnerError = Box<dyn std::error::Error>;

		#[derive(Error, Debug)]
		pub enum BuilderError {
			#[error("standalone builder: {0}")]
			Standalone(InnerError),

			#[error("parachain builder: {0}")]
			Parachain(InnerError),

			#[error("relaychain builder: {0}")]
			Relaychain(InnerError),

			#[error("extrinsic decoding: {0}")]
			ExtrinsicDecode(InnerError),

			#[error("parachain with ID '{0}' not found")]
			ParachainNotFound(u32),
		}

		#vis struct #name {
			#relay_vis #relay_chain_name: #relay_chain,

			#(
				#parachains,
			)*

			#(
				#others_vis #others_names: #others_types,
			)*
		}

		impl #name {
			pub fn new(#relay_chain_name: #relay_chain, #(#parachain_names: #parachain_types,)* #(#others_names: #others_types,)*) -> Result<Self, BuilderError> {
				let mut companion = Self {
					#relay_chain_name,
					#(#parachain_names,)*
					#(#others_names,)*
				};

				#(
					__hidden_tracing::enter_span!(sp_tracing::Level::TRACE, std::stringify!(#relay_chain_name - Onboarding(#parachain_names):));

					let para = _hidden_FudgeParaChain {
						id: _hidden_ParaId::from(#parachain_ids),
						head: companion.#parachain_names.head().map_err(|e| BuilderError::Parachain(e.into()))?,
						code: companion.#parachain_names.code().map_err(|e| BuilderError::Parachain(e.into()))?,
					};

					let collator = companion.#parachain_names.collator();

					companion.#relay_chain_name.onboard_para(para, Box::new(collator)).map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;
				)*

				{
					__hidden_tracing::enter_span!(sp_tracing::Level::TRACE, std::stringify!(#relay_chain_name - BlockBuilding:));

					companion.#relay_chain_name.build_block().map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;
					companion.#relay_chain_name.import_block().map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;
				}

				Ok(companion)
			}

			pub fn append_extrinsics(&mut self, chain: _hidden_Chain, xts: Vec<Vec<u8>>) -> Result<(), BuilderError> {
				for xt in xts {
					self.append_extrinsic(chain, xt)?;
				}

				Ok(())
			}

			pub fn append_extrinsic(&mut self, chain: _hidden_Chain, xt: Vec<u8>) -> Result<(), BuilderError> {
				match chain {
					_hidden_Chain::Relay => {
						self.#relay_chain_name.append_extrinsic(
							__hidden_Decode::decode(&mut xt.as_slice()).map_err(|e| BuilderError::ExtrinsicDecode(e.into()))?
						).map_err(|e| BuilderError::Relaychain(e.into()))?;
						Ok(())
					},
					_hidden_Chain::Para(id) => match id {
						#(
							_ if id == #parachain_ids => {
								self.#parachain_names.append_extrinsic(
									__hidden_Decode::decode(&mut xt.as_slice()).map_err(|e| BuilderError::ExtrinsicDecode(e.into()))?
								).map_err(|e| BuilderError::Parachain(e.into()))?;
								Ok(())
							},
						)*
						_ => return Err(BuilderError::ParachainNotFound(id)),
					}
				}
			}

			pub fn with_state<R>(&self, chain: _hidden_Chain, exec: impl FnOnce() -> R) -> Result<R, BuilderError> {
				match chain {
					_hidden_Chain::Relay => {
						__hidden_tracing::enter_span!(sp_tracing::Level::TRACE, std::stringify!(#relay_chain_name - with_state:));

						self.#relay_chain_name.with_state(exec).map_err(|e| BuilderError::Relaychain(e.into()))
					},
					_hidden_Chain::Para(id) => match id {
						#(
							_ if id == #parachain_ids => {
								__hidden_tracing::enter_span!(sp_tracing::Level::INFO, std::stringify!(#parachain_names - with_state:));

								self.#parachain_names.with_state(exec).map_err(|e| BuilderError::Parachain(e.into()))
							}
						)*
						_ => Err(BuilderError::ParachainNotFound(id))
					}
				}
			}

			pub fn with_mut_state<R>(&mut self,  chain: _hidden_Chain, exec: impl FnOnce() -> R) -> Result<R, BuilderError> {
				match chain {
					_hidden_Chain::Relay => {
						__hidden_tracing::enter_span!(sp_tracing::Level::TRACE, std::stringify!(#relay_chain_name - with_mut_state:));

						self.#relay_chain_name.with_mut_state(exec).map_err(|e| BuilderError::Relaychain(e.into()))
					},
					_hidden_Chain::Para(id) => match id {
						#(
							_ if id == #parachain_ids => {
								__hidden_tracing::enter_span!(sp_tracing::Level::TRACE, std::stringify!(#parachain_names - with_mut_state:));

								self.#parachain_names.with_mut_state(exec).map_err(|e| BuilderError::Parachain(e.into()))
							}
						)*
						_ => Err(BuilderError::ParachainNotFound(id))
					}
				}
			}

			pub fn evolve(&mut self) -> Result<(), BuilderError> {
				{
					#(
						__hidden_tracing::enter_span!(sp_tracing::Level::TRACE, std::stringify!(#parachain_names - BlockBuilding:));

						let para_head = self.#parachain_names.head().map_err(|e| BuilderError::Parachain(e.into()))?;

						self.#relay_chain_name.update_para_head(
							self.#parachain_names.id(),
							para_head,
						).map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;

						self.#relay_chain_name.build_block().map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;

						self.#parachain_names.build_block().map_err(|e| BuilderError::Parachain(e.into())).map(|_| ())?;

						self.#relay_chain_name.import_block().map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;
						self.#relay_chain_name.build_block().map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;
						self.#relay_chain_name.import_block().map_err(|e| BuilderError::Relaychain(e.into())).map(|_| ())?;

						self.#parachain_names.import_block().map_err(|e| BuilderError::Parachain(e.into())).map(|_| ())?;
					)*
				}

				Ok(())
			}
		}
	};

	Ok(ts)
}

fn get_crate_with_name(name: &str) -> SynResult<TokenStream> {
	let found_crate = crate_name(name).map_err(|_| {
		syn::Error::new(
			Span::call_site(),
			format!("Crate {} must be present for companion macro.", name),
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
