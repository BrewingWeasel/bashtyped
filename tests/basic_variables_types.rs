use std::collections::HashMap;

use bashtyped::{BashType, Method, TypeDeclaration};

#[test]
fn test_creating_str_var() {
    let mut file = bashtyped::FileInfo::new(r#"a="lol" #/ string"#);
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([(
            String::from("a"),
            TypeDeclaration {
                bash_type: BashType::String,
                method: Method::Declared,
                range: 0..17,
            },
        )])
    );
}

#[test]
fn test_creating_inferred_str_var() {
    let mut file = bashtyped::FileInfo::new(r#"a="lol""#);
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([(
            String::from("a"),
            TypeDeclaration {
                bash_type: BashType::String,
                method: Method::Inferred,
                range: 0..7,
            },
        )])
    );
}

#[test]
fn test_creating_int_var() {
    let mut file = bashtyped::FileInfo::new(r#"a=1 #/ int"#);
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([(
            String::from("a"),
            TypeDeclaration {
                bash_type: BashType::Integer,
                method: Method::Declared,
                range: 0..10,
            },
        )])
    );
}

#[test]
fn test_creating_int_var_line_before() {
    let mut file = bashtyped::FileInfo::new(
        r#"
#/ int
a=1"#,
    );
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([(
            String::from("a"),
            TypeDeclaration {
                bash_type: BashType::Integer,
                method: Method::Declared,
                range: 1..11,
            },
        )])
    );
}

#[test]
fn test_creating_multiple_int_vars() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1 #/ int
b=3 #/ int"#,
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
                    bash_type: BashType::Integer,
                    method: Method::Declared,
                    range: 11..21,
                },
            )
        ])
    );
}

#[test]
fn test_creating_multiple_int_vars_first_inferred() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1
b=3 #/ int"#,
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
                    bash_type: BashType::Integer,
                    method: Method::Declared,
                    range: 4..14,
                },
            )
        ])
    );
}
