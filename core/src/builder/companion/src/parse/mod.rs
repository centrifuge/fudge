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
use crate::parse::parachain::ParachainDef;
use crate::parse::relaychain::RelayChainDef;
use proc_macro2::Span;
use syn::{
	parse::{Parse, ParseStream},
	spanned::Spanned,
	Attribute, Error, FieldsNamed, Ident, ItemStruct, Result, Visibility,
};

pub mod parachain;
pub mod relaychain;

/// List of additional token to be used for parsing.
mod keyword {
	syn::custom_keyword!(parachain);
	syn::custom_keyword!(relaychain);
}

pub struct CompanionDef {
	pub vis: Visibility,
	pub companion_name: Ident,
	pub attr_span: Span,
	pub relaychain: relaychain::RelayChainDef,
	pub parachains: Vec<parachain::ParachainDef>,
}

impl Parse for CompanionDef {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let ds: ItemStruct = input.parse()?;
		let span = ds.span();

		match ds.fields {
			syn::Fields::Named(named) => Self::parse_fields(
				ds.vis.clone(),
				ds.attrs.clone(),
				ds.ident.clone(),
				named,
				span,
			),
			syn::Fields::Unit => Err(Error::new(
				ds.fields.span(),
				"Must be a struct with named fields. Not an unit struct.",
			)),
			syn::Fields::Unnamed(unnamed) => Err(Error::new(
				unnamed.span(),
				"Must be a struct with named fields. Not an unnamed fields struct.",
			)),
		}
	}
}

impl CompanionDef {
	pub(crate) fn parse_fields(
		vis: Visibility,
		_attrs: Vec<Attribute>,
		companion_name: Ident,
		fields: FieldsNamed,
		attr_span: Span,
	) -> Result<Self> {
		// TODO: Remove and replace with actual parsing
		let mut names: Vec<Ident> = fields
			.named
			.iter()
			.map(|field| {
				field
					.ident
					.as_ref()
					.expect("Parse ensures named field. qed.")
					.clone()
			})
			.collect();
		let relaychain = RelayChainDef {
			name: names.pop().unwrap(),
		};

		let parachains: Vec<ParachainDef> = names
			.into_iter()
			.map(|para_name| ParachainDef { name: para_name })
			.collect();
		// TODO: Remove until here

		Ok(Self {
			vis,
			companion_name,
			attr_span,
			parachains,
			relaychain,
		})
	}
}
