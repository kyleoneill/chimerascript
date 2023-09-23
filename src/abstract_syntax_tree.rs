use pest::iterators::{Pair, Pairs};
use crate::err_handle::ChimeraError;
use crate::frontend::Rule;

#[derive(Debug)]
pub struct ChimeraScriptAST {
    statement: Statement
}

impl ChimeraScriptAST {
    /// Convert Pest tokens into an abstract syntax tree.
    pub fn from_pairs(pairs: Pairs<Rule>) -> Result<Self, ChimeraError> {
        // This might be hacky but every valid Pairs should be a list of a single Pair with a Statement rule
        // We want to discard the outermost Pair and begin parsing on what the Statement contains
        // TODO: Verify that the outermost Pair is a Statement before discarding it like this.
        //       I don't think it's possible to not be a statement but good error handling is good
        for pair in pairs {
            let inner = pair.into_inner();
            let statement = ChimeraScriptAST::parse_value(inner.peek().unwrap())?;
            return Ok(ChimeraScriptAST { statement })
        }
        Err(ChimeraError::FailedParseAST("Failed to convert pest tokens to an AST.".to_owned()))
    }

    fn parse_value(pair: Pair<Rule>) -> Result<Statement, ChimeraError> {
        match pair.as_rule() {
            Rule::Statement => Err(ChimeraError::FailedParseAST("Should not have gotten a statement Rule enum while parsing values".to_owned())),
            _ => { Err(ChimeraError::FailedParseAST("NEED TO FINISH PARSE_VALUES".to_owned())) }
        }
    }
}

#[derive(Debug)]
enum Statement {
    AssignmentExpr(AssignmentExpr),
    AssertCommand(AssertCommand),
    PrintCommand(LiteralOrVariable),
    Expression(Expression)
}

#[derive(Debug)]
struct AssignmentExpr {
    var_name: String,
    expression: Expression
}

#[derive(Debug)]
enum Expression {
    LiteralExpression(LiteralExpression),
    HttpCommand(HttpCommand)
}

#[derive(Debug)]
struct AssertCommand {
    inverted: bool,
    subcommand: AssertSubCommand,
    left_value: LiteralOrVariable,
    right_value: LiteralOrVariable,
    error_message: Option<String>
}

#[derive(Debug)]
enum LiteralOrVariable {
    LiteralValue(LiteralValue),
    VariableValue(String)
}

#[derive(Debug)]
enum AssertSubCommand {
    Not,
    Equals,
    GTE,
    GT,
    LTE,
    LT,
    STATUS
}

#[derive(Debug)]
struct LiteralExpression {
    literal_value: LiteralValue
}

#[derive(Debug)]
struct HttpCommand {
    verb: HTTPVerb,
    path: String,
    http_assignments: Vec<HttpAssignment>,
    key_val_pairs: Vec<KeyValuePair>
}

#[derive(Debug)]
struct HttpAssignment {
    lhs: String,
    rhs: LiteralValue
}

#[derive(Debug)]
struct KeyValuePair {
    key: String,
    value: LiteralValue
}

#[derive(Debug)]
enum LiteralValue {
    Str(String),
    Int(i32),
    Bool(bool)
}

#[derive(Debug)]
enum HTTPVerb {
    GET,
    PUT,
    POST,
    DELETE
}