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
use quote::{quote, ToTokens};
use syn::{
	parse::{Parse, ParseStream},
	LitInt, Type, Visibility,
};

pub struct ParachainDef {
	pub name: Ident,
	pub id: TokenStream,
	pub builder: Type,
	pub vis: Visibility,
}

pub struct ParaId(TokenStream);

impl Parse for ParaId {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let content;
		syn::parenthesized!(content in input);
		let parsed_id = content.parse::<LitInt>()?.base10_parse::<u32>()?;
		Ok(ParaId(parsed_id.to_token_stream()))
	}
}

impl ToTokens for ParaId {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let id = self.0.clone();
		tokens.extend(quote! {#id})
	}
}

pub mod helper {
	use crate::parse::CompanionDef;
	use proc_macro2::TokenStream;
	use quote::{quote, ToTokens};

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
