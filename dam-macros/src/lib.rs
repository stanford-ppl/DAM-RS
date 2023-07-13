use std::collections::HashSet;

use phf::phf_map;
use proc_macro::{self, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse::Parser, parse_macro_input, parse_str, DeriveInput, ImplItemFn};

#[proc_macro_attribute]
pub fn identifiable(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as DeriveInput);

    let name = ast.ident.clone();
    let generics = ast.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => fields.named.push(
                    syn::Field::parse_named
                        .parse2(quote! {identifier: dam_core::identifier::Identifier})
                        .unwrap(),
                ),
                _ => (),
            }

            let ident_string = name.to_string();
            return quote! {
                #ast

                impl #impl_generics dam_core::identifier::Identifiable for #name #ty_generics #where_clause {
                    fn id(&self) -> dam_core::identifier::Identifier {
                        self.identifier
                    }

                    fn name(&self) -> String {
                        (#ident_string).into()
                    }
                }
            }
            .into();
        }
        _ => quote! {compile_error!("identifiable can only be tagged on structs!")}.into(),
    }
}

#[proc_macro_attribute]
pub fn time_managed(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as DeriveInput);

    let name = ast.ident.clone();
    let generics = ast.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => fields.named.push(
                    syn::Field::parse_named
                        .parse2(quote! {time: dam_core::TimeManager})
                        .unwrap(),
                ),
                _ => (),
            }

            return quote! {
                #ast

                impl #impl_generics dam_core::TimeManaged for #name #ty_generics #where_clause {
                    fn time_manager_mut(&mut self) -> &mut dam_core::TimeManager {
                        &mut self.time
                    }

                    fn time_manager(&self) -> &dam_core::TimeManager {
                        &self.time
                    }
                }

                impl #impl_generics dam_core::TimeViewable for #name #ty_generics #where_clause {
                    fn view(&self) -> dam_core::TimeView {
                        self.time.view().into()
                    }
                }
            }
            .into();
        }
        _ => quote! {compile_error!("time_viewable can only be tagged on structs!")}.into(),
    }
}

static MEMBER_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    "time_managed" => "time",
};

#[proc_macro_attribute]
pub fn cleanup(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as ImplItemFn);
    let all_attrs = attrs.into_iter().collect::<Vec<_>>();
    let mut cleanup_attrs = Vec::new();
    let mut already_seen = HashSet::<String>::new();
    for (ind, attr) in all_attrs.iter().enumerate() {
        match attr {
            proc_macro::TokenTree::Ident(ident) if ind % 2 == 0 => {
                let ident_str = ident.to_string();
                if already_seen.contains(&ident_str) {
                    return quote! {compile_error!("Cannot execute duplicate cleanups!");}.into();
                }
                already_seen.insert(ident_str.clone());
                match MEMBER_MAP.get(ident_str.as_str()) {
                    Some(repl) => cleanup_attrs.push(*repl),
                    None => {
                        return quote! {compile_error!("Could not find a valid member map entry.");}
                            .into()
                    }
                }
            }
            proc_macro::TokenTree::Punct(comma) if ind % 2 == 1 && comma.as_char() == ',' => {}
            _ => {
                return quote!{compile_error!("Unexpected token while processing cleanup macro!"); }.into();
            }
        };
    }

    if ast.sig.ident != "cleanup" {
        return quote! {
            compile_error!("#[cleanup] can only be attached to methods named cleanup(&self)!");
        }
        .into();
    }

    for cleanup_attr in cleanup_attrs.into_iter() {
        let stmt = parse_str(format!("self.{}.cleanup();", cleanup_attr).as_str()).unwrap();
        ast.block.stmts.push(stmt);
    }

    ast.into_token_stream().into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn playground() {
        use syn::{parse_quote, Attribute};

        let attr: Attribute = parse_quote! {
            #[cleanup(time_viewable, identifiable)]
        };

        attr.parse_nested_meta(|meta| {
            println!("{:?}", meta.path);
            Ok(())
        })
        .unwrap();

        3;
    }
}
