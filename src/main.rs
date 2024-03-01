use std::{collections::HashMap, fmt::Display, ops::Range};

use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use tree_sitter::{Parser, TreeCursor};

fn main() {
    let source_code = r#"# other thing
do_thing() {
    #/ string
    value="lol"

    #[force]
    #/ int
    fakeint="lol"

    #[set_var(input, int)]
    read input

    #/ int
    other_int=$input

    value=1 #/ int

    real_var=1

    #/ string
    cool_stuff="it is $real_var"
}"#;

    let mut info = FileInfo::new(source_code);

    info.parse_code();

    for error in info.errors {
        error
            .print(Source::from(info.source_code))
            .expect("comment printing to work");
    }
}

struct FileInfo<'src> {
    source_code: &'src str,
    parser: Parser,
    variables: HashMap<String, TypeDeclaration>,
    errors: Vec<Report<'src>>,
    config: Config,
    force: bool,
}

#[derive(Clone)]
struct Comment {
    text: String,
    range: Range<usize>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum BashType {
    String,
    Integer,
    Bool,
    Any,
}

struct ParseError {
    err_type: ParseErrType,
    start: usize,
    end: usize,
}

enum ParseErrType {
    MissingArgument { expected: usize, received: usize },
    InvalidUnicode,
    UnknownVariable(String),
}

impl Display for ParseErrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUnicode => write!(f, "Invalid unicode"),
            Self::MissingArgument {
                expected: e,
                received: r,
            } => write!(f, "Expected {e} arguments, but found {r}"),
            Self::UnknownVariable(var_name) => write!(f, "Found unknown variable {var_name}"),
        }
    }
}

type ParseResult<T> = std::result::Result<T, ParseError>;

impl Display for BashType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BashType::Any => "any",
                BashType::String => "string",
                BashType::Bool => "bool",
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
    parse_err_color: Color,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            specified_color: Color::Blue,
            inferred_color: Color::Magenta,
            parse_err_color: Color::Red,
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
            force: false,
        }
    }

    fn handle_comment(&self, cursor: &mut TreeCursor) -> ParseResult<Option<Comment>> {
        let comment = cursor
            .node()
            .utf8_text(self.source_code.as_bytes())
            .map_err(|_| ParseError {
                err_type: ParseErrType::InvalidUnicode,
                start: cursor.node().start_byte(),
                end: cursor.node().end_byte(),
            })?;
        let range = cursor.node().start_byte()..cursor.node().end_byte();

        Ok(comment
            .strip_prefix("#/")
            .or(comment.strip_prefix("#["))
            .map(|comment_info| Comment {
                text: comment_info.trim().to_owned(),
                range,
            }))
    }

    fn infer_type(&self, cursor: &mut TreeCursor) -> ParseResult<BashType> {
        match cursor.node().kind() {
            "number" => Ok(BashType::Integer),
            "word" => Ok(BashType::String),
            "string" => {
                if cursor.node().named_child_count() == 1 {
                    assert!(cursor.goto_first_child(), "named_child_count is one");
                    cursor.goto_next_sibling();
                    if cursor.node().kind() == "string_content" {
                        Ok(BashType::String)
                    } else {
                        self.infer_type(cursor)
                    }
                } else {
                    Ok(BashType::String)
                }
            }
            "simple_expansion" => {
                cursor.goto_first_child();
                let var_name = cursor
                    .goto_next_sibling()
                    .then(|| {
                        cursor
                            .node()
                            .utf8_text(self.source_code.as_bytes())
                            .map_err(|_| ParseError {
                                err_type: ParseErrType::InvalidUnicode,
                                start: cursor.node().start_byte(),
                                end: cursor.node().end_byte(),
                            })
                    })
                    .expect("Variable to have a name")?;
                Ok(self
                    .variables
                    .get(var_name)
                    .ok_or_else(|| ParseError {
                        err_type: ParseErrType::UnknownVariable(var_name.to_owned()),
                        start: cursor.node().start_byte(),
                        end: cursor.node().end_byte(),
                    })?
                    .bash_type)
            }
            _ => {
                println!("{:?}", cursor.node().kind());
                todo!()
            }
        }
    }

    fn type_from_string(&self, input_type: &str) -> BashType {
        match input_type.trim() {
            "string" => BashType::String,
            "int" => BashType::Integer,
            "bool" => BashType::Bool,
            "any" => BashType::Any,
            _ => todo!(),
        }
    }

    fn handle_node(
        &mut self,
        cursor: &mut TreeCursor,
        possible_comment: Option<Comment>,
    ) -> ParseResult<()> {
        match cursor.node().kind() {
            "comment" => {
                let possible_comment = self.handle_comment(cursor)?.to_owned();
                cursor.goto_next_sibling();
                if let Some(command) = possible_comment
                    .as_ref()
                    .and_then(|v| v.text.strip_suffix(']'))
                {
                    match command {
                        "force" => self.force = true,
                        func_command => {
                            if let Some(info) = func_command
                                .strip_prefix("set_var(")
                                .and_then(|conts| conts.strip_suffix(')'))
                            {
                                let args = info.split(',').collect::<Vec<_>>();
                                if args.len() != 2 {
                                    return Err(ParseError {
                                        err_type: ParseErrType::MissingArgument {
                                            expected: 2,
                                            received: args.len(),
                                        },
                                        start: cursor.node().start_byte(),
                                        end: cursor.node().end_byte(),
                                    });
                                }
                                let final_type = TypeDeclaration {
                                    range: cursor.node().start_byte()..cursor.node().end_byte(),
                                    bash_type: self.type_from_string(args[1]),
                                    method: Method::Declared,
                                };
                                self.set_variable(args[0], final_type, cursor);
                            }
                        }
                    }
                }
                self.handle_node(cursor, possible_comment)?;
            }
            "variable_assignment" => {
                cursor.goto_first_child();
                let name = cursor
                    .node()
                    .utf8_text(self.source_code.as_bytes())
                    .map_err(|_| ParseError {
                        err_type: ParseErrType::InvalidUnicode,
                        start: cursor.node().start_byte(),
                        end: cursor.node().end_byte(),
                    })?;
                cursor.goto_next_sibling();
                cursor.goto_next_sibling();
                let inferred_type = self.infer_type(cursor)?;

                let inferred_location = cursor.node().start_byte()..cursor.node().end_byte();

                cursor.goto_parent();
                let inline_type = (cursor.goto_next_sibling() && cursor.node().kind() == "comment")
                    .then(|| self.handle_comment(cursor))
                    .transpose()?
                    .flatten();

                let final_type = if let Some(comment) = inline_type.or(possible_comment) {
                    let suggested_type = self.type_from_string(&comment.text);
                    if inferred_type.matches(&suggested_type) || self.force {
                        TypeDeclaration {
                            bash_type: suggested_type,
                            range: combine_ranges(comment.range, inferred_location),
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
                                            "Type inferred to be {}",
                                            inferred_type.fg(self.config.inferred_color)
                                        ))
                                        .with_color(self.config.inferred_color),
                                )
                                .finish(),
                        );
                        return Ok(());
                    }
                } else {
                    TypeDeclaration {
                        bash_type: inferred_type,
                        range: inferred_location,
                        method: Method::Inferred,
                    }
                };
                self.set_variable(name, final_type, cursor);
            }
            _ => (),
        }
        Ok(())
    }

    pub fn parse_code(&mut self) {
        let tree = self
            .parser
            .parse(self.source_code, None)
            .expect("treesitter to parse valid code");
        let root_node = tree.root_node();

        let mut cursor = root_node.walk();

        loop {
            if let Err(e) = self.handle_node(&mut cursor, None) {
                self.errors.push(
                    Report::build(ReportKind::Error, (), cursor.node().start_byte())
                        .with_message("Error while parsing comment")
                        .with_label(
                            Label::new(e.start..e.end)
                                .with_message(e.err_type)
                                .with_color(self.config.parse_err_color),
                        )
                        .finish(),
                );
            }

            if cursor.goto_first_child() {
                continue;
            }

            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return;
                }
            }
            self.force = false;
        }
    }

    fn set_variable(&mut self, name: &str, final_type: TypeDeclaration, cursor: &TreeCursor) {
        if let Some(previous_type) = self.variables.get(name) {
            if !previous_type.bash_type.matches(&final_type.bash_type) && !self.force {
                self.errors.push(
                    Report::build(ReportKind::Error, (), cursor.node().start_byte())
                        .with_message(format!("Variable {name} defined with different type"))
                        .with_label(label_from_type_declaration(
                            previous_type,
                            &self.config,
                            false,
                        ))
                        .with_label(label_from_type_declaration(&final_type, &self.config, true))
                        .finish(),
                );
            }
        } else {
            self.variables.insert(name.to_owned(), final_type);
        }
    }
}

fn label_from_type_declaration(
    decl_type: &TypeDeclaration,
    config: &Config,
    is_later: bool,
) -> Label {
    let (color, description) = match decl_type.method {
        Method::Inferred => (config.inferred_color, "inferred"),
        Method::Declared => (config.specified_color, "declared"),
    };
    Label::new(decl_type.range.clone())
        .with_message(format!(
            "Type {}{} to be {}",
            is_later.then_some("later ").unwrap_or_default(),
            description,
            decl_type.bash_type.fg(color)
        ))
        .with_color(color)
}

fn combine_ranges(r1: Range<usize>, r2: Range<usize>) -> Range<usize> {
    Range {
        start: r1.start.min(r2.start),
        end: r2.end.max(r1.end),
    }
}
