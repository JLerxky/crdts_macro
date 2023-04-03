use std::collections::HashMap;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Parser, Result};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Type};

struct Args(Type);

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Args(input.parse()?))
    }
}

#[proc_macro_attribute]
pub fn crdt(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let args = parse_macro_input!(args as Args);

    let v_clock_type = args.0;

    // If the struct has named fields, add a v_clock field to it
    if let syn::Data::Struct(ref mut struct_data) = ast.data {
        if let syn::Fields::Named(fields) = &mut struct_data.fields {
            fields.named.push(
                syn::Field::parse_named
                    .parse2(quote! { v_clock: crdts::VClock<#v_clock_type> })
                    .unwrap(),
            );
        } else {
            panic!("`crdt` can only be used on `struct`s that have named fields");
        }
    } else {
        panic!("`crdt` can only be used on `struct`s");
    }

    // add `CRDT` derive for the struct
    let gen = quote! {
        #[derive(crdts_macro::CRDT, Default, std::fmt::Debug, Clone, PartialEq, Eq, crdts_macro::serde::Serialize, crdts_macro::serde::Deserialize)]
        #[serde(crate = "crdts_macro::serde")]
        #ast
    };

    gen.into()
}

#[proc_macro_derive(CRDT)]
pub fn crdt_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse(input).unwrap();
    let expanded = impl_crdt_macro(input);
    proc_macro::TokenStream::from(expanded)
}

fn impl_crdt_macro(input: syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    let data = &input.data;

    let fields = list_fields(data);

    let m_error_name = Ident::new(&(name.to_string() + "CmRDTError"), Span::call_site());
    let m_error_enum = build_m_error(&fields);

    let v_error_name = Ident::new(&(name.to_string() + "CvRDTError"), Span::call_site());
    let v_error_enum = build_v_error(&fields);

    let op_name = Ident::new(&(name.to_string() + "CrdtOp"), Span::call_site());
    let op_param = build_op(&fields);

    let impl_apply = impl_apply(&fields);
    let impl_validate = impl_validate(&fields);

    let impl_merge = impl_merge(&fields);
    let impl_validate_merge = impl_validate_merge(&fields);

    quote! {
        #[derive(std::fmt::Debug, PartialEq, Eq)]
        pub enum #m_error_name {
            NoneOp,
            #m_error_enum
        }

        impl std::fmt::Display for #m_error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self, f)
            }
        }

        impl std::error::Error for #m_error_name {}

        #[allow(clippy::type_complexity)]
        #[derive(std::fmt::Debug, Clone, PartialEq, Eq, crdts_macro::serde::Serialize, crdts_macro::serde::Deserialize)]
        #[serde(crate = "crdts_macro::serde")]
        pub struct #op_name {
            #op_param
        }

        impl crdts::CmRDT for #name {
            type Op = #op_name;
            type Validation = #m_error_name;

            fn apply(&mut self, op: Self::Op) {
                #impl_apply
            }

            fn validate_op(&self, op: &Self::Op) -> Result<(), Self::Validation> {
                #impl_validate
            }
        }

        #[derive(std::fmt::Debug, PartialEq, Eq)]
        pub enum #v_error_name {
            #v_error_enum
        }

        impl std::fmt::Display for #v_error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self, f)
            }
        }

        impl std::error::Error for #v_error_name {}

        impl crdts::CvRDT for #name {
            type Validation = #v_error_name;

            fn validate_merge(&self, other: &Self) -> Result<(), Self::Validation> {
                #impl_validate_merge
                Ok(())
            }

            fn merge(&mut self, other: Self) {
                #impl_merge
            }
        }
    }
}

fn list_fields(data: &Data) -> HashMap<String, Type> {
    if let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = data
    {
        fields
            .named
            .iter()
            .map(|f| (f.ident.as_ref().unwrap().to_string(), f.ty.clone()))
            .collect()
    } else {
        HashMap::new()
    }
}

fn build_m_error(fields: &HashMap<String, Type>) -> TokenStream {
    fields
        .iter()
        .map(|(field_name, field_type)| {
            let pascal_name = field_name.to_case(Case::Pascal);
            let name = Ident::new(&pascal_name, Span::call_site());
            quote_spanned! { Span::call_site() =>
                #name(<#field_type as crdts::CmRDT>::Validation),
            }
        })
        .collect::<TokenStream>()
}

fn build_v_error(fields: &HashMap<String, Type>) -> TokenStream {
    fields
        .iter()
        .map(|(name, ty)| {
            let pascal_name = name.to_case(Case::Pascal);
            let name = Ident::new(&pascal_name, Span::call_site());
            quote_spanned! { Span::call_site() =>
                #name(<#ty as crdts::CvRDT>::Validation),
            }
        })
        .collect::<TokenStream>()
}

fn build_op(fields: &HashMap<String, Type>) -> TokenStream {
    let mut tokens = TokenStream::new();
    for (name, ty) in fields {
        let (name, is_vclock) = if name == "v_clock" {
            (Ident::new("dot", Span::call_site()), true)
        } else {
            (
                Ident::new(&format!("{}_op", name), Span::call_site()),
                false,
            )
        };
        let op_type = if is_vclock {
            quote! {<#ty as crdts::CmRDT>::Op}
        } else {
            quote! {Option<<#ty as crdts::CmRDT>::Op>}
        };
        tokens.extend(quote_spanned! {Span::call_site() =>
            pub #name: #op_type,
        });
    }
    tokens
}

fn impl_apply(fields: &HashMap<String, Type>) -> TokenStream {
    let op_params = op_params(fields);
    let nones = count_none(fields);

    let apply = fields.keys().filter(|f| *f != "v_clock").map(|f| {
        let field = Ident::new(f, Span::call_site());
        let op = Ident::new(&(f.to_owned() + "_op"), Span::call_site());

        quote_spanned! { Span::call_site() =>
            if let Some(#op) = #op {
                self.#field.apply(#op);
            }
        }
    });

    quote! {
        let Self::Op { dot, #op_params } = op;
        if self.v_clock.get(&dot.actor) >= dot.counter {
            return;
        }
        match (#op_params) {
            (#nones) => return,
            (#op_params) => { #(#apply)* }
        }
        self.v_clock.apply(dot);
    }
}

fn impl_validate(fields: &HashMap<String, Type>) -> TokenStream {
    let op_params = op_params(fields);
    let nones = count_none(fields);

    let validate = fields.keys().filter(|f| f != &"v_clock").map(|f| {
        let pascal_name = f.to_case(Case::Pascal);
        let error_name = Ident::new(&pascal_name, Span::call_site());
        let field = Ident::new(f, Span::call_site());
        let op = Ident::new(&(f.to_owned() + "_op"), Span::call_site());
        quote_spanned! { Span::call_site() =>
            if let Some(#op) = #op {
                self.#field.validate_op(#op).map_err(Self::Validation::#error_name)?;
            }
        }
    });

    quote! {
        let Self::Op {
            dot,
            #op_params
        } = op;
        self.v_clock.validate_op(dot).map_err(Self::Validation::VClock)?;
        match (#op_params) {
            (#nones) => return Err(Self::Validation::NoneOp),
            (#op_params) => {
                #(#validate)*
                return Ok(());
            }
        }
    }
}

fn impl_merge(fields: &HashMap<String, Type>) -> TokenStream {
    fields
        .keys()
        .map(|f| {
            let field = Ident::new(f, Span::call_site());
            quote_spanned! {
                Span::call_site() => self.#field.merge(other.#field);
            }
        })
        .collect()
}

fn impl_validate_merge(fields: &HashMap<String, Type>) -> TokenStream {
    fields
        .keys()
        .map(|field| {
            let error_name = Ident::new(&field.to_case(Case::Pascal), Span::call_site());
            let field = Ident::new(field, Span::call_site());
            quote! {
                self.#field.validate_merge(&other.#field)
                    .map_err(Self::Validation::#error_name)?;
            }
        })
        .collect()
}

fn count_none(fields: &HashMap<String, Type>) -> TokenStream {
    fields
        .keys()
        .filter(|&f| f != "v_clock")
        .map(|_| quote!(None,))
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<TokenStream>()
}

fn op_params(fields: &HashMap<String, Type>) -> TokenStream {
    fields
        .keys()
        .filter(|f| *f != "v_clock")
        .map(|f| format!("{}_op", f))
        .map(|i| Ident::new(&i, Span::call_site()))
        .map(|i| quote!(#i,))
        .collect()
}
