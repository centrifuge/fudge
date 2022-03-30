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

use proc_macro2::{Ident, TokenStream};
use syn::{parse::Parse, spanned::Spanned, Type, Visibility};

pub struct ParachainDef {
	pub name: Ident,
	pub id: TokenStream,
	pub builder: Type,
	pub vis: Visibility,
}

pub mod helper {
	use crate::parse::CompanionDef;
	use proc_macro2::{Ident, Literal, TokenStream, TokenTree};
	use quote::{quote, ToTokens, TokenStreamExt};

	pub fn parachain_ids(def: &CompanionDef) -> Vec<TokenStream> {
		def.parachains.iter().map(|para| para.id.clone()).collect()
	}

	pub fn parachain_names(def: &CompanionDef) -> Vec<TokenStream> {
		def.parachains
			.iter()
			.map(|para| para.name.clone().to_token_stream())
			.collect()
	}

	pub fn parachain_types(def: &CompanionDef) -> Vec<TokenStream> {
		def.parachains
			.iter()
			.map(|para| para.builder.clone().to_token_stream())
			.collect()
	}

	pub fn parachains(def: &CompanionDef) -> Vec<TokenStream> {
		def.parachains
			.iter()
			.map(|para| {
				let vis = para.vis.to_token_stream();
				let name = para.name.to_token_stream();
				let builder = para.builder.to_token_stream();
				quote! {
					#vis #name: #builder
				}
			})
			.collect()
	}
}
