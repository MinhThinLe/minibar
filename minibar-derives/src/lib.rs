use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(NamedModule)]
pub fn named_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = ast.ident;
    let module_name = struct_name.to_string().to_lowercase();

    quote! {
        impl NamedModule for #struct_name {
            fn name() -> &'static str {
                #module_name
            }
        }
    }.into()
}
