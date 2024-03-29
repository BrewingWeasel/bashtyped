use std::{collections::HashMap, fmt::Display, ops::Range};

use ariadne::{Color, Fmt, Label, Report, ReportKind};
use tree_sitter::{Node, Parser, TreeCursor};

pub struct FileInfo<'src> {
    pub source_code: &'src str,
    parser: Parser,
    pub variables: HashMap<String, TypeDeclaration>,
    pub errors: Vec<Report<'src>>,
    config: Config,
    force: bool,
}

#[derive(Clone, Debug)]
struct Comment {
    text: String,
    range: Range<usize>,
}

#[derive(Debug, PartialEq, Clone, Eq)]
pub enum BashType {
    String,
    Integer,
    Bool,
    Any,
    Or(Box<BashType>, Box<BashType>),
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
        match self {
            BashType::Any => write!(f, "any"),
            BashType::String => write!(f, "string"),
            BashType::Bool => write!(f, "bool"),
            BashType::Integer => write!(f, "int"),
            BashType::Or(t1, t2) => write!(f, "{t1} | {t2}"),
        }
    }
}

impl BashType {
    fn matches(&self, other: &Self) -> bool {
        if let BashType::Or(t1, t2) = self {
            return t1.matches(other) || t2.matches(other);
        }
        if let BashType::Or(t1, t2) = other {
            return t1.matches(self) || t2.matches(self);
        }
        self == &BashType::Any || other == &BashType::Any || self == other
    }

    fn types_from_or(&self) -> Vec<Self> {
        if let BashType::Or(t1, t2) = self {
            let mut v = t1.types_from_or();
            v.extend_from_slice(&t2.types_from_or());
            v
        } else {
            vec![self.clone()]
        }
    }

    fn can_contain(&self, other: &Self) -> bool {
        if let BashType::Or(t1, t2) = self {
            if let BashType::Or(_, _) = other {
                let self_types = self.types_from_or();
                other.types_from_or().iter().all(|v| self_types.contains(v))
            } else {
                t1.matches(other) || t2.matches(other)
            }
        } else {
            self == &BashType::Any || self == other
        }
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

#[derive(Debug, PartialEq, Eq)]
pub struct TypeDeclaration {
    pub range: Range<usize>,
    pub bash_type: BashType,
    pub method: Method,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Method {
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

    fn infer_type(&self, node: Node) -> ParseResult<BashType> {
        match node.kind() {
            "number" => Ok(BashType::Integer),
            "word" => Ok(BashType::String),
            "string" => {
                if node.named_child_count() == 1 {
                    let content = node.child(1).expect("named child count to be one");
                    let inferred_type = if content.kind() == "string_content" {
                        Ok(BashType::String)
                    } else {
                        self.infer_type(content)
                    };
                    inferred_type
                } else {
                    Ok(BashType::String)
                }
            }
            "simple_expansion" => {
                let variable = node.child(1).expect("Variable to have a name");
                let var_name = variable
                    .utf8_text(self.source_code.as_bytes())
                    .map_err(|_| ParseError {
                        err_type: ParseErrType::InvalidUnicode,
                        start: variable.start_byte(),
                        end: variable.end_byte(),
                    })?;
                Ok(self
                    .variables
                    .get(var_name)
                    .ok_or_else(|| ParseError {
                        err_type: ParseErrType::UnknownVariable(var_name.to_owned()),
                        start: variable.start_byte(),
                        end: variable.end_byte(),
                    })?
                    .bash_type
                    .clone())
            }
            _ => {
                println!("{:?}", node.kind());
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
            v => {
                let Some((first, second)) = v.split_once('|') else {
                    todo!();
                };
                BashType::Or(
                    Box::new(self.type_from_string(first)),
                    Box::new(self.type_from_string(second)),
                )
            }
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
                let available_sibling = cursor.goto_next_sibling();
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
                if !available_sibling {
                    return Ok(());
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
                let inferred_type = self.infer_type(cursor.node())?;

                let inferred_location = cursor
                    .node()
                    .prev_sibling()
                    .and_then(|v| v.prev_sibling())
                    .expect("already to have checked previous siblings")
                    .start_byte()..cursor.node().end_byte();

                cursor.goto_parent();
                let inline_type = (cursor
                    .node()
                    .next_sibling()
                    .is_some_and(|node| node.kind() == "comment"))
                .then(|| {
                    cursor.goto_next_sibling();
                    self.handle_comment(cursor)
                })
                .transpose()?
                .flatten();

                let final_type = if let Some(comment) = inline_type.or(possible_comment) {
                    let suggested_type = self.type_from_string(&comment.text);
                    if suggested_type.can_contain(&inferred_type) || self.force {
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
            if !final_type.bash_type.can_contain(&previous_type.bash_type) && !self.force {
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
            decl_type.bash_type.clone().fg(color)
        ))
        .with_color(color)
}

fn combine_ranges(r1: Range<usize>, r2: Range<usize>) -> Range<usize> {
    Range {
        start: r1.start.min(r2.start),
        end: r2.end.max(r1.end),
    }
}
