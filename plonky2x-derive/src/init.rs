use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;

pub fn init(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            let recurse = data.fields.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;
                quote! {
                    #name: <#ty as CircuitVariable>::init(builder),
                }
            });
            quote! {
                Self {
                    #(#recurse)*
                }
            }
        }
        Data::Enum(_) => unimplemented!("enums not supported"),
        Data::Union(_) => unimplemented!("unions not supported"),
    }
}
