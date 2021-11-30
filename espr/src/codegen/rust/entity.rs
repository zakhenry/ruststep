use crate::ir::*;

use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::*;

impl ToTokens for EntityAttribute {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let EntityAttribute { name, ty, optional } = self;

        let attr_name = format_ident!("{}", name);
        let attr_type = if *optional {
            quote! { Option<#ty> }
        } else {
            quote! { #ty }
        };
        let use_place_holder = if ty.is_simple() {
            quote! {}
        } else {
            quote! { #[holder(use_place_holder)] }
        };

        tokens.append_all(quote! {
            #use_place_holder
            pub #attr_name : #attr_type
        });
    }
}

impl Entity {
    fn generate_any_def(&self, tokens: &mut TokenStream) {
        let name = format_ident!("{}", self.name.to_pascal_case());
        if !self.subtypes.is_empty() {
            let subtypes = &self.subtypes;
            let names: Vec<_> = subtypes
                .iter()
                .map(|ty| match &ty {
                    TypeRef::Entity { name, .. } => format_ident!("{}", name),
                    _ => unreachable!(),
                })
                .collect();
            let enum_name = format_ident!("{}Any", name);
            tokens.append_all(quote! {
                #[derive(Debug, Clone, PartialEq, Holder)]
                #[holder(table = Tables)]
                #[holder(generate_deserialize)]
                pub enum #enum_name {
                    #(
                    #[holder(use_place_holder)]
                    #[holder(field = #names)]
                    #subtypes(Box<#subtypes>)
                    ),*
                }
            }); // tokens.append_all
        }
    }

    fn generate_into_any(&self, tokens: &mut TokenStream) {
        for ty in &self.supertypes {
            if let TypeRef::Entity {
                name: supertype_name,
                is_supertype,
                ..
            } = ty
            {
                if *is_supertype {
                    let name = if self.subtypes.is_empty() {
                        format_ident!("{}", self.name.to_pascal_case())
                    } else {
                        format_ident!("{}Any", self.name.to_pascal_case())
                    };
                    let any_enum = format_ident!("{}Any", supertype_name.to_pascal_case());
                    tokens.append_all(quote! {
                        impl Into<#any_enum> for #name {
                            fn into(self) -> #any_enum {
                                #any_enum::#name(Box::new(self))
                            }
                        }
                    });
                }
            }
        }
    }

    fn supertype_attributes(&self) -> TokenStream {
        let mut attr_name = Vec::new();
        let mut attr_type = Vec::new();

        for ty in &self.supertypes {
            let (attr, ty) = match ty {
                TypeRef::Named { name, .. } | TypeRef::Entity { name, .. } => {
                    (format_ident!("{}", name), ty)
                }
                _ => unreachable!(),
            };

            attr_name.push(attr.clone());
            attr_type.push(ty.to_token_stream());
        }

        quote! {
            #(pub #attr_name : #attr_type),*
        }
    }
}

impl ToTokens for Entity {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let field_name = format_ident!("{}", self.name);
        let name = format_ident!("{}", self.name.to_pascal_case());

        let attributes = &self.attributes;
        let supertype_attributes = self.supertype_attributes();

        tokens.append_all(quote! {
            #[derive(Debug, Clone, PartialEq, ::derive_new::new, Holder)]
            #[holder(table = Tables)]
            #[holder(field = #field_name)]
            #[holder(generate_deserialize)]
            pub struct #name {
                #supertype_attributes
                #(#attributes),*
            }
        });

        self.generate_any_def(tokens);
        self.generate_into_any(tokens);
    }
}
