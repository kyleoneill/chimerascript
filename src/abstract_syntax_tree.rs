use pest::iterators::{Pair, Pairs};
use crate::err_handle::ChimeraError;
use crate::err_handle::ChimeraError::FailedParseAST;
use crate::frontend::Rule;

#[derive(Debug)]
pub struct ChimeraScriptAST {
    statement: Statement
}

impl ChimeraScriptAST {
    /// Convert Pest tokens into an abstract syntax tree.
    pub fn from_pairs(pairs: Pairs<Rule>) -> Result<Self, ChimeraError> {
        // There should only be one Pair<Rule> here, do I even need a loop or should I just get
        // the first/next out of the iter?
        for pair in pairs {
            let statement = ChimeraScriptAST::parse_rule_to_statement(pair)?;
            return Ok(ChimeraScriptAST { statement })
        }
        Err(FailedParseAST("did not get any Rule pairs".to_owned()))
    }

    fn parse_rule_to_statement(pair: Pair<Rule>) -> Result<Statement, ChimeraError> {
        // TODO: REMOVE ME
        println!("{:#?}", pair);
        match pair.as_rule() {
            Rule::Statement => {
                // The outermost layer is going to be a Rule::Statement, we want to just into_inner
                // it and get to actual parsing
                match pair.into_inner().peek() {
                    Some(inner) => ChimeraScriptAST::parse_rule_to_statement(inner),
                    None => Err(FailedParseAST("Rule::Statement variant did not contain inner token".to_owned()))
                }
            }
            Rule::AssertCommand => {
                // An AssertCommand inner is going to contain
                // 1. Optional Negation
                // 2. AssertSubCommand
                // 3. Value
                // 4. Value
                // 5. Optional QuoteString
                let mut pairs = pair.into_inner();

                // Peek ahead to see if our inner contains an optional Negation
                let negate_assertion = match pairs.peek() {
                    Some(next) => if next.as_rule() == Rule::Negation {Ok(true)} else {Ok(false)},
                    None => Err(FailedParseAST("Rule::AssertCommand contained no inner values".to_owned()))
                }?;
                // peek() does not move the iterator position, so if we did have a negation then we
                // need to move the iterator ahead by one position
                if negate_assertion {
                    let _ = pairs.next();
                }

                // Get the sub-command
                let next_subcommand = pairs.next().ok_or_else(|| FailedParseAST("ran out of tokens when getting assertion subcommand".to_owned()))?;
                if next_subcommand.as_rule() != Rule::AssertSubCommand {return Err(FailedParseAST("Rule::AssertCommand inner tokens missing a Rule::AssertSubcommand".to_owned()))}
                let subcommand = match next_subcommand.as_span().as_str() {
                    "EQUALS" => AssertSubCommand::Equals,
                    "GTE" => AssertSubCommand::GTE,
                    "GT" => AssertSubCommand::GT,
                    "LTE" => AssertSubCommand::LTE,
                    "LT" => AssertSubCommand::LT,
                    "STATUS" => AssertSubCommand::STATUS,
                    _ => return Err(FailedParseAST("Rule::AssertSubcommand contained an invalid value".to_owned()))
                };

                // Get the first value we're asserting with
                let next_value = pairs.next().ok_or_else(|| FailedParseAST("ran out of tokens when getting first assertion Value".to_owned()))?;
                if next_value.as_rule() != Rule::Value {return Err(FailedParseAST("Rule::AssertCommand inner tokens missing a Rule::Value".to_owned()))};
                let first_value = ChimeraScriptAST::parse_rule_to_value(&next_value)?;

                // Get the second value we're asserting with
                let next_second_value = pairs.next().ok_or_else(|| FailedParseAST("ran out of tokens when getting second assertion Value".to_owned()))?;
                if next_second_value.as_rule() != Rule::Value {return Err(FailedParseAST("Rule::AssertCommand inner tokens missing a Rule::Value".to_owned()))};
                let second_value = ChimeraScriptAST::parse_rule_to_value(&next_second_value)?;

                // Check for an optional QuoteString which represents an assertion failure message
                let assertion_failure_message = match pairs.peek() {
                    Some(next) => {
                        if next.as_rule() != Rule::QuoteString {return Err(FailedParseAST("expected to be given a Rule::QuoteString token meant to be used as an assertion error message but got the wrong rule type".to_owned()))}
                        Some(next.as_str().to_owned())
                    }
                    None => None
                };

                Ok(Statement::AssertCommand(AssertCommand {
                    negate_assertion,
                    subcommand,
                    left_value: first_value,
                    right_value: second_value,
                    error_message: assertion_failure_message
                }))
            },
            Rule::AssignmentExpr => {
                Err(ChimeraError::FailedParseAST("NEED TO FINISH PARSE_VALUES ASSI".to_owned()))
            },
            Rule::PrintCommand => {
                Err(ChimeraError::FailedParseAST("NEED TO FINISH PARSE_VALUES PRI".to_owned()))
            },
            Rule::Expression => {
                Err(ChimeraError::FailedParseAST("NEED TO FINISH PARSE_VALUES EXP".to_owned()))
            },
            _ => { Err(FailedParseAST("NEED TO FINISH PARSE_VALUES".to_owned())) }
        }
    }

    fn parse_rule_to_value(pair: &Pair<Rule>) -> Result<Value, ChimeraError> {
        if pair.as_rule() != Rule::Value {return Err(FailedParseAST("expected a Rule::Value but got a different Rule variant".to_owned()))};
        let inner = pair.clone().into_inner().peek().ok_or_else(|| return FailedParseAST("Rule::Value did not contain an inner".to_owned()))?;
        return match inner.as_rule() {
            Rule::LiteralValue => {
                // A literal can be an int, a bool, or a string. Check to see if it's an int
                // or bool before setting it to be a string
                match inner.as_str().parse::<i32>() {
                    Ok(res) => return Ok(Value::Literal(Literal::Int(res))),
                    Err(_) => ()
                };
                let res: Value = match inner.as_str() {
                    "true" => Value::Literal(Literal::Bool(true)),
                    "false" => Value::Literal(Literal::Bool(false)),
                    _ => Value::Literal(Literal::Str(inner.as_str().to_owned())),
                };
                Ok(res)
            },
            Rule::VariableValue => {
                // A VariableValue is stored as a string and looks like (this) or (this.that)
                Ok(Value::Variable(inner.as_str().to_owned()))
            },
            _ => { Err(FailedParseAST("got an invalid Rule variant while parsing the inner of a Rule::Value".to_owned()))}
        }
    }
}

#[derive(Debug)]
enum Statement {
    AssignmentExpr(AssignmentExpr),
    AssertCommand(AssertCommand),
    PrintCommand(Value),
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
    negate_assertion: bool,
    subcommand: AssertSubCommand,
    left_value: Value,
    right_value: Value,
    error_message: Option<String>
}

#[derive(Debug)]
enum Value {
    Literal(Literal),
    Variable(String)
}

#[derive(Debug)]
enum AssertSubCommand {
    Equals,
    GTE,
    GT,
    LTE,
    LT,
    STATUS
}

#[derive(Debug)]
struct LiteralExpression {
    literal_value: Literal
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
    rhs: Literal
}

#[derive(Debug)]
struct KeyValuePair {
    key: String,
    value: Literal
}

#[derive(Debug)]
enum Literal {
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