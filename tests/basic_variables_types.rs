use std::collections::HashMap;

use bashtyped::{BashType, Method, TypeDeclaration};

#[test]
fn test_creating_str_var() {
    let mut file = bashtyped::FileInfo::new(r#"a="lol" #/ string"#);
    file.parse_code();
    assert!(file.errors.is_empty());
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
