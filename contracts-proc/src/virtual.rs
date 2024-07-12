use proc_macro::TokenStream;
use std::{mem, str::FromStr};

use quote::quote;
use syn::{
    FnArg,
    GenericParam,
    ImplItem
    ,
    ItemImpl, parse::Parse, parse_macro_input, punctuated::Punctuated
    , Token, Type,
};

use crate::create_complex_type_rec;

pub fn r#virtual(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemImpl);
    let (_, trait_path, _) =
        input.trait_.clone().expect("should contain trait implementation");
    let self_ty = input.self_ty.clone();
    let mut output = quote! {};

    let mut trait_declr_items = Vec::new();
    for item in input.items.iter_mut() {
        let ImplItem::Fn(func) = item else {
            trait_declr_items.push(quote! {
                #item
            });
            continue;
        };
        let name = func.sig.ident.clone();
        let return_ty = func.sig.output.clone();
        let args = func.sig.inputs.clone();
        let attrs = func.attrs.clone();
        let input_args = args.iter().map(|arg| match arg {
            FnArg::Receiver(_) => {
                panic!("functions should not contain `&self` or `&mut self`")
            }
            FnArg::Typed(arg) => arg.pat.clone(),
        });
        let generics = func.sig.generics.clone();

        if generics.params.is_empty() {
            trait_declr_items.push(quote! {
                #(#attrs)*
                fn #name <This: #trait_path> ( #args ) #return_ty
                {
                    Self::Base::#name::<This>(#(#input_args,)*)
                }
            });
        } else {
            let generic_idents =
                func.sig.generics.params.iter().map(|gp| match gp {
                    GenericParam::Type(ty) => ty.ident.clone(),
                    _ => {
                        panic!("functions should not contain lifetimes or const generics")
                    }
                });
            trait_declr_items.push(quote! {
                #(#attrs)*
                fn #name #generics ( #args ) #return_ty
                {
                    Self::Base::#name::<#(#generic_idents)*>(#(#input_args,)*)
                }
            });
        }
    }

    output.extend(quote! {
        pub trait #trait_path: 'static {
            type Base: #trait_path;

            #(#trait_declr_items)*
        }
    });

    let mut trait_impl_items = Vec::new();
    for item in input.items.iter_mut() {
        let ImplItem::Fn(func) = item else {
            trait_impl_items.push(quote! {
                #item
            });
            continue;
        };
        let generics = func.sig.generics.params.clone();
        if !generics.is_empty() {
            trait_impl_items.push(quote! {
                #func
            });
            continue;
        }
        let name = func.sig.ident.clone();
        let return_ty = func.sig.output.clone();
        let args = func.sig.inputs.clone();
        let block = func.block.clone();
        trait_impl_items.push(quote! {
            fn #name <This: #trait_path> ( #args ) #return_ty
            #block
        });
    }

    output.extend(quote! {
        impl #trait_path for #self_ty {
            type Base = Self;

            #(#trait_impl_items)*
        }
        
        pub struct #self_ty;
    });

    output.into()
}

pub fn r#override(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemImpl);
    let (_, trait_path, _) =
        input.trait_.clone().expect("should contain trait implementation");
    let self_ty = input.self_ty.clone();
    let mut output = quote! {};
    let override_alias = create_override_alias(&mut input);

    let mut items = Vec::new();
    for item in input.items.iter_mut() {
        let ImplItem::Fn(func) = item else {
            items.push(quote! {
                #item
            });
            continue;
        };
        let generics = func.sig.generics.params.clone();
        if !generics.is_empty() {
            items.push(quote! {
                #func
            });
            continue;
        }
        let name = func.sig.ident.clone();
        let return_ty = func.sig.output.clone();
        let args = func.sig.inputs.clone();
        let block = func.block.clone();
        items.push(quote! {
            fn #name <This: #trait_path> ( #args ) #return_ty
            #block
        });
    }

    output.extend(quote! {
        impl<Super: #trait_path> #trait_path for #self_ty<Super> {
            type Base = Super;

            #(#items)*
        }

        #override_alias

        pub struct #self_ty<Super: #trait_path>(Super);
    });
    output.into()
}

fn create_override_alias(input: &mut ItemImpl) -> proc_macro2::TokenStream {
    let mut inherits = vec![];
    for attr in mem::take(&mut input.attrs) {
        if !attr.path().is_ident("inherit") {
            input.attrs.push(attr);
            continue;
        }
        let contents: InheritsAttr = match attr.parse_args() {
            Ok(contents) => contents,
            Err(err) => return err.to_compile_error().into(),
        };
        for ty in contents.types {
            inherits.push(ty);
        }
    }
    if inherits.is_empty() {
        quote! {}
    } else {
        let override_ty = create_complex_type_rec(&inherits);
        quote! {
            type Override = #override_ty;
        }
    }
}

struct InheritsAttr {
    types: Punctuated<Type, Token![,]>,
}

impl Parse for InheritsAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let types = Punctuated::parse_separated_nonempty(input)?;
        Ok(Self { types })
    }
}