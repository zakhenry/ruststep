//! Partial complex entities described in ISO-10303-11 Annex B

use super::*;
use crate::ast;
use std::collections::HashMap;

/// Global constraints in EXPRESS components
#[derive(Debug, PartialEq, Eq)]
pub struct Constraints {
    /// Each super-type can be instantiable as its subtypes,
    /// but possible subtypes cannot be determined from local description in EXPRESS.
    pub instantiables: HashMap<Path, Vec<Vec<Path>>>,
}

impl Constraints {
    pub fn new(ns: &Namespace, st: &SyntaxTree) -> Result<Self, SemanticError> {
        let mut instantiables = Vec::new();
        let root = Scope::root();
        for schema in &st.schemas {
            let scope = root.schema(&schema.name);

            // Be sure that `SUPERTYPE OF` declaration with complex constraint
            // using `ONEOF`, `AND` and `ANDOR` are deprecated:
            //
            // ISO-10303-11 (2004, en) Page 56, Note 1
            // > In order that existing schemas remain valid,
            // > the declaration of subtype/supertype constraints
            // > that use the keywords oneof, andor, or and within
            // > the declaration of an entity, as described in this sub-clause,
            // > remains valid under this edition 2 of EXPRESS.
            // > However, its use is deprecated, and its removal is planned
            // > in future editions. The use of the subtype constraint (see 9.7)
            // > is encouraged instead.
            //
            for entity in &schema.entities {
                match &entity.constraint {
                    Some(ast::Constraint::SuperTypeRule(expr)) => {
                        let path = Path::new(&scope, ScopeType::Entity, &entity.name);
                        instantiables.push((path, Instantiables::from_expr(ns, &scope, expr)?));
                    }
                    _ => continue,
                }
            }
            // TODO: SUBTYPE_CONSTRAINTS
        }

        // TODO Add implicit constraints

        // Replace indices to Path using Namespace
        let instantiables = instantiables
            .into_iter()
            .map(|(path, it)| {
                let it: Vec<Vec<Path>> = it
                    .parts
                    .iter()
                    .map(|pce| {
                        pce.indices
                            .iter()
                            .map(|index| {
                                let (path, _ast) = &ns[*index];
                                path.clone()
                            })
                            .collect()
                    })
                    .collect();
                (path, it)
            })
            .collect();
        Ok(Constraints { instantiables })
    }

    pub fn is_supertype(&self, path: &Path) -> bool {
        self.instantiables.contains_key(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_oneof() {
        let st = ast::SyntaxTree::parse(
            r#"
            SCHEMA test_schema;
              ENTITY base SUPERTYPE OF (ONEOF (sub1, sub2));
                x: REAL;
              END_ENTITY;

              ENTITY sub1 SUBTYPE OF (base);
                y1: REAL;
              END_ENTITY;

              ENTITY sub2 SUBTYPE OF (base);
                y2: REAL;
              END_ENTITY;
            END_SCHEMA;
            "#,
        )
        .unwrap();

        let ns = Namespace::new(&st);
        let c = Constraints::new(&ns, &st).unwrap();

        let scope = Scope::root().schema("test_schema");
        assert_eq!(
            c,
            Constraints {
                instantiables: maplit::hashmap! {
                    Path::entity(&scope, "base") => vec![
                        vec![Path::entity(&scope, "sub1")],
                        vec![Path::entity(&scope, "sub2")],
                    ]
                }
            }
        );
    }

    #[test]
    fn constraint_andor() {
        // Based on `ANDOR` example in ISO-10303-11
        let st = ast::SyntaxTree::parse(
            r#"
            SCHEMA test_schema;
              ENTITY person SUPERTYPE OF (employee ANDOR student);
              END_ENTITY;
              ENTITY employee SUBTYPE OF (person);
              END_ENTITY;
              ENTITY student SUBTYPE OF (person);
              END_ENTITY;
            END_SCHEMA;
            "#,
        )
        .unwrap();

        let ns = Namespace::new(&st);
        let c = Constraints::new(&ns, &st).unwrap();

        let scope = Scope::root().schema("test_schema");
        assert_eq!(
            c,
            Constraints {
                instantiables: maplit::hashmap! {
                    Path::entity(&scope, "person") => vec![
                        vec![Path::entity(&scope, "employee")],
                        vec![Path::entity(&scope, "student")],
                        vec![Path::entity(&scope, "employee"), Path::entity(&scope, "student")],
                    ]
                }
            }
        );
    }

    #[test]
    fn constraint_and() {
        // Based on `AND` example in ISO-10303-11
        let st = ast::SyntaxTree::parse(
            r#"
            SCHEMA test_schema;
              ENTITY person SUPERTYPE OF (ONEOF(male,female) AND ONEOF(citizen,alien));
              END_ENTITY;
              ENTITY male SUBTYPE OF (person);
              END_ENTITY;
              ENTITY female SUBTYPE OF (person);
              END_ENTITY;
              ENTITY citizen SUBTYPE OF (person);
              END_ENTITY;
              ENTITY alien SUBTYPE OF (person);
              END_ENTITY;
            END_SCHEMA;
            "#,
        )
        .unwrap();

        let ns = Namespace::new(&st);
        let c = Constraints::new(&ns, &st).unwrap();

        let scope = Scope::root().schema("test_schema");
        assert_eq!(
            c,
            Constraints {
                instantiables: maplit::hashmap! {
                    Path::entity(&scope, "person") => vec![
                        vec![Path::entity(&scope, "male"), Path::entity(&scope, "citizen")],
                        vec![Path::entity(&scope, "male"), Path::entity(&scope, "alien")],
                        vec![Path::entity(&scope, "female"), Path::entity(&scope, "citizen")],
                        vec![Path::entity(&scope, "female"), Path::entity(&scope, "alien")],
                    ]
                }
            }
        );
    }
}
