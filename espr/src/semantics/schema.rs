use super::{entity::*, namespace::*, scope::*, type_decl::*, *};
use crate::ast;
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
    pub entities: Vec<Entity>,
    pub types: Vec<TypeDecl>,
}

impl Legalize for Schema {
    type Input = ast::schema::Schema;
    fn legalize(
        ns: &Namespace,
        scope: &Scope,
        schema: &Self::Input,
    ) -> Result<Self, SemanticError> {
        let name = schema.name.clone();
        let here = scope.pushed(ScopeType::Schema, &name);
        let entities = schema
            .entities
            .iter()
            .map(|entity| Entity::legalize(ns, &here, entity))
            .collect::<Result<Vec<Entity>, _>>()?;
        let types = schema
            .types
            .iter()
            .map(|entity| TypeDecl::legalize(ns, &here, entity))
            .collect::<Result<Vec<TypeDecl>, _>>()?;
        Ok(Schema {
            name,
            entities,
            types,
        })
    }
}

impl ToTokens for Schema {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = format_ident!("{}", self.name);
        let types = &self.types;
        let entities = &self.entities;
        let entity_name: Vec<_> = entities
            .iter()
            .map(|e| format_ident!("{}", e.name.to_pascal_case()))
            .collect();
        let holder_name: Vec<_> = entities
            .iter()
            .map(|e| format_ident!("{}", e.name))
            .collect();
        let holder_type: Vec<_> = entities
            .iter()
            .map(|e| format_ident!("{}Holder", e.name.to_pascal_case()))
            .collect();
        let iter_name: Vec<_> = entities
            .iter()
            .map(|e| format_ident!("{}_iter", e.name))
            .collect();
        tokens.append_all(quote! {
            pub mod #name {
                use crate::{primitive::*, place_holder::*, tables::*, error::Result};
                use std::collections::HashMap;

                #[derive(Debug, Clone, Default)]
                pub struct Tables {
                    #(
                    #holder_name: HashMap<u64, #holder_type>,
                    )*
                }

                impl Tables {
                    #(
                    pub fn #iter_name<'table>(&'table self) ->
                        impl Iterator<Item = Result<#entity_name>> + 'table
                    {
                        self.#holder_name
                            .values()
                            .cloned()
                            .map(move |value| value.into_owned(&self))
                    }
                    )*
                }

                #(
                impl EntityTable<#holder_type> for Tables {
                    fn get_owned(&self, entity_id: u64) -> Result<#entity_name> {
                        crate::tables::get_owned(self, &self.#holder_name, entity_id)
                    }
                    fn owned_iter<'table>(&'table self) -> Box<dyn Iterator<Item = Result<#entity_name>> + 'table> {
                        crate::tables::owned_iter(self, &self.#holder_name)
                    }
                }
                )*

                #(#types)*
                #(#entities)*
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legalize() {
        let example = SyntaxTree::example();
        let ns = Namespace::new(&example).unwrap();
        dbg!(&ns);
        let schema = &example.schemas[0];
        let scope = Scope::root();
        let schema = Schema::legalize(&ns, &scope, schema).unwrap();
        dbg!(&schema);
    }
}
