use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

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
    }
    .into()
}

#[proc_macro_derive(ModuleData)]
pub fn data_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = ast.ident;

    quote! {
        impl ModuleData for #struct_name {}
    }
    .into()
}
