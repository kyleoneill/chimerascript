use std::collections::HashMap;
use std::fmt::Formatter;
use pest::iterators::{Pair, Pairs};
use serde::de::{Deserialize, Error, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
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
                    "CONTAINS" => AssertSubCommand::CONTAINS,
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
        // TODO: Better number handling than just setting every number to an i64
        match pair.as_str().parse::<i64>() {
            Ok(res) => return Ok(Literal::Number(res)),
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
                                Literal::String(second_inner.as_str().to_owned())
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
                //       have variants which might be borrowed or owned. Applies for both what's returned from if
                //       and else blocks
                if accessors.len() == 1 {
                    return Ok(value.clone())
                }
                else {
                    match value {
                        AssignmentValue::Literal(literal) => { Ok(AssignmentValue::Literal(literal.resolve_access(accessors, context)?.to_owned())) },
                        AssignmentValue::HttpResponse(http_response) => {
                            match accessors[1] {
                                "status_code" => {
                                    if accessors.len() != 2 {
                                        return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(accessors[0].to_string()), accessors[2].to_string(), context.current_line))
                                    }
                                    Ok(AssignmentValue::Literal(Literal::Number(http_response.status_code as i64)))
                                },
                                "body" => {
                                    let mut without_body_accessor = vec![accessors[0]];
                                    if accessors.len() > 2 {
                                        without_body_accessor.append(&mut accessors[2..].to_vec());
                                    }
                                    Ok(AssignmentValue::Literal(http_response.body.resolve_access(without_body_accessor, context)?.to_owned()))
                                },
                                _ => return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(accessors[0].to_string()), accessors[1].to_string(), context.current_line))
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn resolve_to_literal(&self, context: &Context, variable_map: &HashMap<String, AssignmentValue>) -> Result<Literal, ChimeraRuntimeFailure> {
        match self.resolve(context, variable_map)? {
            AssignmentValue::Literal(literal) => Ok(literal),
            _ => Err(ChimeraRuntimeFailure::VarWrongType(self.error_print(), VarTypes::Literal, context.current_line))
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
    LENGTH,
    CONTAINS
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
            AssertSubCommand::LENGTH => write!(f, "have a length of"),
            AssertSubCommand::CONTAINS => write!(f, "to contain")
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Literal {
    String(String),
    // TODO: Need better number support here for floats and u64s and maybe even bigints?
    //       Also I think this to do is duplicated in more than one location, search for/resolve them all when done
    Number(i64),
    Bool(bool),
    Null,
    Object(HashMap<String, Self>),
    List(Vec<Self>)
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::String(str) => write!(f, "{}", str),
            Literal::Number(int) => write!(f, "{}", int),
            Literal::Bool(bool) => write!(f, "{}", bool),
            Literal::Null => write!(f, "<null>"),
            Literal::Object(object) => {
                for (key, val) in object.iter() {
                    let val_string = val.to_string();
                    write!(f, "{{\"{}\"}}\":\"{{{}}}\"", key, val_string)?;
                }
                Ok(())
            },
            Literal::List(list) => {
                let list_as_str = list.into_iter().map(|c| c.to_string()).collect::<Vec<String>>().join(", ");
                write!(f, "[{}]", list_as_str)
            }
        }
    }
}

impl Literal {
    fn resolve_access(&self, mut accessors: Vec<&str>, context: &Context) -> Result<&Self, ChimeraRuntimeFailure> {
        accessors.reverse();
        let var_name = match accessors.len() {
            0 => return Err(ChimeraRuntimeFailure::InternalError("resolving the access of a Literal".to_string())),
            _ => accessors.pop().unwrap().to_owned()
        };
        let mut pointer = self;
        while accessors.len() != 0 {
            let accessor = accessors.pop().unwrap();
            match pointer {
                Literal::Object(obj) => {
                    pointer = match obj.get(accessor) {
                        Some(val) => val,
                        None => return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(var_name), accessor.to_string(), context.current_line))
                    }
                },
                Literal::List(arr) => {
                    let index: usize = match accessor.parse() {
                        Ok(i) => i,
                        Err(_) => return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                    };
                    if index >= arr.len() { return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line)) }
                    pointer = &arr[index];
                },
                _ => break
            }
        }
        if accessors.len() > 0 {
            return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(var_name), accessors[accessors.len() - 2].to_string(), context.current_line))
        }
        Ok(pointer)
    }
    pub fn to_number(&self) -> Option<i64> {
        match self {
            Self::Number(i) => Some(*i),
            _ => None
        }
    }
    fn to_list(&self) -> Option<&Vec<Self>> {
        match self {
            Self::List(list) => Some(list),
            _ => None
        }
    }
    fn internal_to_string(&self) -> Option<&str> {
        match self {
            Self::String(string) => Some(string.as_str()),
            _ => None
        }
    }
    pub fn to_number_or_error(&self, came_from: &Value, context: &Context) -> Result<i64, ChimeraRuntimeFailure> {
        Ok(self.to_number().ok_or_else(|| return ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::Int, context.current_line))?)
    }
    pub fn to_list_or_error(&self, came_from: &Value, context: &Context) -> Result<&Vec<Self>, ChimeraRuntimeFailure> {
        Ok(self.to_list().ok_or_else(|| return ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::List, context.current_line))?)
    }
    pub fn to_string_or_error(&self, came_from: &Value, context: &Context) -> Result<&str, ChimeraRuntimeFailure> {
        Ok(self.internal_to_string().ok_or_else(|| return ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::String, context.current_line))?)
    }
}

impl <'de> Deserialize<'de> for Literal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct LiteralVisitor;
        impl<'de> Visitor<'de> for LiteralVisitor {
            type Value = Literal;
            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Bool(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Number(v))
            }
            // TODO: impl real values for u64 and f64 here
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Number(v as i64))
            }
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Number(v as i64))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
                self.visit_string(String::from(v))
            }
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::String(v))
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Null)
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
                Deserialize::deserialize(deserializer)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Null)
            }
            fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
                let mut vec = Vec::new();
                while let Some(member) = visitor.next_element()? {
                    vec.push(member)
                }
                Ok(Literal::List(vec))
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
                match map.next_key()? {
                    Some(first_key) => {
                        let mut values: HashMap<String, Literal> = HashMap::new();
                        values.insert(first_key, map.next_value()?);
                        while let Some((key, value)) = map.next_entry()? {
                            values.insert(key, value);
                        }
                        Ok(Literal::Object(values))
                    },
                    None => Ok(Literal::Object(HashMap::new()))
                }
            }
        }
        deserializer.deserialize_any(LiteralVisitor)
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
        return match variable_map.get(self.list_name.as_str()) {
            Some(ret) => {
                match ret {
                    AssignmentValue::Literal(lit) => {
                        match lit {
                            Literal::List(list) => {
                                return Ok(list)
                            }
                            _ => ()
                        }
                    },
                    _ => ()
                }
                Err(ChimeraRuntimeFailure::VarWrongType(self.list_name.clone(), VarTypes::List, context.current_line))
            },
            None => Err(ChimeraRuntimeFailure::VarNotFound(self.list_name.clone(), context.current_line))
        }
    }

    pub fn list_mut_ref<'a>(&'a self, variable_map: &'a mut HashMap<String, AssignmentValue>, context: &Context) -> Result<&mut Vec<Literal>, ChimeraRuntimeFailure> {
        return match variable_map.get_mut(self.list_name.as_str()) {
            Some(ret) => {
                match ret {
                    AssignmentValue::Literal(lit) => {
                        match lit {
                            Literal::List(list) => {
                                return Ok(list)
                            }
                            _ => ()
                        }
                    },
                    _ => ()
                }
                Err(ChimeraRuntimeFailure::VarWrongType(self.list_name.clone(), VarTypes::List, context.current_line))
            },
            None => Err(ChimeraRuntimeFailure::VarNotFound(self.list_name.clone(), context.current_line))
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
    HttpResponse(HttpResponse)
}

impl std::fmt::Display for AssignmentValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignmentValue::Literal(literal) => write!(f, "{}", literal),
            AssignmentValue::HttpResponse(res) => write!(f, "[HttpResponse status_code:{} body:{}]", res.status_code, res.body)
        }
    }
}

impl AssignmentValue {
    pub fn to_literal(&self) -> Option<&Literal> {
        match self {
            Self::Literal(literal) => Some(literal),
            _ => None
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct HttpResponse {
    // TODO: Store header data?
    pub status_code: u16,
    pub body: Literal
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
                assert_eq!(assert_command.left_value, Value::Literal(Literal::Number(1)), "Assertion with a numerical literal should have a Literal::Int() value.");
                assert_eq!(assert_command.right_value, Value::Literal(Literal::Number(1)));
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
                assert_eq!(assert_command.left_value, Value::Literal(Literal::Number(1)), "Assertion with a numerical literal should have a Value::Literal(Literal::Int()) value.");
                assert_eq!(assert_command.right_value, Value::Literal(Literal::Number(2)));
                assert_eq!(assert_command.error_message.is_some(), true, "Assertion error_message should be Some() when message is specified.");
                assert_eq!(assert_command.error_message.unwrap(), "\"foo\"".to_owned(), "Assertion error message was not equal to the supplied message");
            },
            _ => panic!("AST statement of a very simple assertion was not resolved as an AssertCommand variant.")
        }
    }

    #[test]
    /// Test the ASSERT subcommands; EQUALS, GTE, GT, LTE, LT, STATUS
    fn assertion_subcommands() {
        let trees: Vec<AssertCommand> = ["ASSERT EQUALS 1 1", "ASSERT GTE 1 1", "ASSERT GT 1 1", "ASSERT LTE 1 1", "ASSERT LT 1 1", "ASSERT STATUS 1 1", "ASSERT LENGTH (foo) 1", "ASSERT CONTAINS (foo) 1"].into_iter().map(|x| str_to_ast(x).statement.into()).collect();
        assert_eq!(trees.len(), 8);
        assert_eq!(trees[0].subcommand, AssertSubCommand::EQUALS);
        assert_eq!(trees[1].subcommand, AssertSubCommand::GTE);
        assert_eq!(trees[2].subcommand, AssertSubCommand::GT);
        assert_eq!(trees[3].subcommand, AssertSubCommand::LTE);
        assert_eq!(trees[4].subcommand, AssertSubCommand::LT);
        assert_eq!(trees[5].subcommand, AssertSubCommand::STATUS);
        assert_eq!(trees[6].subcommand, AssertSubCommand::LENGTH);
        assert_eq!(trees[7].subcommand, AssertSubCommand::CONTAINS);
    }

    #[test]
    /// Test assertions with each of the Value variants
    fn assertion_values() {
        let trees: Vec<AssertCommand> = ["ASSERT EQUALS (foo) 1", "ASSERT EQUALS \"test\" 10", "ASSERT EQUALS true false"].into_iter().map(|x| str_to_ast(x).statement.into()).collect();
        assert_eq!(trees.len(), 3);
        assert_eq!(trees[0].left_value, Value::Variable("foo".to_owned()));
        assert_eq!(trees[0].right_value, Value::Literal(Literal::Number(1)));
        assert_eq!(trees[1].left_value, Value::Literal(Literal::String("test".to_owned())));
        assert_eq!(trees[2].left_value, Value::Literal(Literal::Bool(true)));
        assert_eq!(trees[2].right_value, Value::Literal(Literal::Bool(false)));
    }

    #[test]
    /// Test a PRINT statement
    fn print_statement() {
        let ast = str_to_ast("PRINT 5");
        match ast.statement {
            Statement::PrintCommand(val) => assert_eq!(val, Value::Literal(Literal::Number(5))),
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
                    Expression::LiteralExpression(literal_expression) => assert_eq!(literal_expression, Literal::Number(5)),
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
        assert_eq!(full_expression.http_assignments[0].rhs, Value::Literal(Literal::Number(5)));
        assert_eq!(full_expression.key_val_pairs.len(), 2);
        assert_eq!(full_expression.key_val_pairs[0].key, "timeout".to_owned());
        assert_eq!(full_expression.key_val_pairs[0].value, Value::Literal(Literal::Number(60)));
    }

    #[test]
    /// Test the LIST command
    fn list_expression() {
        let new_list_expression: ListExpression = str_to_ast("LIST NEW [1, true, \"hello world\", (my_var)]").statement.into();
        match new_list_expression {
            ListExpression::New(list_values) => {
                assert_eq!(list_values.len(), 4, "Expected list values to contain 4 values when 4 were provided to LIST NEW");
                assert_eq!(list_values[0], Value::Literal(Literal::Number(1)), "When passing a 1 as the first list value, should have gotten a Value Literal Int");
                assert_eq!(list_values[1], Value::Literal(Literal::Bool(true)), "When passing a true as the second list value, should have gotten a Value Literal Bool");
                assert_eq!(list_values[2], Value::Literal(Literal::String("hello world".to_owned())), "When passing a \"hello world\" as the third list value, should have gotten a Value Literal Str");
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
                                assert_eq!(append_val, Value::Literal(Literal::Number(5)), "Expected ListCommand's Append operation to contain a Literal Int 5 when the APPEND command was given a 5");
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
                                assert_eq!(remove_val, Value::Literal(Literal::Number(10)), "Expected ListCommand's Remove operation to contain a Literal Int 10 when the REMOVE command was given a 10");
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
