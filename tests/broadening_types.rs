use std::collections::HashMap;

use bashtyped::{BashType, Method, TypeDeclaration};

#[test]
fn test_broadening_type_def_from_other_var() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1
b="$a" #/ int | string "#,
    );
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([
            (
                String::from("a"),
                TypeDeclaration {
                    bash_type: BashType::Integer,
                    method: Method::Inferred,
                    range: 0..3,
                },
            ),
            (
                String::from("b"),
                TypeDeclaration {
                    bash_type: BashType::Or(
                        Box::new(BashType::Integer),
                        Box::new(BashType::String)
                    ),
                    method: Method::Declared,
                    range: 4..27,
                },
            ),
        ])
    );
}

#[test]
fn test_further_broadening() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1 #/ int | bool
b="$a" #/ int | string | bool "#,
    );
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([
            (
                String::from("a"),
                TypeDeclaration {
                    bash_type: BashType::Or(Box::new(BashType::Integer), Box::new(BashType::Bool)),
                    method: Method::Declared,
                    range: 0..17,
                },
            ),
            (
                String::from("b"),
                TypeDeclaration {
                    bash_type: BashType::Or(
                        Box::new(BashType::Integer),
                        Box::new(BashType::Or(
                            Box::new(BashType::String),
                            Box::new(BashType::Bool)
                        ))
                    ),
                    method: Method::Declared,
                    range: 18..48,
                },
            ),
        ])
    );
}

#[test]
fn test_less_broadening() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1 #/ int | bool | string
b="$a" #/ int | string "#,
    );
    file.parse_code();
    assert!(!file.errors.is_empty());
}

#[test]
fn test_either_type_to_single() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1 #/ int | bool
b="$a" #/ bool "#,
    );
    file.parse_code();
    assert!(!file.errors.is_empty());
}

#[test]
fn test_int_to_any() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1 #/ int
b="$a" #/ any "#,
    );
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([
            (
                String::from("a"),
                TypeDeclaration {
                    bash_type: BashType::Integer,
                    method: Method::Declared,
                    range: 0..10,
                },
            ),
            (
                String::from("b"),
                TypeDeclaration {
                    bash_type: BashType::Any,
                    method: Method::Declared,
                    range: 11..25,
                },
            ),
        ])
    );
}

#[test]
fn test_any_to_int() {
    let mut file = bashtyped::FileInfo::new(
        r#"a="v" #/ any
b="$a" #/ int "#,
    );
    file.parse_code();
    assert!(!file.errors.is_empty());
}

#[test]
fn test_or_in_diff_orders() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1 #/ int | string
b="$a" #/ string | int "#,
    );
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([
            (
                String::from("a"),
                TypeDeclaration {
                    bash_type: BashType::Or(
                        Box::new(BashType::Integer),
                        Box::new(BashType::String)
                    ),
                    method: Method::Declared,
                    range: 0..19,
                },
            ),
            (
                String::from("b"),
                TypeDeclaration {
                    bash_type: BashType::Or(
                        Box::new(BashType::String),
                        Box::new(BashType::Integer)
                    ),
                    method: Method::Declared,
                    range: 20..43,
                },
            ),
        ])
    );
}
