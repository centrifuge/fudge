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

use proc_macro2::Span;
use quote::ToTokens;
use syn::{
	parse::{Parse, ParseStream},
	parse2,
	spanned::Spanned,
	Attribute, Error, Field, FieldsNamed, Ident, ItemStruct, Result, Visibility,
};

use crate::parse::{parachain::ParachainDef, relaychain::RelaychainDef};

pub mod parachain;
pub mod relaychain;

/// List of additional token to be used for parsing.
mod keyword {
	syn::custom_keyword!(parachain);
	syn::custom_keyword!(relaychain);
}

/// Parse for `#[fudge::parachain]`
pub struct FieldAttrParachain(proc_macro2::Span);

impl Spanned for FieldAttrParachain {
	fn span(&self) -> proc_macro2::Span {
		self.0
	}
}

impl Parse for FieldAttrParachain {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		input.parse::<syn::Token![#]>()?;
		let content;
		syn::bracketed!(content in input);
		content.parse::<syn::Ident>()?;
		content.parse::<syn::Token![::]>()?;

		Ok(FieldAttrParachain(
			content.parse::<keyword::parachain>()?.span(),
		))
	}
}

pub struct CompanionDef {
	pub vis: Visibility,
	pub companion_name: Ident,
	pub attr_span: Span,
	pub relaychain: relaychain::RelaychainDef,
	pub parachains: Vec<parachain::ParachainDef>,
	pub others: Vec<Field>,
}

pub enum FieldType {
	Parachain(ParachainDef),
	Relaychain(RelaychainDef),
	Other(Field),
}

impl Parse for CompanionDef {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let ds: ItemStruct = input.parse()?;
		let span = ds.span();

		match ds.fields {
			syn::Fields::Named(named) => Self::parse_struct(
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

const EXPECTED_PATH_SEGMENTS: usize = 2;

impl CompanionDef {
	pub(crate) fn parse_struct(
		vis: Visibility,
		_attrs: Vec<Attribute>,
		companion_name: Ident,
		fields: FieldsNamed,
		attr_span: Span,
	) -> Result<Self> {
		let mut parachains = Vec::new();
		let mut others = Vec::new();
		let mut relaychain = None;

		for field in fields.named.iter() {
			match Self::parse_field(field.clone())? {
				FieldType::Other(named) => others.push(named), // TODO: Other fields are not supported currently
				FieldType::Parachain(def) => parachains.push(def),
				FieldType::Relaychain(def) => match relaychain {
					None => relaychain = Some(def),
					Some(_) => {
						return Err(Error::new(attr_span, "Only one relaychain field allowed."))
					}
				},
			}
		}

		Ok(Self {
			vis,
			companion_name,
			attr_span,
			parachains,
			relaychain: relaychain
				.ok_or_else(|| Error::new(fields.span(), "Relay chain not found."))?,
			others,
		})
	}

	pub fn parse_field(field: Field) -> Result<FieldType> {
		let companion_fields: Vec<&Attribute> = field
			.attrs
			.iter()
			.filter(|attr| {
				attr.path
					.segments
					.first()
					.map_or(false, |segment| segment.ident == "fudge")
			})
			.collect();

		if companion_fields.is_empty() {
			return Ok(FieldType::Other(field));
		}

		for attr in companion_fields {
			if attr.path.segments.len() != EXPECTED_PATH_SEGMENTS {
				return Err(Error::new(
					field.span(),
					format!("Expected {} path segments", EXPECTED_PATH_SEGMENTS),
				));
			}

			let second = attr
				.path
				.segments
				.last()
				.ok_or_else(|| Error::new(field.span(), "Last path segment not found."))?;

			return match second.ident.to_string().as_str() {
				"parachain" => {
					let id: parachain::ParaId = parse2(attr.tokens.clone())?;

					Ok(FieldType::Parachain(ParachainDef {
						name: field.ident.clone().ok_or_else(|| {
							Error::new(field.span(), "Parachain field identifier not found.")
						})?,
						id: id.to_token_stream(),
						builder: field.ty.clone(),
						vis: field.vis.clone(),
					}))
				}
				"relaychain" => Ok(FieldType::Relaychain(RelaychainDef {
					name: field.ident.clone().ok_or_else(|| {
						Error::new(field.span(), "Relaychain field identifier not found.")
					})?,
					builder: field.ty.clone(),
					vis: field.vis.clone(),
				})),
				_ => Err(Error::new(
					field.span(),
					"Only parachain or relaychain attributes supported currently.",
				)),
			};
		}

		Err(Error::new(
			field.span(),
			"Only parachain or relaychain attributes supported currently.",
		))
	}
}
