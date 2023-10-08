use proc_macro::{self, TokenStream};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse::Parser, parse_macro_input, DeriveInput, Path, token::PathSep, punctuated::Punctuated, PathSegment};

fn make_dam_path(path: &str, fqn: bool) -> Path {
    let mut segments = Punctuated::new();
    segments.push(PathSegment { ident: Ident::new(path, Span::call_site()), arguments: syn::PathArguments::None });
    Path {
        leading_colon: (if fqn { Some(PathSep::default()) } else {None}),
        segments
    }
}

#[proc_macro_attribute]
pub fn context_internal(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    context_impl(_attrs, item, make_dam_path("crate", false))
}

#[proc_macro_attribute]
pub fn context_macro(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    context_impl(_attrs, item, make_dam_path("dam_rs", true))
}

fn context_impl(_attrs: TokenStream, item: TokenStream, dam_path: Path) -> TokenStream {
    let mut ast = parse_macro_input!(item as DeriveInput);

    let name = ast.ident.clone();
    let generics = ast.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => fields.named.push(
                    syn::Field::parse_named
                        .parse2(quote! {context_info: #dam_path::macro_support::ContextInfo})
                        .unwrap(),
                ),
                _ => (),
            }

            let ident_string = name.to_string();
            return quote! {
                #ast

                impl #impl_generics #dam_path::macro_support::Identifiable for #name #ty_generics #where_clause {
                    fn id(&self) -> #dam_path::macro_support::Identifier {
                        self.context_info.id
                    }

                    fn name(&self) -> String {
                        (#ident_string).into()
                    }
                }

                impl #impl_generics #dam_path::macro_support::TimeViewable for #name #ty_generics #where_clause {
                    fn view(&self) -> #dam_path::macro_support::TimeView {
                        self.context_info.time.view().into()
                    }
                }

                impl #impl_generics std::ops::Deref for #name #ty_generics #where_clause {
                    type Target = #dam_path::macro_support::ContextInfo;
                    fn deref(&self) -> &Self::Target {
                        &self.context_info
                    }
                }

                impl #impl_generics std::ops::DerefMut for #name #ty_generics #where_clause {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.context_info
                    }
                }
            }
            .into();
        }
        _ => quote! {compile_error!("Context can only be tagged on structs!")}.into(),
    }
}

#[proc_macro_attribute]
pub fn event_type_internal(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    event_type_impl(_attrs, item, make_dam_path("crate", false))
}

#[proc_macro_attribute]
pub fn event_type(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    event_type_impl(_attrs, item, make_dam_path("dan_rs", false))
}

fn event_type_impl(_attrs: TokenStream, item: TokenStream, dam_path: Path) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let name = ast.ident.clone();
    let ident_string = name.to_string();
    let mod_name = Ident::new(
        format!("{}_metrics_mod", ident_string).as_str(),
        name.span(),
    );

    let generics = ast.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #ast

        impl #impl_generics #dam_path::macro_support::logging::LogEvent for super::#name #ty_generics #where_clause {
            const NAME: &'static str = #ident_string;
        }

        #[allow(non_snake_case)]
        mod #mod_name {
            use #dam_path::macro_support::logging::registry::*;
            

            #[distributed_slice(METRICS)]
            static EVENT_NAME: &'static str = #ident_string;
        }
    }
    .into()
}
