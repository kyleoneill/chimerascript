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
                let left_value = ChimeraScriptAST::parse_rule_to_value(next_value)?;

                // Get the second value we're asserting with
                let next_second_value = pairs.next().ok_or_else(|| FailedParseAST("ran out of tokens when getting second assertion Value".to_owned()))?;
                if next_second_value.as_rule() != Rule::Value {return Err(FailedParseAST("Rule::AssertCommand inner tokens missing a Rule::Value".to_owned()))};
                let right_value = ChimeraScriptAST::parse_rule_to_value(next_second_value)?;

                // Check for an optional QuoteString which represents an assertion failure message
                let error_message = match pairs.peek() {
                    Some(next) => {
                        if next.as_rule() != Rule::QuoteString {return Err(FailedParseAST("expected to be given a Rule::QuoteString token meant to be used as an assertion error message but got the wrong rule type".to_owned()))}
                        Some(next.as_str().to_owned())
                    }
                    None => None
                };

                Ok(Statement::AssertCommand(AssertCommand {
                    negate_assertion,
                    subcommand,
                    left_value,
                    right_value,
                    error_message
                }))
            },
            Rule::AssignmentExpr => {
                // An AssignmentExpr is going to contain
                // 1. A string representing a variable name
                // 2. An expression
                let mut pairs = pair.into_inner();

                let next_str = pairs.next().ok_or_else(|| return FailedParseAST("ran out of tokens when getting variable name of an AssignmentExpr".to_owned()))?;
                if next_str.as_rule() != Rule::VariableNameAssignment {return Err(FailedParseAST("Rule::AssignmentExpr did not contain a Rule::VariableNameAssignment to use as a variable name".to_owned()))}
                let var_name = next_str.as_str().to_owned();

                let next_expr = pairs.next().ok_or_else(|| return FailedParseAST("ran out of tokens when getting expression out of an AssignmentExpr".to_owned()))?;
                if next_expr.as_rule() != Rule::Expression {return Err(FailedParseAST("Rule::AssignmentExpr did not contain a Rule::Expression inner".to_owned()))}
                let expression = ChimeraScriptAST::parse_rule_to_expression(next_expr)?;
                Ok(Statement::AssignmentExpr(AssignmentExpr {
                    var_name,
                    expression
                }))
            },
            Rule::PrintCommand => {
                // A PrintCommand is going to contain
                // 1. A value to print
                let mut pairs = pair.into_inner();

                let next_value = pairs.next().ok_or_else(|| return FailedParseAST("ran out of tokens when getting a value out of a PrintCommand".to_owned()))?;
                let next_value = ChimeraScriptAST::parse_rule_to_value(next_value)?;
                Ok(Statement::PrintCommand(next_value))
            },
            Rule::Expression => {
                // Moved to shared method as AssignmentExpr also needs to construct an Expression
                let expression = ChimeraScriptAST::parse_rule_to_expression(pair)?;
                Ok(Statement::Expression(expression))
            },
            _ => { Err(FailedParseAST("got an invalid Rule variant while constructing a Statement".to_owned())) }
        }
    }

    fn parse_rule_to_value(pair: Pair<Rule>) -> Result<Value, ChimeraError> {
        if pair.as_rule() != Rule::Value {return Err(FailedParseAST("expected a Rule::Value but got a different Rule variant".to_owned()))};
        let inner = pair.into_inner().peek().ok_or_else(|| return FailedParseAST("Rule::Value did not contain an inner".to_owned()))?;
        return match inner.as_rule() {
            Rule::LiteralValue => {
                let literal_value = ChimeraScriptAST::parse_rule_to_literal_value(inner)?;
                Ok(Value::Literal(literal_value))
            },
            Rule::VariableValue => {
                // A VariableValue is stored as a string and looks like (this) or (this.that)
                Ok(Value::Variable(inner.as_str().to_owned()))
            },
            _ => { Err(FailedParseAST("got an invalid Rule variant while parsing the inner of a Rule::Value".to_owned()))}
        }
    }

    fn parse_rule_to_literal_value(pair: Pair<Rule>) -> Result<Literal, ChimeraError> {
        // A literal can be an int, a bool, or a string. Check to see if it's an int
        // or bool before setting it to be a string
        match pair.as_str().parse::<i32>() {
            Ok(res) => return Ok(Literal::Int(res)),
            Err(_) => ()
        };
        let res = match pair.as_str() {
            "true" => Literal::Bool(true),
            "false" => Literal::Bool(false),
            _ => Literal::Str(pair.as_str().to_owned()),
        };
        Ok(res)
    }

    fn parse_rule_to_expression(pair: Pair<Rule>) -> Result<Expression, ChimeraError> {
        // An Expression is going to contain EITHER
        // a. A LiteralValue which will hold some literal
        // b. An HttpCommand which will contain
        //   1. An Http verb
        //   2. The slash path of the Http command
        //   3. Optional list of HttpAssignment, which look like `field="value"`
        //   4. Optional list of KeyValuePair, which look like `timeout=>60`
        if pair.as_rule() != Rule::Expression {return Err(FailedParseAST("tried to parse a non-Expression rule as an Expression".to_owned()))}
        let mut expression_pairs = pair.into_inner();

        let first_token = expression_pairs.next().ok_or_else(|| return FailedParseAST("did not get any tokens inside a Rule::Expression".to_owned()))?;
        match first_token.as_rule() {
            Rule::LiteralValue => {
                let literal_value = ChimeraScriptAST::parse_rule_to_literal_value(first_token)?;
                return Ok(Expression::LiteralExpression(literal_value))
            },
            Rule::HttpCommand => {
                let mut http_pairs = first_token.into_inner();

                let verb_token = http_pairs.next().ok_or_else(|| return FailedParseAST("did not get any tokens inside a Rule::HttpCommand".to_owned()))?;
                if verb_token.as_rule() != Rule::HTTPVerb {return Err(FailedParseAST("Rule::HttpCommand did not contain a Rule::HttpVerb".to_owned()))}
                let verb = match verb_token.as_str() {
                    "GET" => HTTPVerb::GET,
                    "PUT" => HTTPVerb::PUT,
                    "POST" => HTTPVerb::POST,
                    "DELETE" => HTTPVerb::DELETE,
                    _ => return Err(FailedParseAST("got an invalid value for an Http verb while parsing an expression".to_owned()))
                };

                let path_token = http_pairs.next().ok_or_else(|| return FailedParseAST("ran out of tokens when getting a Rule::Path for a Rule::HttpCommand".to_string()))?;
                if path_token.as_rule() != Rule::Path {return Err(FailedParseAST("expected to get a Rule::Path token while parsing a Rule::HttpCommand but did not get one".to_owned()))}
                let path = path_token.as_str().to_owned();

                // Peek ahead and iterate over the next pairs to get all of the HttpAssignment ones
                let mut http_assignments: Vec<HttpAssignment> = Vec::new();
                while http_pairs.peek().is_some() && http_pairs.peek().unwrap().as_rule() == Rule::HttpAssignment {
                    let mut http_assignment_pairs = http_pairs.next().unwrap().into_inner();

                    let assignment_token = http_assignment_pairs.next().ok_or_else(|| return FailedParseAST("failed to get another token when looking for a VariableNameAssignment when parsing an HttpAssignment".to_owned()))?;
                    if assignment_token.as_rule() != Rule::VariableNameAssignment {return Err(FailedParseAST("failed to get a VariableNameAssignment when parsing an HttpAssignment".to_owned()))}
                    let lhs = assignment_token.as_str().to_owned();

                    let value_token = http_assignment_pairs.next().ok_or_else(|| return FailedParseAST("failed to get a Value token while parsing an HttpAssignment".to_owned()))?;
                    let rhs = ChimeraScriptAST::parse_rule_to_value(value_token)?;

                    let http_assignment = HttpAssignment {
                        lhs,
                        rhs
                    };
                    http_assignments.push(http_assignment);
                }

                // Peek ahead and iterate over the next pairs to get all of the KeyValuePair ones
                let mut key_val_pairs: Vec<KeyValuePair> = Vec::new();
                while http_pairs.peek().is_some() && http_pairs.peek().unwrap().as_rule() == Rule::KeyValuePair {
                    let mut key_value_pairs = http_pairs.next().unwrap().into_inner();

                    let assignment_token = key_value_pairs.next().ok_or_else(|| return FailedParseAST("failed to get another token when looking for a VariableNameAssignment when parsing a KeyValuePair".to_owned()))?;
                    if assignment_token.as_rule() != Rule::VariableNameAssignment {return Err(FailedParseAST("failed to get a VariableNameAssignment when parsing a KeyValuePair".to_owned()))}
                    let key = assignment_token.as_str().to_owned();

                    let value_token = key_value_pairs.next().ok_or_else(|| return FailedParseAST("failed to get a Value token while parsing a KeyValuePair".to_owned()))?;
                    let value = ChimeraScriptAST::parse_rule_to_value(value_token)?;

                    let key_value = KeyValuePair {
                        key,
                        value
                    };
                    key_val_pairs.push(key_value);
                }
                Ok(Expression::HttpCommand(HttpCommand {
                    verb,
                    path,
                    http_assignments,
                    key_val_pairs
                }))
            },
            _ => {return Err(FailedParseAST("Rule::Expression contained an invalid inner rule, expected to only get LiteralValue or HttpCommand".to_owned()))}
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
    LiteralExpression(Literal),
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
struct HttpCommand {
    verb: HTTPVerb,
    path: String,
    http_assignments: Vec<HttpAssignment>,
    key_val_pairs: Vec<KeyValuePair>
}

#[derive(Debug)]
struct HttpAssignment {
    lhs: String,
    rhs: Value
}

#[derive(Debug)]
struct KeyValuePair {
    key: String,
    value: Value
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
