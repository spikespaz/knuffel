use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;


pub enum Scalar {
    Enum(Enum),
}

pub struct Enum {
    pub ident: syn::Ident,
    pub variants: Vec<Variant>,
}

pub struct Variant {
    pub ident: syn::Ident,
    pub name: String,
}

impl Enum {
    fn new(ident: syn::Ident, _attrs: Vec<syn::Attribute>,
           src_variants: impl Iterator<Item=syn::Variant>)
        -> syn::Result<Self>
    {
        let mut variants = Vec::new();
        for var in src_variants {
            match var.fields {
                syn::Fields::Unit => {
                    let name = heck::KebabCase
                        ::to_kebab_case(&var.ident.to_string()[..]);
                    variants.push(Variant {
                        ident: var.ident,
                        name,
                    });
                }
                _ => {
                    return Err(syn::Error::new(var.span(),
                        "only unit variants are allowed for DecodeScalar"));
                }
            }
        }
        Ok(Enum {
            ident,
            variants,
        })
    }
}


impl Parse for Scalar {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = input.call(syn::Attribute::parse_outer)?;
        let ahead = input.fork();
        let _vis: syn::Visibility = ahead.parse()?;

        let lookahead = ahead.lookahead1();
        if lookahead.peek(syn::Token![enum]) {
            let item: syn::ItemEnum = input.parse()?;
            attrs.extend(item.attrs);
            Enum::new(item.ident, attrs,
                      item.variants.into_iter())
                .map(Scalar::Enum)
        } else {
            Err(lookahead.error())
        }
    }
}

pub fn emit_scalar(s: &Scalar) -> syn::Result<TokenStream> {
    match s {
        Scalar::Enum(e) => {
            emit_enum(e)
        }
    }
}


pub fn emit_enum(e: &Enum) -> syn::Result<TokenStream> {
    let e_name = &e.ident;
    let t_name_err = format!("unexpected type name for {}", e_name);
    let value_err = if e.variants.len() <= 3 {
        format!("expected one of {}",
                e.variants.iter()
                .map(|v| format!("`{}`", v.name.escape_default()))
                .collect::<Vec<_>>()
                .join(", "))
    } else {
        format!("expected `{}`, `{}`, or one of {} others",
                e.variants[0].name.escape_default(),
                e.variants[1].name.escape_default(),
                e.variants.len() - 2)
    };
    let match_branches = e.variants.iter()
        .map(|var| {
            let name = &var.name;
            let ident = &var.ident;
            quote!(#name => Ok(#e_name::#ident))
        });
    Ok(quote! {
        impl<S: ::knuffel::traits::Span> ::knuffel::DecodeScalar<S>
                for #e_name {
            fn raw_decode(val: &::knuffel::span::Spanned<
                          ::knuffel::ast::Literal, S>)
                -> Result<#e_name, ::knuffel::Error<S>>
            {
                match &**val {
                    ::knuffel::ast::Literal::String(ref s) => {
                        match &s[..] {
                            #(#match_branches,)*
                            _ => Err(::knuffel::Error::new(
                                    val.span(), #value_err)),
                        }
                    }
                    _ => Err(::knuffel::Error::new(val.span(),
                                                   "expected string value")),
                }
            }
            fn type_check(type_name: &Option<::knuffel::span::Spanned<
                          ::knuffel::ast::TypeName, S>>)
                -> Result<(), ::knuffel::Error<S>>
            {
                if let Some(typ) = type_name {
                    return Err(::knuffel::Error::new(typ.span(), #t_name_err));
                }
                Ok(())
            }
        }
    })
}