use std::{collections::HashMap, fmt::Display, ops::Range};

use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use tree_sitter::{Node, Parser, TreeCursor};
use tree_sitter_bash;

fn main() {
    let source_code = r#"# other thing
do_thing() {
    #/ string
    value="lol"

    #/ int
    fakeint="lol"

    #/ int
    value=1
}"#;

    let mut info = FileInfo::new(source_code);

    info.parse_code();

    for error in info.errors {
        error.print(Source::from(info.source_code)).unwrap();
    }
}

struct FileInfo<'src> {
    source_code: &'src str,
    parser: Parser,
    variables: HashMap<String, TypeDeclaration>,
    errors: Vec<Report<'src>>,
    config: Config,
}

struct Comment<'src> {
    text: &'src str,
    range: Range<usize>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum BashType {
    String,
    Integer,
    Any,
}

impl Display for BashType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BashType::Any => "any",
                BashType::String => "string",
                BashType::Integer => "int",
            }
        )
    }
}

impl BashType {
    fn matches(&self, other: &Self) -> bool {
        self == &BashType::Any || other == &BashType::Any || self == other
    }
}

struct Config {
    specified_color: Color,
    inferred_color: Color,
    previous_color: Color,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            specified_color: Color::Blue,
            inferred_color: Color::Magenta,
            previous_color: Color::Red,
        }
    }
}

struct TypeDeclaration {
    range: Range<usize>,
    bash_type: BashType,
    method: Method,
}

enum Method {
    Inferred,
    Declared,
}

impl<'a> FileInfo<'a> {
    pub fn new(source_code: &'a str) -> FileInfo<'a> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_bash::language())
            .expect("Error loading Bash grammar");

        Self {
            source_code,
            parser,
            variables: HashMap::new(),
            errors: Vec::new(),
            config: Config::default(),
        }
    }

    fn handle_comment(&mut self, cursor: &mut TreeCursor) {
        let comment = cursor
            .node()
            .utf8_text(self.source_code.as_bytes())
            .unwrap();
        let range = cursor.node().start_byte()..cursor.node().end_byte();

        if let Some(comment_info) = comment.strip_prefix("#/") {
            if cursor.goto_next_sibling() {
                self.handle_node(
                    cursor,
                    Some(Comment {
                        text: comment_info.trim(),
                        range,
                    }),
                )
            }
        }
    }

    fn infer_type(&self, node: Node) -> BashType {
        match node.kind() {
            "number" => BashType::Integer,
            "word" => BashType::String,
            "string" => BashType::String, // TODO: make this handle other parts ex $()
            _ => todo!(),
        }
    }

    fn type_from_string(&self, input_type: &str) -> BashType {
        match input_type {
            "string" => BashType::String,
            "int" => BashType::Integer,
            "any" => BashType::Any,
            _ => todo!(),
        }
    }

    fn handle_node(&mut self, cursor: &mut TreeCursor, possible_comment: Option<Comment>) {
        match cursor.node().kind() {
            "comment" => self.handle_comment(cursor),
            "variable_assignment" => {
                let name = cursor
                    .goto_first_child()
                    .then(|| cursor.node())
                    .unwrap()
                    .utf8_text(self.source_code.as_bytes())
                    .unwrap();
                cursor.goto_next_sibling();
                let value = cursor.goto_next_sibling().then(|| cursor.node()).unwrap();
                let inferred_type = self.infer_type(value);

                let inferred_location = cursor.node().start_byte()..cursor.node().end_byte();
                let final_type = if let Some(comment) = possible_comment {
                    let suggested_type = self.type_from_string(comment.text);
                    if inferred_type.matches(&suggested_type) {
                        TypeDeclaration {
                            bash_type: suggested_type,
                            range: comment.range,
                            method: Method::Declared,
                        }
                    } else {
                        self.errors.push(
                            Report::build(ReportKind::Error, (), cursor.node().start_byte())
                                .with_message("Types do not match")
                                .with_label(
                                    Label::new(comment.range)
                                        .with_message(format!(
                                            "Type specified as {}",
                                            suggested_type.fg(self.config.specified_color)
                                        ))
                                        .with_color(self.config.specified_color),
                                )
                                .with_label(
                                    Label::new(inferred_location)
                                        .with_message(format!(
                                            "Type later inferred to be {}",
                                            inferred_type.fg(self.config.inferred_color)
                                        ))
                                        .with_color(self.config.inferred_color),
                                )
                                .finish(),
                        );
                        return;
                    }
                } else {
                    TypeDeclaration {
                        bash_type: inferred_type,
                        range: inferred_location,
                        method: Method::Inferred,
                    }
                };
                if let Some(previous_type) = self.variables.get(name) {
                    if !previous_type.bash_type.matches(&final_type.bash_type) {
                        self.errors.push(
                            Report::build(ReportKind::Error, (), cursor.node().start_byte())
                                .with_message(format!(
                                    "Variable {name} defined with different type"
                                ))
                                .with_label(label_from_type_declaration(
                                    previous_type,
                                    &self.config,
                                ))
                                .with_label(label_from_type_declaration(&final_type, &self.config))
                                .finish(),
                        );
                    }
                } else {
                    self.variables.insert(name.to_owned(), final_type);
                }
            }
            _ => (),
        }
    }

    pub fn parse_code(&mut self) {
        let tree = self.parser.parse(self.source_code, None).unwrap();
        let root_node = tree.root_node();

        let mut cursor = root_node.walk();

        loop {
            self.handle_node(&mut cursor, None);

            if cursor.goto_first_child() {
                continue;
            }

            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return;
                }
            }
        }
    }
}

fn label_from_type_declaration(decl_type: &TypeDeclaration, config: &Config) -> Label {
    let (color, description) = match decl_type.method {
        Method::Inferred => (config.inferred_color, "inferred"),
        Method::Declared => (config.specified_color, "declared"),
    };
    Label::new(decl_type.range.clone())
        .with_message(format!(
            "Type {} to be {}",
            description,
            decl_type.bash_type.fg(color)
        ))
        .with_color(color)
}
