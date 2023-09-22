use pest::iterators::Pairs;
use crate::err_handle::ChimeraError;
use crate::frontend::Rule;

#[derive(Debug)]
pub struct ChimeraScriptAST {
    statement: Statement
}

impl ChimeraScriptAST {
    /// Convert
    pub fn from_pairs(pairs: Pairs<Rule>) -> Result<Self, ChimeraError> {
        println!("{:?}", pairs);
        Err(ChimeraError::FailedParseAST("Failed to convert pest tokens to an AST.".to_owned()))
    }
}

#[derive(Debug)]
enum Statement {
    AssignmentExpr(AssignmentExpr),
    AssertCommand,
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