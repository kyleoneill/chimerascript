use crate::err_handle::{ChimeraCompileError, ChimeraRuntimeFailure, VarTypes};
use crate::frontend::{Context, Rule};
use crate::literal::{Data, DataKind, Literal, NumberKind};
use crate::variable_map::VariableMap;
use crate::{frontend, CLIENT};
use pest::iterators::Pair;
use reqwest::header::{HeaderMap, HeaderName};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::ops::Deref;

// This has a return value despite only panicking so satisfy the compiler, as it's called inside of
// `ok_or_else(|| no_pairs_panic())` closures which are meant to transform an Option into a Result.
// They are being used to get a value out of an Option and panic if it's a None, but the closure needs to return
// an Err() in order for the compiler to allow the `ok_or_else()` to be question marked
fn no_pairs_panic(rule_name: &str) -> ChimeraCompileError {
    panic!(
        "Ran out of mandatory inner pairs when parsing a Rule::{}",
        rule_name
    )
}

#[derive(Debug)]
pub struct ChimeraScriptAST {
    pub functions: Vec<Function>,
}

impl ChimeraScriptAST {
    /// Generate an abstract syntax tree from a string of ChimeraScript
    pub fn new(input: &str) -> Result<Self, ChimeraCompileError> {
        let mut pairs = frontend::parse_main(input)?;
        let main_pair = pairs.next().ok_or_else(|| panic!("Did not get any pairs after parsing a string into a Rule::Main but there must be at least one"))?;
        if main_pair.as_rule() != Rule::Main {
            panic!("Expected the first pair of a parse to be Rule::Main but it was not")
        };
        let function_pairs = main_pair.into_inner();
        let mut functions: Vec<Function> = Vec::new();
        for function_pair in function_pairs {
            if function_pair.as_rule() == Rule::EOI {
                break;
            }
            let function = Self::pair_to_function(function_pair)?;
            functions.push(function);
        }
        Ok(Self { functions })
    }

    fn pair_to_function(function_pair: Pair<Rule>) -> Result<Function, ChimeraCompileError> {
        if function_pair.as_rule() != Rule::Function {
            panic!("Expected pairs within a Rule::Main to only be Rule::Function but one was not")
        };
        let mut function_pairs = function_pair.into_inner();
        let mut current_pair = function_pairs
            .next()
            .expect("Rule::Function contained no inner pairs when it must have at least two");
        let mut decorators: Vec<Decorator> = Vec::new();
        if current_pair.as_rule() == Rule::Decorators {
            let decorator_pairs = current_pair.into_inner();
            for decorator_pair in decorator_pairs {
                match decorator_pair.as_rule() {
                    Rule::StrPlus => {
                        decorators.push(Decorator::Key(decorator_pair.as_str().to_owned()))
                    }
                    Rule::DecoratorKeyValuePair => {
                        let mut kv_inner = decorator_pair.into_inner();
                        let key_pair = kv_inner
                            .next()
                            .expect("A Rule::DecoratorKeyValuePair must contain a key pair");
                        let value_pair = kv_inner
                            .next()
                            .expect("A Rule::DecoratorKeyValuePair must contain a value pair");
                        decorators.push(Decorator::KeyValue((
                            key_pair.as_str().to_owned(),
                            value_pair.as_str().to_owned(),
                        )));
                    }
                    _ => panic!("Got an invalid Rule variant inside of a Rule::Decorator"),
                }
            }
            current_pair = function_pairs.next().expect("A Rule::Function must contain at least one pair after a Rule::Decorator but it did not");
        }
        if current_pair.as_rule() != Rule::StrPlus {
            panic!("Expected a StrPlus rule inside a Rule::Function for the function name")
        };
        let name = current_pair.as_str().to_owned();
        let block = ChimeraScriptAST::pair_to_block(
            function_pairs
                .next()
                .expect("Expected a Rule::Block inside a Rule::Function"),
        )?;
        Ok(Function {
            decorators,
            name,
            block,
        })
    }

    fn pair_to_block(block_pair: Pair<Rule>) -> Result<Vec<BlockContents>, ChimeraCompileError> {
        if block_pair.as_rule() != Rule::Block {
            panic!("Expected rule to be Rule::Block when parsing into a Vec<BlockContents>")
        };
        let mut block: Vec<BlockContents> = Vec::new();
        let block_pair_inner = block_pair.into_inner();
        for block_content in block_pair_inner {
            let content = match block_content.as_rule() {
                Rule::Statement => {
                    BlockContents::Statement(ChimeraScriptAST::pair_to_statement(block_content)?)
                }
                Rule::Function => {
                    BlockContents::Function(ChimeraScriptAST::pair_to_function(block_content)?)
                }
                Rule::Teardown => {
                    BlockContents::Teardown(ChimeraScriptAST::pair_to_teardown(block_content)?)
                }
                _ => panic!("Got an invalid rule when parsing a Rule::Block inner"),
            };
            block.push(content);
        }
        Ok(block)
    }

    fn pair_to_teardown(teardown_pair: Pair<Rule>) -> Result<Teardown, ChimeraCompileError> {
        if teardown_pair.as_rule() != Rule::Teardown {
            return Err(ChimeraCompileError::new(
                "Got invalid data when reading a teardown block",
                teardown_pair.line_col(),
            ));
        };
        let mut statements: Vec<Statement> = Vec::new();
        let teardown_inner = teardown_pair.into_inner();
        for teardown_statement in teardown_inner {
            statements.push(ChimeraScriptAST::pair_to_statement(teardown_statement)?)
        }
        Ok(Teardown { statements })
    }

    fn pair_to_statement(statement_pair: Pair<Rule>) -> Result<Statement, ChimeraCompileError> {
        if statement_pair.as_rule() != Rule::Statement {
            return Err(ChimeraCompileError::new(
                "Got invalid data when reading a statement",
                statement_pair.line_col(),
            ));
        };
        let statement_inner = statement_pair
            .into_inner()
            .next()
            .expect("A Rule::Statement inner must always have one inner pair");
        // TODO: Break these up into their own individual "pair_to_x" functions. Clean up how they're written
        match statement_inner.as_rule() {
            Rule::AssertCommand => Ok(Statement::AssertCommand(Self::parse_rule_to_assertion(
                statement_inner,
            )?)),
            Rule::AssignmentExpr => {
                // An AssignmentExpr is going to contain
                // 1. A string representing a variable name
                // 2. An expression
                let mut pairs = statement_inner.into_inner();

                let var_name_pair = pairs
                    .next()
                    .ok_or_else(|| no_pairs_panic("AssignmentExpr's variable name"))?;
                if var_name_pair.as_rule() != Rule::VariableNameAssignment {
                    return Err(ChimeraCompileError::new(
                        "Expected data to be a valid variable name",
                        var_name_pair.line_col(),
                    ));
                }
                let var_name = var_name_pair.as_str().to_owned();

                let expression_pair = pairs
                    .next()
                    .ok_or_else(|| no_pairs_panic("AssignmentExpr's expression"))?;
                let expression = ChimeraScriptAST::parse_rule_to_expression(expression_pair)?;
                Ok(Statement::AssignmentExpr(AssignmentExpr {
                    var_name,
                    expression,
                }))
            }
            Rule::PrintCommand => {
                // A PrintCommand is going to contain
                // 1. A value to print
                let mut pairs = statement_inner.into_inner();
                let value_pair = pairs
                    .next()
                    .ok_or_else(|| no_pairs_panic("PrintCommand's value"))?;
                let value = ChimeraScriptAST::parse_rule_to_value(value_pair)?;
                Ok(Statement::PrintCommand(value))
            }
            Rule::Expression => {
                let expression = ChimeraScriptAST::parse_rule_to_expression(statement_inner)?;
                Ok(Statement::Expression(expression))
            }
            _ => Err(ChimeraCompileError::new(
                "Did not get a valid statement",
                statement_inner.line_col(),
            )),
        }
    }

    fn parse_rule_to_assertion(pair: Pair<Rule>) -> Result<AssertCommand, ChimeraCompileError> {
        // An AssertCommand inner is going to contain
        // 1. Optional Negation
        // 2. AssertSubCommand
        // 3. Value
        // 4. Value
        // 5. Optional QuoteString
        if pair.as_rule() != Rule::AssertCommand {
            return Err(ChimeraCompileError::new(
                "Did not get a valid AssertCommand value",
                pair.line_col(),
            ));
        }
        let mut pairs = pair.into_inner();

        // Peek ahead to see if our inner contains an optional Negation
        let negate_assertion = match pairs.peek() {
            Some(next) => next.as_rule() == Rule::Negation,
            None => panic!("Expected a Rule::AssertCommand to contain inner pairs but it did not"),
        };

        // peek() does not move the iterator position, so if we did have a negation then we
        // need to move the iterator ahead by one position
        if negate_assertion {
            let _ = pairs.next();
        }

        // Get the sub-command
        let subcommand_pair = pairs
            .next()
            .ok_or_else(|| no_pairs_panic("AssertCommand subcommand"))?;
        if subcommand_pair.as_rule() != Rule::AssertSubCommand {
            return Err(ChimeraCompileError::new(
                "Got invalid data when reading an assertion subcommand",
                subcommand_pair.line_col(),
            ));
        }
        let subcommand = match subcommand_pair.as_span().as_str() {
            "EQUALS" => AssertSubCommand::Equals,
            "GTE" => AssertSubCommand::GTE,
            "GT" => AssertSubCommand::GT,
            "LTE" => AssertSubCommand::LTE,
            "LT" => AssertSubCommand::LT,
            "STATUS" => AssertSubCommand::Status,
            "LENGTH" => AssertSubCommand::Length,
            "CONTAINS" => AssertSubCommand::Contains,
            _ => {
                return Err(ChimeraCompileError::new(
                    "Got an invalid assertion subcommand value",
                    subcommand_pair.line_col(),
                ))
            }
        };

        // Get the first value we're asserting with
        let left_value_pair = pairs
            .next()
            .ok_or_else(|| no_pairs_panic("AssertCommand's first value param"))?;
        let left_value = ChimeraScriptAST::parse_rule_to_value(left_value_pair)?;

        // Get the second value we're asserting with
        let right_value_pair = pairs
            .next()
            .ok_or_else(|| no_pairs_panic("AssertCommand's second value param"))?;
        let right_value = ChimeraScriptAST::parse_rule_to_value(right_value_pair)?;

        // Check for an optional error message, can be a literal quotestring or a formatted string
        let error_message = match pairs.peek() {
            Some(next) => match next.as_rule() {
                Rule::QuoteString => Some(Value::Literal(Literal::String(
                    Self::parse_quotestring_rule(next)?,
                ))),
                Rule::FormattedString => Some(Value::FormattedString(
                    Self::parse_rule_to_formatted_string(next)?,
                )),
                _ => {
                    return Err(ChimeraCompileError::new(
                        "Got an invalid rule inside an AssertCommand's error message",
                        next.line_col(),
                    ))
                }
            },
            None => None,
        };

        Ok(AssertCommand {
            negate_assertion,
            subcommand,
            left_value,
            right_value,
            error_message,
        })
    }

    fn parse_rule_to_variable_name(pair: Pair<Rule>) -> Result<String, ChimeraCompileError> {
        if pair.as_rule() != Rule::VariableValue {
            return Err(ChimeraCompileError::new(
                "Did not get a valid variable value",
                pair.line_col(),
            ));
        }
        let inner = pair
            .into_inner()
            .next()
            .expect("A VariableValue must always have a NestedVariable inner");
        Ok(inner.as_str().to_owned())
    }

    fn parse_rule_to_value(pair: Pair<Rule>) -> Result<Value, ChimeraCompileError> {
        if pair.as_rule() != Rule::Value {
            return Err(ChimeraCompileError::new(
                "Did not get a valid value",
                pair.line_col(),
            ));
        };
        let inner = pair
            .into_inner()
            .peek()
            .ok_or_else(|| no_pairs_panic("Value"))?;
        match inner.as_rule() {
            Rule::LiteralValue => Ok(Value::Literal(
                ChimeraScriptAST::parse_rule_to_literal_value(inner)?,
            )),
            Rule::VariableValue => Ok(Value::Variable(
                ChimeraScriptAST::parse_rule_to_variable_name(inner)?,
            )),
            Rule::FormattedString => Ok(Value::FormattedString(
                Self::parse_rule_to_formatted_string(inner)?,
            )),
            _ => Err(ChimeraCompileError::new(
                "Did not get a valid Value",
                inner.line_col(),
            )),
        }
    }

    fn parse_quotestring_rule(pair: Pair<Rule>) -> Result<String, ChimeraCompileError> {
        if pair.as_rule() != Rule::QuoteString {
            return Err(ChimeraCompileError::new(
                "Expected data to be a quoted string",
                pair.line_col(),
            ));
        }
        Ok(pair
            .into_inner()
            .next()
            .expect("A Rule::QuoteString must contain an inner value but it didn't")
            .as_str()
            .to_owned())
    }

    fn parse_rule_to_literal_value(pair: Pair<Rule>) -> Result<Literal, ChimeraCompileError> {
        if pair.as_rule() != Rule::LiteralValue {
            return Err(ChimeraCompileError::new(
                "Did not get a valid literal",
                pair.line_col(),
            ));
        }
        let literal_value = pair
            .into_inner()
            .peek()
            .ok_or_else(|| no_pairs_panic("LiteralValue"))?;
        match literal_value.as_rule() {
            Rule::QuoteString => Ok(Literal::String(ChimeraScriptAST::parse_quotestring_rule(
                literal_value,
            )?)),
            Rule::Number => {
                let number_kind = literal_value
                    .into_inner()
                    .peek()
                    .ok_or_else(|| no_pairs_panic("Number"))?;
                match number_kind.as_rule() {
                    Rule::Float => match number_kind.as_str().parse::<f64>() {
                        Ok(as_float) => Ok(Literal::Number(NumberKind::F64(as_float))),
                        Err(_) => Err(ChimeraCompileError::new(
                            "Failed to parse a float",
                            number_kind.line_col(),
                        )),
                    },
                    Rule::SignedNumber => match number_kind.as_str().parse::<i64>() {
                        Ok(as_signed) => Ok(Literal::Number(NumberKind::I64(as_signed))),
                        Err(_) => Err(ChimeraCompileError::new(
                            "Failed to parse a signed number",
                            number_kind.line_col(),
                        )),
                    },
                    Rule::UnsignedNumber => match number_kind.as_str().parse::<u64>() {
                        Ok(as_unsigned) => Ok(Literal::Number(NumberKind::U64(as_unsigned))),
                        Err(_) => Err(ChimeraCompileError::new(
                            "Failed to parse an unsigned number",
                            number_kind.line_col(),
                        )),
                    },
                    _ => Err(ChimeraCompileError::new(
                        "Did not get a valid number",
                        number_kind.line_col(),
                    )),
                }
            }
            Rule::Boolean => match literal_value.as_str() {
                "true" | "True" => Ok(Literal::Bool(true)),
                "false" | "False" => Ok(Literal::Bool(false)),
                _ => Err(ChimeraCompileError::new(
                    "Did not get a valid boolean",
                    literal_value.line_col(),
                )),
            },
            Rule::Null => Ok(Literal::Null),
            _ => Err(ChimeraCompileError::new(
                "Did not get a valid literal",
                literal_value.line_col(),
            )),
        }
    }

    fn parse_rule_to_path(pair: Pair<Rule>) -> Result<Vec<Value>, ChimeraCompileError> {
        if pair.as_rule() != Rule::Path {
            return Err(ChimeraCompileError::new(
                "Did not get a valid path",
                pair.line_col(),
            ));
        }
        let path_inner = pair.into_inner();
        let mut build_path: Vec<Value> = Vec::new();
        let mut buffer: String = String::new();
        for token in path_inner {
            match token.as_rule() {
                Rule::PathEndpoint => {
                    buffer.push('/');
                    let endpoint_portion = token.into_inner();
                    for pair in endpoint_portion {
                        let kind = pair
                            .into_inner()
                            .next()
                            .ok_or_else(|| no_pairs_panic("PathEndpoint"))?;
                        match kind.as_rule() {
                            Rule::StrPlus => buffer.push_str(kind.as_str()),
                            Rule::VariableValue => {
                                build_path.push(Value::Literal(Literal::String(buffer)));
                                buffer = String::new();
                                let var_name = ChimeraScriptAST::parse_rule_to_variable_name(kind)?;
                                build_path.push(Value::Variable(var_name));
                            }
                            _ => {
                                return Err(ChimeraCompileError::new(
                                    "Did not get a valid path endpoint",
                                    kind.line_col(),
                                ))
                            }
                        }
                    }
                }
                _ => {
                    return Err(ChimeraCompileError::new(
                        "Did not get a valid path",
                        token.line_col(),
                    ))
                }
            }
        }
        // check if the buffer is empty, add it if it is
        if !buffer.is_empty() {
            build_path.push(Value::Literal(Literal::String(buffer)));
        }
        Ok(build_path)
    }

    fn parse_rule_to_http_assignment(
        pair: Pair<Rule>,
    ) -> Result<HttpAssignment, ChimeraCompileError> {
        if pair.as_rule() != Rule::HttpAssignment && pair.as_rule() != Rule::HttpHeader {
            return Err(ChimeraCompileError::new(
                "Did not get a valid http assignment",
                pair.line_col(),
            ));
        };
        let mut http_assignment_pairs = pair.into_inner();

        let assignment_token = http_assignment_pairs
            .next()
            .ok_or_else(|| no_pairs_panic("HttpAssignment"))?;
        if assignment_token.as_rule() != Rule::VariableNameAssignment {
            return Err(ChimeraCompileError::new(
                "Did not get a valid variable name for an http key value pair",
                assignment_token.line_col(),
            ));
        }
        let lhs = assignment_token.as_str().to_owned();

        let value_token = http_assignment_pairs
            .next()
            .ok_or_else(|| no_pairs_panic("HttpAssignment"))?;
        let rhs = ChimeraScriptAST::parse_rule_to_value(value_token)?;

        Ok(HttpAssignment { lhs, rhs })
    }

    fn parse_rule_to_query_params(
        pair: Pair<Rule>,
    ) -> Result<Vec<HttpAssignment>, ChimeraCompileError> {
        if pair.as_rule() != Rule::QueryParams {
            return Err(ChimeraCompileError::new(
                "Did not get a valid query param",
                pair.line_col(),
            ));
        }
        let mut query_param_pairs = pair.into_inner();
        let mut query_params: Vec<HttpAssignment> = Vec::new();

        // Will always have a first pair, it will be an HttpAssignment
        let first_param =
            ChimeraScriptAST::parse_rule_to_http_assignment(query_param_pairs.next().unwrap())?;
        query_params.push(first_param);

        // Check for optional additional query params
        while query_param_pairs.peek().is_some()
            && query_param_pairs.peek().unwrap().as_rule() == Rule::AdditionalPathArgs
        {
            let mut inner = query_param_pairs.next().unwrap().into_inner();
            let http_assignment =
                ChimeraScriptAST::parse_rule_to_http_assignment(inner.next().unwrap())?;
            query_params.push(http_assignment);
        }

        Ok(query_params)
    }

    fn parse_rule_to_http_command(pair: Pair<Rule>) -> Result<Expression, ChimeraCompileError> {
        if pair.as_rule() != Rule::HttpCommand {
            return Err(ChimeraCompileError::new(
                "Did not get a valid http command",
                pair.line_col(),
            ));
        }
        let mut http_pairs = pair.into_inner();

        let verb_token = http_pairs
            .next()
            .ok_or_else(|| no_pairs_panic("HttpCommand"))?;
        if verb_token.as_rule() != Rule::HTTPVerb {
            return Err(ChimeraCompileError::new(
                "Did not get a valid HTTP verb",
                verb_token.line_col(),
            ));
        }
        let verb = match verb_token.as_str() {
            "GET" => HTTPVerb::Get,
            "PUT" => HTTPVerb::Put,
            "POST" => HTTPVerb::Post,
            "DELETE" => HTTPVerb::Delete,
            _ => {
                return Err(ChimeraCompileError::new(
                    "Did not get a valid HTTP verb",
                    verb_token.line_col(),
                ))
            }
        };

        let path_token = http_pairs
            .next()
            .ok_or_else(|| no_pairs_panic("HttpCommand"))?;
        let path = ChimeraScriptAST::parse_rule_to_path(path_token)?;

        let mut query_params: Vec<HttpAssignment> = Vec::new();
        if http_pairs.peek().is_some() && http_pairs.peek().unwrap().as_rule() == Rule::QueryParams
        {
            query_params =
                ChimeraScriptAST::parse_rule_to_query_params(http_pairs.next().unwrap())?;
        }

        // Peek ahead and iterate over any HttpAssignment pairs to get body params
        let mut http_assignments: Vec<HttpAssignment> = Vec::new();
        while http_pairs.peek().is_some()
            && http_pairs.peek().unwrap().as_rule() == Rule::HttpAssignment
        {
            let http_assignment =
                ChimeraScriptAST::parse_rule_to_http_assignment(http_pairs.next().unwrap())?;
            http_assignments.push(http_assignment);
        }

        // Peek ahead and iterate over any HttpHeader HttpAssignment pairs
        let mut headers: Vec<HttpAssignment> = Vec::new();
        while http_pairs.peek().is_some()
            && http_pairs.peek().unwrap().as_rule() == Rule::HttpHeader
        {
            let http_assignment =
                ChimeraScriptAST::parse_rule_to_http_assignment(http_pairs.next().unwrap())?;
            headers.push(http_assignment);
        }

        // Peek ahead and iterate over any KeyValuePair pairs
        let mut key_val_pairs: Vec<KeyValuePair> = Vec::new();
        while http_pairs.peek().is_some()
            && http_pairs.peek().unwrap().as_rule() == Rule::KeyValuePair
        {
            let mut key_value_pairs = http_pairs.next().unwrap().into_inner();

            let assignment_token = key_value_pairs
                .next()
                .ok_or_else(|| no_pairs_panic("KeyValuePair"))?;
            if assignment_token.as_rule() != Rule::VariableNameAssignment {
                return Err(ChimeraCompileError::new(
                    "Did not get a valid key for a key value pair",
                    assignment_token.line_col(),
                ));
            }
            let key = assignment_token.as_str().to_owned();

            let value_token = key_value_pairs
                .next()
                .ok_or_else(|| no_pairs_panic("KeyValuePair"))?;
            let value = ChimeraScriptAST::parse_rule_to_value(value_token)?;

            let key_value = KeyValuePair { key, value };
            key_val_pairs.push(key_value);
        }
        Ok(Expression::HttpCommand(HttpCommand {
            verb,
            path,
            query_params,
            http_assignments,
            headers,
            key_val_pairs,
        }))
    }

    fn parse_rule_to_expression(pair: Pair<Rule>) -> Result<Expression, ChimeraCompileError> {
        // An Expression is going to contain
        // a. A LiteralValue which will hold some literal
        // b. An HttpCommand which will contain
        //   1. An Http verb
        //   2. The path of an HTTP request
        //   3. Optional list of query params
        //   3. Optional list of body params
        //   4. Optional list of KeyValuePair, which look like `timeout=>60`
        // c. A LIST expression
        // d. A formatted string expression
        if pair.as_rule() != Rule::Expression {
            return Err(ChimeraCompileError::new(
                "Did not get a valid expression",
                pair.line_col(),
            ));
        }
        let mut expression_pairs = pair.into_inner();

        let first_token = expression_pairs
            .next()
            .ok_or_else(|| no_pairs_panic("Expression"))?;
        match first_token.as_rule() {
            Rule::LiteralValue => Ok(Expression::Literal(Self::parse_rule_to_literal_value(
                first_token,
            )?)),
            Rule::HttpCommand => Self::parse_rule_to_http_command(first_token),
            Rule::ListExpression => {
                let mut list_paris = first_token.into_inner();
                let list_expression_kind_token = list_paris
                    .next()
                    .ok_or_else(|| no_pairs_panic("ListExpression"))?;
                match list_expression_kind_token.as_rule() {
                    Rule::ListNew => {
                        let mut list_new_pairs = list_expression_kind_token.into_inner();
                        let mut list_values: Vec<Value> = Vec::new();

                        // Check for potential list items, contained in Rule::CommaSeparatedValues
                        if let Some(mut list_value_token) = list_new_pairs.next() {
                            // A ListNew contains zero or more CommaSeparatedValues, read them all
                            while list_value_token.as_rule() == Rule::CommaSeparatedValues {
                                let mut inner = list_value_token.into_inner();
                                let literal_token = inner
                                    .next()
                                    .ok_or_else(|| no_pairs_panic("CommaSeparatedValues"))?;
                                let value = Self::parse_rule_to_value(literal_token)?;
                                list_values.push(value);
                                list_value_token = list_new_pairs
                                    .next()
                                    .ok_or_else(|| no_pairs_panic("CommaSeparatedValues"))?;
                            }
                            // After all CommaSeparatedValues are read the final pair is going to be a Value
                            let value = Self::parse_rule_to_value(list_value_token)?;
                            list_values.push(value);
                        }

                        Ok(Expression::List(ListExpression::New(list_values)))
                    }
                    Rule::ListCommandExpr => {
                        let mut list_command_expr_tokens = list_expression_kind_token.into_inner();
                        // Save the op pair to parse last as it might depend on the third token to set its value
                        let command_token = list_command_expr_tokens
                            .next()
                            .ok_or_else(|| no_pairs_panic("ListCommandExpr command"))?;
                        let variable_name_token = list_command_expr_tokens
                            .next()
                            .ok_or_else(|| no_pairs_panic("ListCommandExpr variable name"))?;
                        let list_name = Self::parse_rule_to_variable_name(variable_name_token)?;
                        let operation = match list_command_expr_tokens.next() {
                            Some(value_token) => {
                                let value = Self::parse_rule_to_value(value_token)?;
                                match command_token.as_str() {
                                    "APPEND" => ListCommandOperations::MutateOperations(
                                        MutateListOperations::Append(value),
                                    ),
                                    "REMOVE" => ListCommandOperations::MutateOperations(
                                        MutateListOperations::Remove(value),
                                    ),
                                    _ => {
                                        return Err(ChimeraCompileError::new(
                                            "Invalid list command when using a value",
                                            command_token.line_col(),
                                        ))
                                    }
                                }
                            }
                            None => match command_token.as_str() {
                                "LENGTH" => ListCommandOperations::Length,
                                "POP" => ListCommandOperations::MutateOperations(
                                    MutateListOperations::Pop,
                                ),
                                _ => {
                                    return Err(ChimeraCompileError::new(
                                        "Invalid list command when not using a value",
                                        command_token.line_col(),
                                    ))
                                }
                            },
                        };
                        Ok(Expression::List(ListExpression::ListArgument(
                            ListCommand {
                                list_name,
                                operation,
                            },
                        )))
                    }
                    _ => Err(ChimeraCompileError::new(
                        "Did not get a valid list expression",
                        list_expression_kind_token.line_col(),
                    )),
                }
            }
            Rule::FormattedString => Ok(Expression::FormattedString(
                Self::parse_rule_to_formatted_string(first_token)?,
            )),
            _ => Err(ChimeraCompileError::new(
                "Did not get a valid expression",
                first_token.line_col(),
            )),
        }
    }

    fn parse_rule_to_formatted_string(pair: Pair<Rule>) -> Result<Vec<Value>, ChimeraCompileError> {
        if pair.as_rule() != Rule::FormattedString {
            return Err(ChimeraCompileError::new(
                "Did not get a FormattedString",
                pair.line_col(),
            ));
        }
        let mut values: Vec<Value> = Vec::new();
        let inner = pair.into_inner();
        for pair in inner {
            let mut formatted_string_inner = pair.into_inner();
            let inner_value = formatted_string_inner
                .next()
                .expect("A Rule::FormattedStringInner must contain an inner value");
            match inner_value.as_rule() {
                Rule::UserString => values.push(Value::Literal(Literal::String(
                    inner_value.as_str().to_string(),
                ))),
                Rule::VariableValue => {
                    let var_name = Self::parse_rule_to_variable_name(inner_value)?;
                    values.push(Value::Variable(var_name));
                }
                _ => {
                    return Err(ChimeraCompileError::new(
                        "Did not get a valid rule while iterating FormattedString inner",
                        inner_value.line_col(),
                    ))
                }
            }
        }
        Ok(values)
    }
}

#[derive(Debug)]
pub enum Decorator {
    Key(String),
    KeyValue((String, String)),
}

#[derive(Debug)]
pub enum BlockContents {
    Function(Function),
    Statement(Statement),
    Teardown(Teardown),
}

#[derive(Debug)]
pub struct Teardown {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct Function {
    decorators: Vec<Decorator>,
    pub name: String,
    pub block: Vec<BlockContents>,
}

impl Function {
    pub fn has_key(&self, checked_key: &str) -> bool {
        for decorator in &self.decorators {
            match decorator {
                Decorator::Key(self_key) => {
                    if self_key.as_str() == checked_key {
                        return true;
                    }
                }
                _ => continue,
            }
        }
        false
    }

    pub fn is_expected_failure(&self) -> bool {
        self.has_key("expected-failure")
    }

    pub fn has_name(&self, name: &str) -> bool {
        self.name.as_str() == name
    }

    pub fn is_test_function(&self) -> bool {
        self.has_key("test")
    }
}

#[derive(Debug)]
pub enum Statement {
    AssignmentExpr(AssignmentExpr),
    AssertCommand(AssertCommand),
    PrintCommand(Value),
    Expression(Expression),
}

#[derive(Debug)]
pub struct AssignmentExpr {
    pub var_name: String,
    pub expression: Expression,
}

#[derive(Debug)]
pub enum Expression {
    Literal(Literal),
    HttpCommand(HttpCommand),
    List(ListExpression),
    FormattedString(Vec<Value>),
}

#[derive(Debug)]
pub struct AssertCommand {
    pub negate_assertion: bool,
    pub subcommand: AssertSubCommand,
    pub left_value: Value,
    pub right_value: Value,
    pub error_message: Option<Value>,
}

impl From<Statement> for AssertCommand {
    fn from(value: Statement) -> Self {
        match value {
            Statement::AssertCommand(assert_cmd) => assert_cmd,
            _ => panic!("tried to use a Statement as an AssertCommand when it was not one"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Literal(Literal),
    Variable(String),
    FormattedString(Vec<Value>),
}

impl std::str::FromStr for Value {
    type Err = core::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::Literal(Literal::String(s.to_string())))
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Literal(literal) => write!(f, "{}", literal),
            Value::Variable(var_name) => write!(f, "{}", var_name),
            Value::FormattedString(formatted_string) => write!(f, "{:?}", formatted_string),
        }
    }
}

impl Value {
    pub fn error_print(&self) -> String {
        match self {
            Value::Literal(literal) => format!("value '{}'", literal),
            Value::Variable(var_name) => format!("var '{}'", var_name.to_owned()),
            Value::FormattedString(formatted_string) => format!("fmt_str '{:?}'", formatted_string),
        }
    }

    pub fn resolve(
        &self,
        context: &Context,
        variable_map: &VariableMap,
    ) -> Result<Data, ChimeraRuntimeFailure> {
        match self {
            Value::Literal(val) => Ok(Data::from_literal(val.clone())),
            Value::Variable(var_name) => {
                Ok(Self::get_from_var_map(context, var_name, variable_map)?)
            }
            Value::FormattedString(formatted_string) => {
                let mut built_str: String = String::new();
                for value in formatted_string {
                    match value {
                        Self::Literal(literal) => built_str.push_str(literal.to_string().as_str()),
                        Self::Variable(var_name) => {
                            let resolved = Self::get_from_var_map(context, var_name, variable_map)?;
                            let binding = resolved.borrow(context)?;
                            let as_string = binding.to_string();
                            built_str.push_str(as_string.as_str());
                        },
                        Self::FormattedString(_) => return Err(ChimeraRuntimeFailure::InternalError("building a formatted string, got a formatted string inside a formatted string".to_owned()))
                    }
                }
                Ok(Data::from_literal(Literal::String(built_str)))
            }
        }
    }

    fn get_from_var_map(
        context: &Context,
        var_name: &str,
        variable_map: &VariableMap,
    ) -> Result<Data, ChimeraRuntimeFailure> {
        let accessors: Vec<&str> = var_name.split('.').collect();
        let value = variable_map.get(context, accessors[0])?;
        if accessors.len() == 1 {
            Ok(value.clone())
        } else {
            Ok(value.resolve_access(accessors, context)?)
        }
    }
}

#[derive(Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum AssertSubCommand {
    Equals,
    GTE,
    GT,
    LTE,
    LT,
    Status,
    Length,
    Contains,
}

impl std::fmt::Display for AssertSubCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AssertSubCommand::Equals => write!(f, "equal"),
            AssertSubCommand::GTE => write!(f, "be greater than or equal to"),
            AssertSubCommand::GT => write!(f, "be greater than"),
            AssertSubCommand::LTE => write!(f, "be less than or equal to"),
            AssertSubCommand::LT => write!(f, "be less than"),
            AssertSubCommand::Status => write!(f, "have a status code of"),
            AssertSubCommand::Length => write!(f, "have a length of"),
            AssertSubCommand::Contains => write!(f, "to contain"),
        }
    }
}

#[derive(Debug)]
pub struct HttpCommand {
    pub verb: HTTPVerb,
    pub path: Vec<Value>,
    pub query_params: Vec<HttpAssignment>,
    pub http_assignments: Vec<HttpAssignment>,
    pub headers: Vec<HttpAssignment>,
    // TODO: REMOVE THIS WHEN IMPLEMENTING KV PAIRS
    #[allow(dead_code)]
    key_val_pairs: Vec<KeyValuePair>,
}

impl From<Statement> for HttpCommand {
    fn from(value: Statement) -> Self {
        match value {
            Statement::Expression(expr) => match expr {
                Expression::HttpCommand(http_command) => http_command,
                _ => panic!("tried to use an Expression as an HttpCommand when it was not one"),
            },
            _ => panic!("tried to use a Statement as an Expression when it was not one"),
        }
    }
}

impl HttpCommand {
    pub fn resolve_path(
        &self,
        context: &Context,
        variable_map: &VariableMap,
    ) -> Result<String, ChimeraRuntimeFailure> {
        let client = CLIENT
            .get()
            .expect("Failed to get web client while resolving an HTTP expression");
        let mut resolved_path: String = client.get_domain().to_owned();
        for portion in &self.path {
            match portion
                .resolve(context, variable_map)?
                .borrow(context)?
                .deref()
            {
                DataKind::Literal(literal) => resolved_path.push_str(literal.to_string().as_str()),
                DataKind::Collection(_) => {
                    return Err(ChimeraRuntimeFailure::VarWrongType(
                        portion.error_print(),
                        VarTypes::Literal,
                        context.current_line,
                    ))
                }
            }
        }
        let mut is_first_param = true;
        for query_param in &self.query_params {
            if is_first_param {
                resolved_path.push('?');
                is_first_param = false;
            } else {
                resolved_path.push('&');
            }
            let data = query_param.rhs.resolve(context, variable_map)?;
            let borrowed_data = data.borrow(context)?;
            // TODO: This currently allows for a collection var to be used here, is that actually what I want? Should this error?
            let formatted = format!("{}={}", query_param.lhs, borrowed_data);
            resolved_path.push_str(formatted.as_str());
        }
        Ok(resolved_path)
    }
    pub fn resolve_body(
        &self,
        context: &Context,
        variable_map: &VariableMap,
    ) -> Result<HashMap<String, String>, ChimeraRuntimeFailure> {
        let mut body_map: HashMap<String, String> = HashMap::new();
        for assignment in &self.http_assignments {
            let key = assignment.lhs.clone();
            let value = assignment
                .rhs
                .resolve(context, variable_map)?
                .borrow(context)?
                .to_string();
            body_map.insert(key, value);
        }
        Ok(body_map)
    }
    pub fn resolve_header(
        &self,
        context: &Context,
        variable_map: &VariableMap,
    ) -> Result<HeaderMap, ChimeraRuntimeFailure> {
        let mut headers: HeaderMap = HeaderMap::new();
        for pair in &self.headers {
            let header_name = match HeaderName::from_lowercase(pair.lhs.as_bytes()) {
                Ok(valid) => valid,
                Err(_) => {
                    return Err(ChimeraRuntimeFailure::InvalidHeader(
                        context.current_line,
                        pair.lhs.clone(),
                    ))
                }
            };
            let value = match pair
                .rhs
                .resolve(context, variable_map)?
                .borrow(context)?
                .to_string()
                .parse()
            {
                Ok(val) => val,
                Err(_) => {
                    return Err(ChimeraRuntimeFailure::InternalError(
                        "resolving header value".to_owned(),
                    ))
                }
            };
            headers.insert(header_name, value);
        }
        Ok(headers)
    }
}

#[derive(Debug)]
pub struct HttpAssignment {
    pub lhs: String,
    pub rhs: Value,
}

#[derive(Debug)]
// TODO: REMOVE ALLOW DEAD CODE WHEN IMPLEMENTING KV PAIRS, HERE AND IN HTTP COMMAND STRUCT
#[allow(dead_code)]
pub struct KeyValuePair {
    key: String,
    value: Value,
}

#[derive(Debug)]
pub enum ListExpression {
    New(Vec<Value>),
    ListArgument(ListCommand),
}

#[derive(Debug)]
pub struct ListCommand {
    pub list_name: String,
    pub operation: ListCommandOperations,
}

#[derive(Debug)]
pub enum ListCommandOperations {
    MutateOperations(MutateListOperations),
    Length,
}

#[derive(Debug)]
pub enum MutateListOperations {
    Append(Value),
    Remove(Value),
    Pop,
}

impl From<Statement> for ListExpression {
    fn from(value: Statement) -> Self {
        match value {
            Statement::Expression(expr) => match expr {
                Expression::List(list_command) => list_command,
                _ => panic!("tried to use an Expression as a ListExpression when it was not one"),
            },
            _ => panic!("tried to use a Statement as an Expression when it was not one"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum HTTPVerb {
    Get,
    Put,
    Post,
    Delete,
}

/*
-------------------------------------------------------------------------------------------------
Here be testing
-------------------------------------------------------------------------------------------------
 */

#[cfg(test)]
mod ast_tests {
    use super::*;
    use crate::frontend::CScriptTokenPairs;
    use pest::Parser;
    use std::str::FromStr;

    // TODO: Add tests here for a test-case functions, decorators, teardown, nested functions

    fn str_to_statement(input: &str) -> Statement {
        let mut pairs = CScriptTokenPairs::parse(Rule::Statement, input)
            .expect("Failed to parse a ChimeraScript string with Pest.");
        let statement_pair = pairs
            .next()
            .expect("Did not get any pairs after parsing a string with a Rule::Statement");
        ChimeraScriptAST::pair_to_statement(statement_pair)
            .expect("Failed to convert Pair<Rule> into a Statement")
    }

    #[test]
    /// Test the simplest possible assertion, 1 == 1, resolves to be an AssertCommand for two literals
    fn simple_parse() {
        match str_to_statement("ASSERT EQUALS 1 1;") {
            Statement::AssertCommand(assert_command) => {
                assert_eq!(assert_command.negate_assertion, false, "negate_assertion should be false for an assertion which does not contain 'NOT'.");
                assert_eq!(
                    assert_command.subcommand,
                    AssertSubCommand::Equals,
                    "Assertion using EQUALS should have an AssertSubCommand::Equals subcommand."
                );
                assert_eq!(
                    assert_command.left_value,
                    Value::Literal(Literal::Number(NumberKind::U64(1))),
                    "Assertion with a numerical literal should have a Literal::Int() value."
                );
                assert_eq!(
                    assert_command.right_value,
                    Value::Literal(Literal::Number(NumberKind::U64(1)))
                );
                assert!(
                    assert_command.error_message.is_none(),
                    "Assertion error_message should be None when no message is specified."
                );
            }
            _ => panic!("An ASSERT statement was not resolved into a Statement::AssertCommand."),
        }
    }

    #[test]
    /// Test that a statement can take place over multiple lines
    fn multiline_statement() {
        // This test fails if it panics while parsing
        str_to_statement("ASSERT EQUALS \n\n 1 \n 1;");
    }

    #[test]
    /// Test that comments work as expected. Comments should be ignored and not generate any rule pairs
    fn comments() {
        let assertion_with_comment: AssertCommand =
            str_to_statement("ASSERT EQUALS 1 1; //this assertion ends with a comment").into();
        assert_eq!(assertion_with_comment.subcommand, AssertSubCommand::Equals);

        let assertion_with_midline_comment: AssertCommand =
            str_to_statement("ASSERT EQUALS 1 /*this is a midline comment*/ 1;").into();
        assert_eq!(
            assertion_with_midline_comment.subcommand,
            AssertSubCommand::Equals
        );
        assert_eq!(
            assertion_with_midline_comment.left_value,
            Value::Literal(Literal::Number(NumberKind::U64(1)))
        );
        assert_eq!(
            assertion_with_midline_comment.right_value,
            Value::Literal(Literal::Number(NumberKind::U64(1)))
        );
    }

    #[test]
    /// Test an EQUALS assertion which is negated and has an error message
    fn full_equality_assertion() {
        match str_to_statement("ASSERT NOT EQUALS 1 2 \"foo\";") {
            Statement::AssertCommand(assert_command) => {
                assert!(
                    assert_command.negate_assertion,
                    "negate_assertion should be true for an assertion which contains 'NOT'."
                );
                assert_eq!(
                    assert_command.subcommand,
                    AssertSubCommand::Equals,
                    "Assertion using EQUALS should have an AssertSubCommand::Equals subcommand."
                );
                assert_eq!(assert_command.left_value, Value::Literal(Literal::Number(NumberKind::U64(1))), "Assertion with a numerical literal should have a Value::Literal(Literal::Int()) value.");
                assert_eq!(
                    assert_command.right_value,
                    Value::Literal(Literal::Number(NumberKind::U64(2)))
                );
                assert!(
                    assert_command.error_message.is_some(),
                    "Assertion error_message should be Some() when message is specified."
                );
                assert_eq!(
                    assert_command.error_message.unwrap(),
                    Value::Literal(Literal::String("foo".to_owned())),
                    "Assertion error message was not a Literal::String equal to the supplied message"
                );
            }
            _ => panic!("An ASSERT statement was not resolved into a Statement::AssertCommand"),
        }
    }

    #[test]
    /// Test an ASSERT command which uses formatted string
    fn formatted_string_assertion() {
        match str_to_statement("ASSERT EQUALS \"normal str\" \"(var_str)\" \"This is an error (message) with words\";") {
            Statement::AssertCommand(assert_command) => {
                assert_eq!(
                    assert_command.left_value,
                    Value::Literal(Literal::String("normal str".to_owned())),
                    "Assertion lhs should be a string literal when using a QuoteString"
                );
                assert_eq!(
                    assert_command.right_value,
                    Value::FormattedString(vec![Value::Variable("var_str".to_owned())]),
                    "Assertion rhs should be a Value::FormattedString of length 1 when using a FormattedString"
                );
                assert!(assert_command.error_message.is_some());
                assert_eq!(
                    assert_command.error_message.unwrap(),
                    Value::FormattedString(vec![
                        Value::Literal(Literal::String("This is an error ".to_owned())),
                        Value::Variable("message".to_owned()),
                        Value::Literal(Literal::String(" with words".to_owned()))
                    ])
                );
            },
            _ => panic!("An ASSERT statement was not resolved into a Statement::AssertCommand when using formatted strings"),
        }
    }

    #[test]
    /// Test a FORMAT_STR expression
    fn formatted_string_expression() {
        match str_to_statement(
            "FORMAT_STR \"this is a (formatted) string with some (variable_names)\";",
        ) {
            Statement::Expression(expression) => match expression {
                Expression::FormattedString(formatted_string) => {
                    assert_eq!(
                            formatted_string.len(),
                            4,
                            "FormattedString should have a length of 4 when written in the format of 'string -> var -> string -> var'"
                        );
                    assert_eq!(
                        formatted_string,
                        vec![
                            Value::Literal(Literal::String("this is a ".to_owned())),
                            Value::Variable("formatted".to_owned()),
                            Value::Literal(Literal::String(" string with some ".to_owned())),
                            Value::Variable("variable_names".to_owned())
                        ]
                    );
                }
                _ => panic!(
                    "A FORMAT_STR statement was not resolved into an Expression::FormattedString"
                ),
            },
            _ => panic!("A FORMAT_STR statement was not resolved into a Statement::Expression"),
        }
    }

    #[test]
    /// Test that strings can use more characters than just ASCII letters and numbers
    fn special_string_characters() {
        match str_to_statement("LITERAL \"this is a, wordy, string. It has punctuation! Wow!@#$%^&*-=+<>?\";") {
            Statement::Expression(expression) => {
                match expression {
                    Expression::Literal(literal) => {
                        match literal {
                            Literal::String(literal_string) => {
                                assert_eq!(
                                    literal_string,
                                    "this is a, wordy, string. It has punctuation! Wow!@#$%^&*-=+<>?".to_owned(),
                                    "Failed to assert equality on a string which uses special characters"
                                );
                            },
                            _ => panic!("A LITERAL statement with a string containing special characters was not resolved into a Literal::String")
                        }
                    },
                    _ => panic!("A LITERAL statement was not resolved into an Expression::Literal")
                }
            },
            _ => panic!("A LITERAL statement was not resolved into a Statement::Expression")
        }
    }

    #[test]
    /// Test LITERAL values
    fn literal_values() {
        let trees: Vec<Literal> = [
            "LITERAL \"this is a string\";",
            "LITERAL \"foo\";",
            "LITERAL 5;",
            "LITERAL 0;",
            "LITERAL -10;",
            "LITERAL 5.5;",
            "LITERAL -10.7;",
            "LITERAL true;",
            "LITERAL True;",
            "LITERAL false;",
            "LITERAL False;",
            "LITERAL null;",
            "LITERAL Null;",
        ]
        .into_iter()
        .map(|x| str_to_statement(x).into())
        .collect();
        assert_eq!(trees.len(), 13);
        assert_eq!(trees[0], Literal::String("this is a string".to_owned()));
        assert_eq!(trees[1], Literal::String("foo".to_owned()));
        assert_eq!(trees[2], Literal::Number(NumberKind::U64(5u64)));
        assert_eq!(trees[3], Literal::Number(NumberKind::U64(0u64)));
        assert_eq!(trees[4], Literal::Number(NumberKind::I64(-10i64)));
        assert_eq!(trees[5], Literal::Number(NumberKind::F64(5.5f64)));
        assert_eq!(trees[6], Literal::Number(NumberKind::F64(-10.7f64)));
        assert_eq!(trees[7], Literal::Bool(true));
        assert_eq!(trees[8], Literal::Bool(true));
        assert_eq!(trees[9], Literal::Bool(false));
        assert_eq!(trees[10], Literal::Bool(false));
        assert_eq!(trees[11], Literal::Null);
        assert_eq!(trees[12], Literal::Null);
    }

    #[test]
    /// Test the ASSERT subcommands
    fn assertion_subcommands() {
        let trees: Vec<AssertCommand> = [
            "ASSERT EQUALS 1 1;",
            "ASSERT GTE 1 1;",
            "ASSERT GT 1 1;",
            "ASSERT LTE 1 1;",
            "ASSERT LT 1 1;",
            "ASSERT STATUS 1 1;",
            "ASSERT LENGTH (foo) 1;",
            "ASSERT CONTAINS (foo) 1;",
        ]
        .into_iter()
        .map(|x| str_to_statement(x).into())
        .collect();
        assert_eq!(trees.len(), 8);
        assert_eq!(trees[0].subcommand, AssertSubCommand::Equals);
        assert_eq!(trees[1].subcommand, AssertSubCommand::GTE);
        assert_eq!(trees[2].subcommand, AssertSubCommand::GT);
        assert_eq!(trees[3].subcommand, AssertSubCommand::LTE);
        assert_eq!(trees[4].subcommand, AssertSubCommand::LT);
        assert_eq!(trees[5].subcommand, AssertSubCommand::Status);
        assert_eq!(trees[6].subcommand, AssertSubCommand::Length);
        assert_eq!(trees[7].subcommand, AssertSubCommand::Contains);
    }

    #[test]
    /// Test assertions with each of the Value variants
    fn assertion_values() {
        let assertion: AssertCommand = str_to_statement("ASSERT EQUALS (foo) 1;").into();
        assert_eq!(assertion.left_value, Value::Variable("foo".to_owned()));
        assert_eq!(
            assertion.right_value,
            Value::Literal(Literal::Number(NumberKind::U64(1)))
        );
    }

    #[test]
    /// Test a PRINT statement
    fn print_statement() {
        match str_to_statement("PRINT 5;") {
            Statement::PrintCommand(val) => {
                assert_eq!(val, Value::Literal(Literal::Number(NumberKind::U64(5))))
            }
            _ => panic!("Statement for a PRINT did not resolve to the correct variant."),
        }
    }

    #[test]
    /// Test a simple assignment with a literal expression
    fn assignment_expression() {
        match str_to_statement("var foo = LITERAL 5;") {
            Statement::AssignmentExpr(assignment_expr) => {
                assert_eq!(assignment_expr.var_name, "foo".to_owned());
                match assignment_expr.expression {
                    Expression::Literal(literal_expression) => {
                        match literal_expression {
                            Literal::Number(numberkind) => {
                                assert_eq!(numberkind, NumberKind::U64(5));
                            },
                            _ => panic!("Expected a LITERAL expression for a '5' to resolve as a Literal::Number but it did not")
                        }
                    },
                    _ => panic!("Expected a LITERAL expression to resolve as an Expression::LiteralExpression but it did not")
                }
            }
            _ => panic!(
                "Statement for an assignment expression did not resolve to the correct variant."
            ),
        }
    }

    #[test]
    /// Test an Http command expression
    fn http_expression() {
        // Basic commands for each HTTP verb
        let http_commands: Vec<HttpCommand> =
            ["GET /foo/bar;", "PUT /foo;", "POST /foo;", "DELETE /foo;"]
                .into_iter()
                .map(|x| str_to_statement(x).into())
                .collect();
        assert_eq!(http_commands.len(), 4);
        assert_eq!(http_commands[0].verb, HTTPVerb::Get);
        assert_eq!(
            http_commands[0].path,
            vec![Value::from_str("/foo/bar").unwrap()]
        );
        assert_eq!(http_commands[1].verb, HTTPVerb::Put);
        assert_eq!(http_commands[2].verb, HTTPVerb::Post);
        assert_eq!(http_commands[3].verb, HTTPVerb::Delete);

        // HTTP expression with query params of varying types
        let with_path_assignments: HttpCommand = str_to_statement(
            "GET /foo/bar/baz?foo=5&another=\"bar\"&boolean=true&with_var=(some_var);",
        )
        .into();
        assert_eq!(
            with_path_assignments.path,
            vec![Value::from_str("/foo/bar/baz").unwrap()]
        );
        assert_eq!(with_path_assignments.query_params.len(), 4);
        assert_eq!(with_path_assignments.query_params[0].lhs, "foo");
        assert_eq!(
            with_path_assignments.query_params[0].rhs,
            Value::Literal(Literal::Number(NumberKind::U64(5)))
        );
        assert_eq!(with_path_assignments.query_params[1].lhs, "another");
        assert_eq!(
            with_path_assignments.query_params[1].rhs,
            Value::Literal(Literal::String("bar".to_owned()))
        );
        assert_eq!(with_path_assignments.query_params[2].lhs, "boolean");
        assert_eq!(
            with_path_assignments.query_params[2].rhs,
            Value::Literal(Literal::Bool(true))
        );
        assert_eq!(with_path_assignments.query_params[3].lhs, "with_var");
        assert_eq!(
            with_path_assignments.query_params[3].rhs,
            Value::Variable("some_var".to_owned())
        );

        // HTTP command with a path, query params, body params, and key/value pairs
        let full_expression: HttpCommand = str_to_statement("GET /foo/bar/baz?foo=5&another=\"bar\" some_num=5 some_str=\"value\" timeout=>60 boolKey=>false;").into();
        assert_eq!(full_expression.verb, HTTPVerb::Get);
        assert_eq!(
            full_expression.path,
            vec![Value::from_str("/foo/bar/baz").unwrap()]
        );
        assert_eq!(full_expression.query_params.len(), 2);
        assert_eq!(full_expression.query_params[0].lhs, "foo");
        assert_eq!(
            full_expression.query_params[0].rhs,
            Value::Literal(Literal::Number(NumberKind::U64(5)))
        );
        assert_eq!(full_expression.query_params[1].lhs, "another");
        assert_eq!(
            full_expression.query_params[1].rhs,
            Value::Literal(Literal::String("bar".to_owned()))
        );
        assert_eq!(full_expression.http_assignments.len(), 2);
        assert_eq!(
            full_expression.http_assignments[0].lhs,
            "some_num".to_owned()
        );
        assert_eq!(
            full_expression.http_assignments[0].rhs,
            Value::Literal(Literal::Number(NumberKind::U64(5)))
        );
        assert_eq!(full_expression.key_val_pairs.len(), 2);
        assert_eq!(full_expression.key_val_pairs[0].key, "timeout".to_owned());
        assert_eq!(
            full_expression.key_val_pairs[0].value,
            Value::Literal(Literal::Number(NumberKind::U64(60)))
        );

        // HTTP command where there are multiple variables in the path
        let endpoint_with_variable: HttpCommand =
            str_to_statement("GET /foo/beginning(some_var)end/(another_var)/ending?foo=5&bar=10;")
                .into();
        assert_eq!(endpoint_with_variable.path.len(), 5);
        assert_eq!(
            endpoint_with_variable.path[0],
            Value::from_str("/foo/beginning").unwrap()
        );
        assert_eq!(
            endpoint_with_variable.path[1],
            Value::Variable("some_var".to_owned())
        );
        assert_eq!(
            endpoint_with_variable.path[2],
            Value::from_str("end/").unwrap()
        );
        assert_eq!(
            endpoint_with_variable.path[3],
            Value::Variable("another_var".to_owned())
        );
        assert_eq!(
            endpoint_with_variable.path[4],
            Value::from_str("/ending").unwrap()
        );
        assert_eq!(endpoint_with_variable.query_params.len(), 2);
        assert_eq!(endpoint_with_variable.query_params[0].lhs, "foo");
        assert_eq!(
            endpoint_with_variable.query_params[0].rhs,
            Value::Literal(Literal::Number(NumberKind::U64(5)))
        );
        assert_eq!(endpoint_with_variable.query_params[1].lhs, "bar");
        assert_eq!(
            endpoint_with_variable.query_params[1].rhs,
            Value::Literal(Literal::Number(NumberKind::U64(10)))
        );

        // HTTP command with a header
        let endpoint_with_header: HttpCommand =
            str_to_statement("GET /foo/bar authorization:\"bar\" age:(some_var);").into();
        assert_eq!(endpoint_with_header.path.len(), 1);
        assert_eq!(
            endpoint_with_header.path[0],
            Value::from_str("/foo/bar").unwrap()
        );
        assert_eq!(endpoint_with_header.headers.len(), 2);
        assert_eq!(
            endpoint_with_header.headers[0].lhs,
            "authorization".to_owned()
        );
        assert_eq!(
            endpoint_with_header.headers[0].rhs,
            Value::Literal(Literal::String("bar".to_owned()))
        );
        assert_eq!(endpoint_with_header.headers[1].lhs, "age".to_owned());
        assert_eq!(
            endpoint_with_header.headers[1].rhs,
            Value::Variable("some_var".to_owned())
        );
    }

    #[test]
    /// Test the LIST command
    fn list_expression() {
        // Test that a list can be created without any initial data
        let new_empty_list: ListExpression = str_to_statement("LIST NEW [];").into();
        match new_empty_list {
            ListExpression::New(list_values) => {
                assert_eq!(list_values.len(), 0, "Expected a list created with zero elements in it to have a length of 0 but it was not")
            }
            _ => {
                panic!("Expected a ListExpression::New when making a new list but did not get one")
            }
        }

        // Test creating a new list with different types of data as initial values
        let new_list_expression: ListExpression =
            str_to_statement("LIST NEW [1, true, \"hello world\", (my_var)];").into();
        match new_list_expression {
            ListExpression::New(list_values) => {
                assert_eq!(list_values.len(), 4, "Expected list values to contain 4 values when 4 were provided to LIST NEW");
                assert_eq!(list_values[0], Value::Literal(Literal::Number(NumberKind::U64(1))), "When passing a 1 as the first list value, should have gotten a Value Literal Int");
                assert_eq!(list_values[1], Value::Literal(Literal::Bool(true)), "When passing a true as the second list value, should have gotten a Value Literal Bool");
                assert_eq!(list_values[2], Value::Literal(Literal::String("hello world".to_owned())), "When passing a \"hello world\" as the third list value, should have gotten a Value Literal Str");
                assert_eq!(list_values[3], Value::Variable("my_var".to_string()), "When passing a (my_var) as the fourth list value, should have gotten a Value Variable");
            }
            ListExpression::ListArgument(_) => panic!("Got a ListExpression::ListArgument variant when a ListExpression::New was expected")
        }

        // Test appending a literal to a list
        let list_append_expression: ListExpression =
            str_to_statement("LIST APPEND (my_list) 5;").into();
        match list_append_expression {
            ListExpression::New(_) => panic!("Got a ListExpression::New variant when a ListExpression::ListArgument was expected"),
            ListExpression::ListArgument(list_command) => {
                assert_eq!(list_command.list_name.as_str(), "my_list", "Expected ListCommand to have a list_name of my_list when the command used that as the list variable name");
                match list_command.operation {
                    ListCommandOperations::MutateOperations(mutable_list_operations) => {
                        match mutable_list_operations {
                            MutateListOperations::Append(append_val) => {
                                assert_eq!(append_val, Value::Literal(Literal::Number(NumberKind::U64(5))), "Expected ListCommand's Append operation to contain a Literal Int 5 when the APPEND command was given a 5");
                            },
                            _ => panic!("Expected ListCommand operation to be a MutableOperation Append when using an APPEND command but it wasn't")
                        }
                    },
                    _ => panic!("Expected ListCommand's operation field to be of the Append variant when using an APPEND command but it wasn't")
                }
            }
        }

        // Test removing an item from a list
        let list_remove_expression: ListExpression =
            str_to_statement("LIST REMOVE (my_list) 10;").into();
        match list_remove_expression {
            ListExpression::New(_) => panic!("Got a ListExpression::New variant when a ListExpression::ListArgument was expected"),
            ListExpression::ListArgument(list_command) => {
                assert_eq!(list_command.list_name.as_str(), "my_list", "Expected ListCommand to have a list_name of my_list when the command used that as the list variable name");
                match list_command.operation {
                    ListCommandOperations::MutateOperations(mutable_list_operations) => {
                        match mutable_list_operations {
                            MutateListOperations::Remove(remove_val) => {
                                assert_eq!(remove_val, Value::Literal(Literal::Number(NumberKind::U64(10))), "Expected ListCommand's Remove operation to contain a Literal Int 10 when the REMOVE command was given a 10");
                            },
                            _ => panic!("Expected ListCommand operation to be a MutableOperation Remove when using a REMOVE command but it wasn't")
                        }
                    },
                    _ => panic!("Expected ListCommand's operation field to be of the Remove variant when using an REMOVE command but it wasn't")
                }
            }
        }

        // Test getting the length of a list
        let list_length_expression: ListExpression =
            str_to_statement("LIST LENGTH (some_list);").into();
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

        // Test popping an item off of a list
        let list_pop_expression: ListExpression = str_to_statement("LIST POP (some_list);").into();
        match list_pop_expression {
            ListExpression::New(_) => panic!("Got a ListExpression::New variant when a ListExpression::ListArgument was expected"),
            ListExpression::ListArgument(list_command) => {
                assert_eq!(list_command.list_name.as_str(), "some_list", "Expected ListCommand to have a list_name of some_list when the command used that as the list variable name");
                match list_command.operation {
                    ListCommandOperations::MutateOperations(mutable_operation) => {
                        match mutable_operation {
                            MutateListOperations::Pop => (),
                            _ => panic!("Expected MutableOperations to be of the Pop variant when using a POP command but it wasn't")
                        }
                    },
                    _ => panic!("Expected ListCommand's operation field to be of the MutateOperations variant when using a POP command but it wasn't")
                }
            }
        }

        // Test that a list cannot be created with an invalid comma separation
        let failure_res = std::panic::catch_unwind(|| {
            str_to_statement("LIST NEW [1,];");
        });
        assert!(
            failure_res.is_err(),
            "Expected list creation to fail when the only value passed in was comma separated"
        );
    }

    #[test]
    /// Test that a statement without an EndOf fails to parse
    fn no_end_of_statement() {
        let failure_res = std::panic::catch_unwind(|| str_to_statement("ASSERT EQUALS 1 1"));
        assert!(
            failure_res.is_err(),
            "Expected a statement with no EndOf to fail to parse"
        );
    }
}
