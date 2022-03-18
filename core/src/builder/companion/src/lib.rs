use proc_macro::TokenStream;

#[proc_macro_attribute]
fn companion(metadata: TokenStream, input: TokenStream) -> TokenStream {
	// do something

	quote!()
}

#[cfg(test)]
mod tests {}
