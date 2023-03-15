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
use proc_macro2::TokenStream;
use syn::{parse2, spanned::Spanned, Result};

use crate::parse::CompanionDef;

mod expand;
mod parse;

/// A macro for generating a relay-parachain-builder.
///
/// # Usage
/// In order to generate a companion builder use the macro as shown below.
///
/// ```rust
///
/// #[fudge::companion]
/// struct Local {
/// 	#[fudge::parachain(2000)]
/// 	pub centrifuge: ParachainBuilder<PBlock, PRtApi, PCidp, PDp, ()>,
///
///     #[fudge::parachain(2001)]
/// 	pub acala: ParachainBuilder<PBlock, PRtApi, PCidp, PDp, ()>,
///
///  	#[fudge::relaychain]
///		pub polkadot: RelaychainBuilder<RBlock, RRtApi, RRuntime, RCidp, RDp, ()>,
/// }
/// ```
#[proc_macro_attribute]
pub fn companion(
	attr: proc_macro::TokenStream,
	item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let attr: TokenStream = attr.into();
	let item: TokenStream = item.into();
	impl_companion_gen(attr, item)
		.unwrap_or_else(|err| err.to_compile_error())
		.into()
}

fn impl_companion_gen(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
	if !attr.is_empty() {
		let msg =
			"Invalid companion macro call: expected no attributes, e.g. macro call must be just \
			`#[fudge::companion]` or `#[companion]`";
		let span = proc_macro2::TokenStream::from(attr).span();
		return Err(syn::Error::new(span, msg));
	}

	let def: CompanionDef = parse2(item)?;
	expand::expand(def)
}

#[cfg(test)]
mod tests {}
