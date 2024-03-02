use ariadne::Source;
use bashtyped::FileInfo;

fn main() {
    let source_code = r#"# other thing
val="lol" #/ bool | string
val=1 #/ int
echo "hi"
"#;

    let mut info = FileInfo::new(source_code);

    info.parse_code();

    for error in info.errors {
        error
            .print(Source::from(info.source_code))
            .expect("comment printing to work");
    }
}
