use std::collections::HashMap;
use std::fmt::Formatter;
use pest::iterators::{Pair, Pairs};
use serde_json::Value as SerdeJsonValue;
use crate::err_handle::{ChimeraCompileError, ChimeraRuntimeFailure, VarTypes};
use crate::err_handle::ChimeraCompileError::FailedParseAST;
use crate::frontend::{Rule, Context};

#[derive(Debug)]
pub struct ChimeraScriptAST {
    pub statement: Statement
}

impl ChimeraScriptAST {
    /// Convert Pest tokens into an abstract syntax tree.
    pub fn from_pairs(pairs: Pairs<Rule>) -> Result<Self, ChimeraCompileError> {
        // There should only be one Pair<Rule> here. Do I even need a loop or should I just get
        // the first/next out of the iter?
        for pair in pairs {
            let statement = ChimeraScriptAST::parse_rule_to_statement(pair)?;
            return Ok(ChimeraScriptAST { statement })
        }
        Err(FailedParseAST("did not get any Rule pairs".to_owned()))
    }

    fn parse_rule_to_statement(pair: Pair<Rule>) -> Result<Statement, ChimeraCompileError> {
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
                    "EQUALS" => AssertSubCommand::EQUALS,
                    "GTE" => AssertSubCommand::GTE,
                    "GT" => AssertSubCommand::GT,
                    "LTE" => AssertSubCommand::LTE,
                    "LT" => AssertSubCommand::LT,
                    "STATUS" => AssertSubCommand::STATUS,
                    "LENGTH" => AssertSubCommand::LENGTH,
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

    fn parse_rule_to_variable_name(pair: Pair<Rule>) -> Result<String, ChimeraCompileError> {
        if pair.as_rule() != Rule::VariableValue {return Err(FailedParseAST("Expected a VariableValue but got a different rule".to_owned()))}
        let var_name_str = pair.as_str();
        // We want to remove the opening and closing parenthesis from the var name
        Ok(var_name_str[1..var_name_str.len() - 1].to_owned())
    }

    fn parse_rule_to_value(pair: Pair<Rule>) -> Result<Value, ChimeraCompileError> {
        if pair.as_rule() != Rule::Value {return Err(FailedParseAST("expected a Rule::Value but got a different Rule variant".to_owned()))};
        let inner = pair.into_inner().peek().ok_or_else(|| return FailedParseAST("Rule::Value did not contain an inner".to_owned()))?;
        return match inner.as_rule() {
            Rule::LiteralValue => {
                let literal_value = ChimeraScriptAST::parse_rule_to_literal_value(inner)?;
                Ok(Value::Literal(literal_value))
            },
            Rule::VariableValue => Ok(Value::Variable(ChimeraScriptAST::parse_rule_to_variable_name(inner)?)),
            _ => { Err(FailedParseAST("got an invalid Rule variant while parsing the inner of a Rule::Value".to_owned()))}
        }
    }

    fn parse_rule_to_literal_value(pair: Pair<Rule>) -> Result<Literal, ChimeraCompileError> {
        // A literal can be an int, a bool, or a string. Check to see if it's an int
        // or bool before setting it to be a string
        if pair.as_rule() != Rule::LiteralValue { return Err(FailedParseAST("Expected a Rule::LiteralValue but got a different Rule variant".to_owned())) }
        match pair.as_str().parse::<i64>() {
            Ok(res) => return Ok(Literal::Int(res)),
            Err(_) => ()
        };
        let res = match pair.as_str() {
            "true" => Literal::Bool(true),
            "false" => Literal::Bool(false),
            "null" => Literal::Null,
            _ => {
                // TODO: Refactor this to be a bit more readable and match how the rest of the token
                //       parsing is being handled. This is currently calling into_inner to grab a QuoteString,
                //       into_inner again to grab an AggregatedString, and then getting the str value
                //       of the AggregatedString
                match pair.into_inner().peek() {
                    Some(first_inner) => {
                        match first_inner.into_inner().peek() {
                            Some(second_inner) => {
                                Literal::Str(second_inner.as_str().to_owned())
                            },
                            None => return Err(FailedParseAST("Failed to get tokens for a Literal String value".to_owned()))
                        }
                    },
                    None => return Err(FailedParseAST("Failed to get tokens for a Literal String value".to_owned()))
                }
            },
        };
        Ok(res)
    }

    fn parse_rule_to_expression(pair: Pair<Rule>) -> Result<Expression, ChimeraCompileError> {
        // An Expression is going to contain
        // a. A LiteralValue which will hold some literal
        // b. An HttpCommand which will contain
        //   1. An Http verb
        //   2. The slash path of the Http command
        //   3. Optional list of HttpAssignment, which look like `field="value"`
        //   4. Optional list of KeyValuePair, which look like `timeout=>60`
        // c. A LIST expression
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
            Rule::ListExpression => {
                let mut list_paris = first_token.into_inner();
                let list_expression_kind_token = list_paris.next().ok_or_else(|| return FailedParseAST("Did not get any tokens inside a ListExpression".to_owned()))?;
                match list_expression_kind_token.as_rule() {
                    Rule::ListNew => {
                        let mut list_new_pairs = list_expression_kind_token.into_inner();
                        let mut list_value_token = list_new_pairs.next().ok_or_else(|| return FailedParseAST("Did not get any tokens inside a ListNew".to_owned()))?;
                        let mut list_values: Vec<Value> = Vec::new();
                        while list_value_token.as_rule() == Rule::CommaSeparatedValues {
                            let mut inner = list_value_token.into_inner();
                            let literal_token = inner.next().ok_or_else(|| return FailedParseAST("Did not get an inner token when parsing a CommaSeparatedValues, which should always contain a Literal".to_owned()))?;
                            let value = ChimeraScriptAST::parse_rule_to_value(literal_token)?;
                            list_values.push(value);
                            list_value_token = list_new_pairs.next().ok_or_else(|| return FailedParseAST("Ran out of tokens when parsing CommaSeparatedValues. This token stream should always end with a Literal".to_owned()))?;
                        }
                        let value = ChimeraScriptAST::parse_rule_to_value(list_value_token)?;
                        list_values.push(value);
                        Ok(Expression::ListExpression(ListExpression::New(list_values)))
                    },
                    Rule::ListCommandExpr => {
                        let mut list_command_expr_tokens = list_expression_kind_token.into_inner();
                        // Save the op pair to parse last as it might depend on the third token to set its value
                        let command_token = list_command_expr_tokens.next().ok_or_else(|| return FailedParseAST("Ran out of tokens when parsing ListCommandExpr to get a ListCommand".to_owned()))?;
                        let variable_name_token = list_command_expr_tokens.next().ok_or_else(|| return FailedParseAST("Ran out of tokens when parsing ListCommandExpr to get a VariableValue".to_owned()))?;
                        let list_name = ChimeraScriptAST::parse_rule_to_variable_name(variable_name_token)?;
                        let list_command = match list_command_expr_tokens.next() {
                            Some(value_token) => {
                                let value = ChimeraScriptAST::parse_rule_to_value(value_token)?;
                                match command_token.as_str() {
                                    "APPEND" => ListCommand { list_name, operation: ListCommandOperations::MutateOperations(MutateListOperations::Append(value)) },
                                    "REMOVE" => ListCommand { list_name, operation: ListCommandOperations::MutateOperations(MutateListOperations::Remove(value)) },
                                    _ => return Err(FailedParseAST("Got an invalid list command while parsing a ListCommandExpr with an additional argument".to_owned()))
                                }
                            },
                            None => {
                                match command_token.as_str() {
                                    "LENGTH" => ListCommand { list_name, operation: ListCommandOperations::Length },
                                    _ => return Err(FailedParseAST("Got an invalid list command while parsing a ListCommandExpr".to_owned()))
                                }
                            }
                        };
                        Ok(Expression::ListExpression(ListExpression::ListArgument(list_command)))
                    },
                    _ => { return Err(FailedParseAST("ListExpression contained an invalid inner rule".to_owned())) }
                }
            },
            _ => { return Err(FailedParseAST("Expression contained an invalid inner rule".to_owned())) }
        }
    }
}

#[derive(Debug)]
pub enum Statement {
    AssignmentExpr(AssignmentExpr),
    AssertCommand(AssertCommand),
    PrintCommand(Value),
    Expression(Expression)
}

#[derive(Debug)]
pub struct AssignmentExpr {
    pub var_name: String,
    pub expression: Expression
}

#[derive(Debug)]
pub enum Expression {
    LiteralExpression(Literal),
    HttpCommand(HttpCommand),
    ListExpression(ListExpression)
}

#[derive(Debug)]
pub struct AssertCommand {
    pub negate_assertion: bool,
    pub subcommand: AssertSubCommand,
    pub left_value: Value,
    pub right_value: Value,
    pub error_message: Option<String>
}

impl From<Statement> for AssertCommand {
    fn from(value: Statement) -> Self {
        match value {
            Statement::AssertCommand(assert_cmd) => assert_cmd,
            _ => panic!("tried to use a Statement as an AssertCommand when it was not one")
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Literal(Literal),
    Variable(String)
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Literal(literal) => write!(f, "{}", literal),
            Value::Variable(var_name) => write!(f, "{}", var_name)
        }
    }
}

impl Value {
    // This name is bad, come up with a better one
    pub fn error_print(&self) -> String {
        match self {
            Value::Literal(literal) => format!("value {}", literal.to_string()),
            Value::Variable(var_name) => format!("var {}", var_name.to_owned())
        }
    }

    pub fn resolve(&self, context: &Context, variable_map: &HashMap<String, AssignmentValue>) -> Result<AssignmentValue, ChimeraRuntimeFailure> {
        match self {
            Value::Literal(val) => {
                Ok(AssignmentValue::Literal(val.clone()))
            },
            Value::Variable(var_name) => {
                let accessors: Vec<&str> = var_name.split(".").collect();
                let value = match variable_map.get(accessors[0]) {
                    Some(res) => res,
                    None => return Err(ChimeraRuntimeFailure::VarNotFound(var_name.to_owned(), context.current_line))
                };
                // TODO: Is there a way to make this method return a ref? clone might be
                //       expensive for large AssignmentValues, like for a big web response.
                //       I think I want to use a Cow here, as that is used for enums that can
                //       have variants which might be borrowed or owned
                if accessors.len() == 1 {
                    return Ok(value.clone())
                }
                else {
                    match value {
                        AssignmentValue::Literal(_literal) => {
                            // TODO: Add support here for indexing a Literal::List when that's added
                            Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(accessors[0].to_string()), accessors[1].to_string(), context.current_line))
                        },
                        AssignmentValue::HttpResponse(http_response) => {
                            match accessors[1] {
                                "status_code" => {
                                    if accessors.len() != 2 {
                                        return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(accessors[0].to_string()), accessors[2].to_string(), context.current_line))
                                    }
                                    Ok(AssignmentValue::Literal(Literal::Int(http_response.status_code as i64)))
                                },
                                "body" => {
                                    if accessors.len() == 2 {
                                        Ok(AssignmentValue::JsonValue(http_response.body.clone()))
                                    }
                                    else {
                                        crate::util::access_json(&http_response.body, &accessors[2..], context)
                                    }
                                },
                                _ => return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(accessors[0].to_string()), accessors[1].to_string(), context.current_line))
                            }
                        },
                        AssignmentValue::JsonValue(json_val) => {
                            crate::util::access_json(json_val, &accessors[1..], context)
                        },
                        AssignmentValue::List(list) => {
                            let index: usize = match accessors[1].parse() {
                                Ok(res) => res,
                                Err(_) => return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                            };
                            if index >= list.len() {
                                return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                            }
                            Ok(AssignmentValue::Literal(list[index].clone()))
                        }
                    }
                }
            }
        }
    }

    pub fn resolve_to_literal(&self, context: &Context, variable_map: &HashMap<String, AssignmentValue>) -> Result<Literal, ChimeraRuntimeFailure> {
        match self.resolve(context, variable_map)? {
            AssignmentValue::Literal(literal) => Ok(literal),
            _ => Err(ChimeraRuntimeFailure::UnsupportedOperation(context.current_line))
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AssertSubCommand {
    EQUALS,
    GTE,
    GT,
    LTE,
    LT,
    STATUS,
    LENGTH
}

impl std::fmt::Display for AssertSubCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AssertSubCommand::EQUALS => write!(f, "equal"),
            AssertSubCommand::GTE => write!(f, "be greater than or equal to"),
            AssertSubCommand::GT => write!(f, "be greater than"),
            AssertSubCommand::LTE => write!(f, "be less than or equal to"),
            AssertSubCommand::LT => write!(f, "be less than"),
            AssertSubCommand::STATUS => write!(f, "have a status code of"),
            AssertSubCommand::LENGTH => write!(f, "have a length of")
        }
    }
}

#[derive(Debug)]
pub struct HttpCommand {
    pub verb: HTTPVerb,
    pub path: String,
    pub http_assignments: Vec<HttpAssignment>,
    key_val_pairs: Vec<KeyValuePair>
}

impl From<Statement> for HttpCommand {
    fn from(value: Statement) -> Self {
        match value {
            Statement::Expression(expr) => match expr {
                Expression::HttpCommand(http_command) => http_command,
                _ => panic!("tried to use an Expression as an HttpCommand when it was not one")
            },
            _ => panic!("tried to use a Statement as an Expression when it was not one")
        }
    }
}

#[derive(Debug)]
pub struct HttpAssignment {
    pub lhs: String,
    pub rhs: Value
}

#[derive(Debug)]
pub struct KeyValuePair {
    key: String,
    value: Value
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Literal {
    // TODO: Add support for a List(Vec<Self>)
    Str(String),
    Int(i64),
    Bool(bool),
    Null
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Str(str) => write!(f, "{}", str),
            Literal::Int(int) => write!(f, "{}", int),
            Literal::Bool(bool) => write!(f, "{}", bool),
            Literal::Null => write!(f, "<null>")
        }
    }
}

#[derive(Debug)]
pub enum ListExpression {
    New(Vec<Value>),
    ListArgument(ListCommand)
}

#[derive(Debug)]
pub struct ListCommand {
    pub list_name: String,
    pub operation: ListCommandOperations
}

impl ListCommand {
    pub fn list_ref<'a>(&'a self, variable_map: &'a HashMap<String, AssignmentValue>, context: &Context) -> Result<&Vec<Literal>, ChimeraRuntimeFailure> {
        match variable_map.get(self.list_name.as_str()) {
            Some(ret) => {
                match ret {
                    AssignmentValue::List(ret_list) => Ok(ret_list),
                    _ => return Err(ChimeraRuntimeFailure::VarWrongType(self.list_name.clone(), VarTypes::List, context.current_line))
                }
            },
            None => return Err(ChimeraRuntimeFailure::VarNotFound(self.list_name.clone(), context.current_line))
        }
    }

    pub fn list_mut_ref<'a>(&'a self, variable_map: &'a mut HashMap<String, AssignmentValue>, context: &Context) -> Result<&mut Vec<Literal>, ChimeraRuntimeFailure> {
        match variable_map.get_mut(self.list_name.as_str()) {
            Some(ret) => {
                match ret {
                    AssignmentValue::List(ret_list) => Ok(ret_list),
                    _ => return Err(ChimeraRuntimeFailure::VarWrongType(self.list_name.clone(), VarTypes::List, context.current_line))
                }
            },
            None => return Err(ChimeraRuntimeFailure::VarNotFound(self.list_name.clone(), context.current_line))
        }
    }
}

#[derive(Debug)]
pub enum ListCommandOperations {
    MutateOperations(MutateListOperations),
    Length
}

#[derive(Debug)]
pub enum MutateListOperations {
    Append(Value),
    Remove(Value)
}

impl From<Statement> for ListExpression {
    fn from(value: Statement) -> Self {
        match value {
            Statement::Expression(expr) => match expr {
                Expression::ListExpression(list_command) => list_command,
                _ => panic!("tried to use an Expression as a ListExpression when it was not one")
            },
            _ => panic!("tried to use a Statement as an Expression when it was not one")
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum HTTPVerb {
    GET,
    PUT,
    POST,
    DELETE
}

#[derive(Clone, PartialEq, Debug)]
pub enum AssignmentValue {
    Literal(Literal),
    // TODO: We should be storing a serde_json::Value::Object here rather than a serde_json::Value,
    //       that way literals can only ever be represented in one way (my Literal variant)
    JsonValue(SerdeJsonValue),
    // TODO: Will also need a new variant like JsonArray for serde_json::Value::Array if the array contains
    //       maps and cannot be represented as a Literal::List when that is implemented
    HttpResponse(HttpResponse),
    List(Vec<Literal>)
}

impl std::fmt::Display for AssignmentValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignmentValue::Literal(literal) => write!(f, "{}", literal),
            AssignmentValue::HttpResponse(res) => write!(f, "var {}", res.var_name),
            AssignmentValue::JsonValue(json_val) => {
                let json_str = crate::util::serde_json_to_string(json_val);
                write!(f, "{}", json_str)
            },
            AssignmentValue::List(list) => {
                let list_as_str = list.into_iter().map(|c| c.to_string()).collect::<Vec<String>>().join(", ");
                write!(f, "[{}]", list_as_str)
            }
        }
    }
}

impl AssignmentValue {
    pub fn resolve_value(value: &Value, variable_map: &HashMap<String, Self>, context: &Context) -> Result<Self, ChimeraRuntimeFailure> {
        value.resolve(context, variable_map)
    }

    pub fn is_numeric(&self) -> bool {
        match self {
            Self::Literal(literal) => {
                match literal {
                    Literal::Int(_) => true,
                    _ => false
                }
            },
            Self::HttpResponse(_) => false,
            Self::JsonValue(json_val) => {
                match json_val {
                    SerdeJsonValue::Number(_) => true,
                    _ => false
                }
            },
            Self::List(_) => false
        }
    }

    pub fn to_int(&self) -> i64 {
        // TODO: Fold is_numeric into this method and have it return a Result<i64, ChimeraruntimeError>
        match self {
            Self::Literal(literal) => {
                match literal {
                    Literal::Str(_str) => panic!("Tried to convert a Literal::String to an int"),
                    Literal::Bool(bool) => {
                        match bool {
                            true => 1,
                            false => 0
                        }
                    },
                    Literal::Int(int) => *int as i64,
                    Literal::Null => panic!("Tried to convert a null value to an int")
                }
            },
            Self::HttpResponse(_) => panic!("Tried to convert a HttpResponse to an int"),
            Self::JsonValue(json_val) => {
                match json_val {
                    SerdeJsonValue::Number(num) => {
                        match num.as_i64() {
                            Some(n) => n,
                            None => panic!("Failed to convert a serde_json::Value Number to an i64")
                        }
                    },
                    _ => panic!("Tried to convert a serde_json::Value to a num when it was not a Number")
                }
            },
            Self::List(_) => panic!("Tried to convert a List to an int")
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct HttpResponse {
    // TODO: Store header data?
    pub status_code: u16,
    pub body: SerdeJsonValue,
    // TODO: Resolve error handling better, this adds code smell and this field shouldn't be here
    pub var_name: String
}

/*
-------------------------------------------------------------------------------------------------
Here be testing
-------------------------------------------------------------------------------------------------
 */

#[cfg(test)]
mod ast_tests {
    use pest::Parser;
    use crate::frontend::CScriptTokenPairs;
    use super::*;

    fn str_to_ast(input: &str) -> ChimeraScriptAST {
        let pairs = match CScriptTokenPairs::parse(Rule::Statement, input) {
            Ok(p) => p,
            Err(_) => panic!("Failed to parse a ChimeraScript string with Pest.")
        };
        match ChimeraScriptAST::from_pairs(pairs) {
            Ok(ast) => ast,
            Err(_chimera_error) => {
                panic!("Failed to convert Pest tokens into an AST.")
            }
        }
    }

    #[test]
    /// Test the simplest possible assertion, 1 == 1, resolves to be an AssertCommand for two literals
    fn simple_parse() {
        let ast = str_to_ast("ASSERT EQUALS 1 1");
        match ast.statement {
            Statement::AssertCommand(assert_command) => {
                assert_eq!(assert_command.negate_assertion, false, "negate_assertion should be false for an assertion which does not contain 'NOT'.");
                assert_eq!(assert_command.subcommand, AssertSubCommand::EQUALS, "Assertion using EQUALS should have an AssertSubCommand::Equals subcommand.");
                assert_eq!(assert_command.left_value, Value::Literal(Literal::Int(1)), "Assertion with a numerical literal should have a Literal::Int() value.");
                assert_eq!(assert_command.right_value, Value::Literal(Literal::Int(1)));
                assert_eq!(assert_command.error_message.is_none(), true, "Assertion error_message should be None when no message is specified.");
            },
            _ => panic!("AST statement of a very simple assertion was not resolved as an AssertCommand variant.")
        }
    }

    #[test]
    /// Test an EQUALS assertion which is negated and has an error message
    fn full_equality_assertion() {
        let ast = str_to_ast("ASSERT NOT EQUALS 1 2 \"foo\"");
        match ast.statement {
            Statement::AssertCommand(assert_command) => {
                assert_eq!(assert_command.negate_assertion, true, "negate_assertion should be true for an assertion which contains 'NOT'.");
                assert_eq!(assert_command.subcommand, AssertSubCommand::EQUALS, "Assertion using EQUALS should have an AssertSubCommand::Equals subcommand.");
                assert_eq!(assert_command.left_value, Value::Literal(Literal::Int(1)), "Assertion with a numerical literal should have a Value::Literal(Literal::Int()) value.");
                assert_eq!(assert_command.right_value, Value::Literal(Literal::Int(2)));
                assert_eq!(assert_command.error_message.is_some(), true, "Assertion error_message should be Some() when message is specified.");
                assert_eq!(assert_command.error_message.unwrap(), "\"foo\"".to_owned(), "Assertion error message was not equal to the supplied message");
            },
            _ => panic!("AST statement of a very simple assertion was not resolved as an AssertCommand variant.")
        }
    }

    #[test]
    /// Test the ASSERT subcommands; EQUALS, GTE, GT, LTE, LT, STATUS
    fn assertion_subcommands() {
        let trees: Vec<AssertCommand> = ["ASSERT EQUALS 1 1", "ASSERT GTE 1 1", "ASSERT GT 1 1", "ASSERT LTE 1 1", "ASSERT LT 1 1", "ASSERT STATUS 1 1"].into_iter().map(|x| str_to_ast(x).statement.into()).collect();
        assert_eq!(trees.len(), 6);
        assert_eq!(trees[0].subcommand, AssertSubCommand::EQUALS);
        assert_eq!(trees[1].subcommand, AssertSubCommand::GTE);
        assert_eq!(trees[2].subcommand, AssertSubCommand::GT);
        assert_eq!(trees[3].subcommand, AssertSubCommand::LTE);
        assert_eq!(trees[4].subcommand, AssertSubCommand::LT);
        assert_eq!(trees[5].subcommand, AssertSubCommand::STATUS);
    }

    #[test]
    /// Test assertions with each of the Value variants
    fn assertion_values() {
        let trees: Vec<AssertCommand> = ["ASSERT EQUALS (foo) 1", "ASSERT EQUALS \"test\" 10", "ASSERT EQUALS true false"].into_iter().map(|x| str_to_ast(x).statement.into()).collect();
        assert_eq!(trees.len(), 3);
        assert_eq!(trees[0].left_value, Value::Variable("foo".to_owned()));
        assert_eq!(trees[0].right_value, Value::Literal(Literal::Int(1)));
        assert_eq!(trees[1].left_value, Value::Literal(Literal::Str("test".to_owned())));
        assert_eq!(trees[2].left_value, Value::Literal(Literal::Bool(true)));
        assert_eq!(trees[2].right_value, Value::Literal(Literal::Bool(false)));
    }

    #[test]
    /// Test a PRINT statement
    fn print_statement() {
        let ast = str_to_ast("PRINT 5");
        match ast.statement {
            Statement::PrintCommand(val) => assert_eq!(val, Value::Literal(Literal::Int(5))),
            _ => panic!("Statement for a PRINT did not resolve to the correct variant.")
        }
    }

    #[test]
    /// Test a simple assignment with a literal expression
    fn assignment_expression() {
        let ast = str_to_ast("var foo = LITERAL 5");
        match ast.statement {
            Statement::AssignmentExpr(assignment_expr) => {
                assert_eq!(assignment_expr.var_name, "foo".to_owned());
                match assignment_expr.expression {
                    Expression::LiteralExpression(literal_expression) => assert_eq!(literal_expression, Literal::Int(5)),
                    _ => panic!("Assignment expression assigning a LITERAL did not resolve with the correct expression field")
                }
            },
            _ => panic!("Statement for an assignment expression did not resolve to the correct variant.")
        }
    }

    #[test]
    /// Test an Http command expression
    fn http_expression() {
        let http_commands: Vec<HttpCommand> = ["GET /foo/bar", "PUT /foo", "POST /foo", "DELETE /foo"].into_iter().map(|x| str_to_ast(x).statement.into()).collect();
        assert_eq!(http_commands.len(), 4);
        assert_eq!(http_commands[0].verb, HTTPVerb::GET);
        assert_eq!(http_commands[0].path, "/foo/bar".to_owned());
        assert_eq!(http_commands[1].verb, HTTPVerb::PUT);
        assert_eq!(http_commands[2].verb, HTTPVerb::POST);
        assert_eq!(http_commands[3].verb, HTTPVerb::DELETE);

        let with_path_assignments: HttpCommand = str_to_ast("GET /foo/bar/baz?foo=5&another=\"bar\"&boolean=true").statement.into();
        assert_eq!(with_path_assignments.path, "/foo/bar/baz?foo=5&another=\"bar\"&boolean=true".to_owned());

        // This HttpCommand has a path with args, assignments, and key/value pairs
        // Probably should make this more atomic though (test just assignment, then key/value, then multiple of each)
        let full_expression: HttpCommand = str_to_ast("GET /foo/bar/baz?foo=5&another=\"bar\" some_num=5 some_str=\"value\" timeout=>60 boolKey=>false").statement.into();
        assert_eq!(full_expression.verb, HTTPVerb::GET);
        assert_eq!(full_expression.path, "/foo/bar/baz?foo=5&another=\"bar\"".to_owned());
        assert_eq!(full_expression.http_assignments.len(), 2);
        assert_eq!(full_expression.http_assignments[0].lhs, "some_num".to_owned());
        assert_eq!(full_expression.http_assignments[0].rhs, Value::Literal(Literal::Int(5)));
        assert_eq!(full_expression.key_val_pairs.len(), 2);
        assert_eq!(full_expression.key_val_pairs[0].key, "timeout".to_owned());
        assert_eq!(full_expression.key_val_pairs[0].value, Value::Literal(Literal::Int(60)));
    }

    #[test]
    /// Test the LIST command
    fn list_expression() {
        let new_list_expression: ListExpression = str_to_ast("LIST NEW [1, true, \"hello world\", (my_var)]").statement.into();
        match new_list_expression {
            ListExpression::New(list_values) => {
                assert_eq!(list_values.len(), 4, "Expected list values to contain 4 values when 4 were provided to LIST NEW");
                assert_eq!(list_values[0], Value::Literal(Literal::Int(1)), "When passing a 1 as the first list value, should have gotten a Value Literal Int");
                assert_eq!(list_values[1], Value::Literal(Literal::Bool(true)), "When passing a true as the second list value, should have gotten a Value Literal Bool");
                assert_eq!(list_values[2], Value::Literal(Literal::Str("hello world".to_owned())), "When passing a \"hello world\" as the third list value, should have gotten a Value Literal Str");
                assert_eq!(list_values[3], Value::Variable("my_var".to_string()), "When passing a (my_var) as the fourth list value, should have gotten a Value Variable");
            }
            ListExpression::ListArgument(_) => panic!("Got a ListExpression::ListArgument variant when a ListExpression::New was expected")
        }
        let list_append_expression: ListExpression = str_to_ast("LIST APPEND (my_list) 5").statement.into();
        match list_append_expression {
            ListExpression::New(_) => panic!("Got a ListExpression::New variant when a ListExpression::ListArgument was expected"),
            ListExpression::ListArgument(list_command) => {
                assert_eq!(list_command.list_name.as_str(), "my_list", "Expected ListCommand to have a list_name of my_list when the command used that as the list variable name");
                match list_command.operation {
                    ListCommandOperations::MutateOperations(mutable_list_operations) => {
                        match mutable_list_operations {
                            MutateListOperations::Append(append_val) => {
                                assert_eq!(append_val, Value::Literal(Literal::Int(5)), "Expected ListCommand's Append operation to contain a Literal Int 5 when the APPEND command was given a 5");
                            },
                            _ => panic!("Expected ListCommand operation to be a MutableOperation Append when using an APPEND command but it wasn't")
                        }
                    },
                    _ => panic!("Expected ListCommand's operation field to be of the Append variant when using an APPEND command but it wasn't")
                }
            }
        }
        let list_remove_expression: ListExpression = str_to_ast("LIST REMOVE (my_list) 10").statement.into();
        match list_remove_expression {
            ListExpression::New(_) => panic!("Got a ListExpression::New variant when a ListExpression::ListArgument was expected"),
            ListExpression::ListArgument(list_command) => {
                assert_eq!(list_command.list_name.as_str(), "my_list", "Expected ListCommand to have a list_name of my_list when the command used that as the list variable name");
                match list_command.operation {
                    ListCommandOperations::MutateOperations(mutable_list_operations) => {
                        match mutable_list_operations {
                            MutateListOperations::Remove(remove_val) => {
                                assert_eq!(remove_val, Value::Literal(Literal::Int(10)), "Expected ListCommand's Remove operation to contain a Literal Int 10 when the REMOVE command was given a 10");
                            },
                            _ => panic!("Expected ListCommand operation to be a MutableOperation Remove when using a REMOVE command but it wasn't")
                        }
                    },
                    _ => panic!("Expected ListCommand's operation field to be of the Remove variant when using an REMOVE command but it wasn't")
                }
            }
        }
        let list_length_expression: ListExpression = str_to_ast("LIST LENGTH (some_list)").statement.into();
        match list_length_expression {
            ListExpression::New(_) => panic!("Got a ListExpression::New variant when a ListExpression::ListArgument was expected"),
            ListExpression::ListArgument(list_command) => {
                assert_eq!(list_command.list_name.as_str(), "some_list", "Expected ListCommand to have a list_name of some_list when the command used that as the list variable name");
                match list_command.operation {
                    ListCommandOperations::Length => (),
                    _ => panic!("Expected ListCommand's operation field to be of the Length variant when using a LENGTH command but it wasn't")
                }
            }
        }
    }
}
