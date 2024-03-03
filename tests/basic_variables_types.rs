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

#[test]
fn test_creating_multiple_int_vars_three_inferred() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1
b=2
c=3"#,
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
                    method: Method::Inferred,
                    range: 4..7,
                },
            ),
            (
                String::from("c"),
                TypeDeclaration {
                    bash_type: BashType::Integer,
                    method: Method::Inferred,
                    range: 8..11,
                },
            )
        ])
    );
}

#[test]
fn test_creating_inferred_var_after_commands() {
    let mut file = bashtyped::FileInfo::new(
        r#"echo ""
sudo pacman -Syu
sudo rm -rf /
whoa="Silvester Belt""#,
    );
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([(
            String::from("whoa"),
            TypeDeclaration {
                bash_type: BashType::String,
                method: Method::Inferred,
                range: 39..60,
            },
        ),])
    );
}

#[test]
fn test_inferred_var_from_variable() {
    let mut file = bashtyped::FileInfo::new(
        r#"a="yes" #/ string
b="$a""#,
    );
    file.parse_code();
    assert!(file.errors.is_empty());
    assert_eq!(
        file.variables,
        HashMap::from([
            (
                String::from("a"),
                TypeDeclaration {
                    bash_type: BashType::String,
                    method: Method::Declared,
                    range: 0..17,
                },
            ),
            (
                String::from("b"),
                TypeDeclaration {
                    bash_type: BashType::String,
                    method: Method::Inferred,
                    range: 18..24,
                },
            )
        ])
    );
}

#[test]
fn test_inferred_int_from_variable_no_quotes() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1 #/ int
b="$a""#,
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
                    method: Method::Inferred,
                    range: 11..17,
                },
            )
        ])
    );
}

#[test]
fn test_inferred_with_var_in_string() {
    let mut file = bashtyped::FileInfo::new(
        r#"a=1
b="I love $b""#,
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
                    bash_type: BashType::String,
                    method: Method::Inferred,
                    range: 4..17,
                },
            ),
        ])
    );
}
