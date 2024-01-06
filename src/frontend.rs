use std::collections::HashMap;
use std::fmt::Display;
use pest::iterators::Pairs;
use pest::Parser;
use pest_derive::Parser;
use crate::abstract_syntax_tree::{AssignmentValue, ChimeraScriptAST, Statement, Function, BlockContents};
use crate::err_handle::{ChimeraCompileError, ChimeraRuntimeFailure};

pub struct Context {
    pub current_line: i32
}

impl Context {
    pub fn new() -> Self {
        Self { current_line: 0 }
    }
}

#[derive(Parser, Debug)]
#[grammar = "grammar.pest"]
pub struct CScriptTokenPairs;

/// Parse a string with Pest using the Main rule
pub fn parse_main(input: &str) -> Result<Pairs<Rule>, ChimeraCompileError> {
    match CScriptTokenPairs::parse(Rule::Main, input) {
        Ok(parsed) => Ok(parsed),
        Err(e) => {
            return Err(handle_ast_err(e))
        }
    }
}

// TODO: Have to support running a test by name. Should just add a new function for it. Search an ast.functions
//       for a test/nested-test of a given name and then run that test and its direct parents back to the top
//       of the stack to the outermost test
pub fn run_functions(ast: ChimeraScriptAST, web_client: &reqwest::blocking::Client) -> (usize, usize, usize) {
    let mut tests_passed = 0;
    let mut tests_failed = 0;
    let mut tests_errored = 0;
    for function in ast.functions {
        if function.is_test_function() {
            let mut function_variables: HashMap<String, AssignmentValue> = HashMap::new();
            match run_test_function(function, &mut function_variables, 0, web_client) {
                Ok((passed, failed, errored)) => { tests_passed += passed; tests_failed += failed; tests_errored += errored },
                Err(runtime_error) => { runtime_error.print_error(); tests_errored += 1; }
            }
        }
    }
    (tests_passed, tests_failed, tests_errored)
}

pub fn print_in_function(thing: &impl Display, depth: usize) {
    // TODO: Is there a better way to display in a function?
    for _ in 0..depth {
        print!(" ");
    }
    println!("{}", thing);
}

pub fn print_function_error(e: ChimeraRuntimeFailure, depth: usize) {
    // TODO: This is hacky, find a better solution for printing errors at the correct depth
    //       Maybe pass in a formatter object to print_error() which handles printing
    //       in the right formatting. See the to-do above the print_error() function
    for _ in 0..(depth + 1) {
        eprint!(" ");
    }
    e.print_error();
}

// TODO: Should variable scoping be added? How will this impact the teardown stack (if teardown is added by called non-
//       test functions)?
pub fn run_test_function(function: Function, variable_map: &mut HashMap<String, AssignmentValue>, depth: usize, web_client: &reqwest::blocking::Client) -> Result<(usize, usize, usize), ChimeraRuntimeFailure> {
    print_in_function(&format!("RUNNING TEST {}", function.name), depth);
    let mut context = Context::new();
    // TODO: If the ability to call functions is added (like calling an init function) the teardown stack needs to be
    //       passed as a mut reference into that function so it can add teardown to the stack. Should only be able
    //       to call non-test functions?
    let mut teardown_stack: Vec<Statement> = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut errored = 0;
    let mut this_test_passed = true;

    // Get these two variables here as they are needed at the end and the for..in.. is about to consume function
    let is_expected_failure = function.is_expected_failure();
    let function_name = function.name;

    for block_contents in function.block {
        match block_contents {
            BlockContents::Function(nested_function) => {
                match run_test_function(nested_function, variable_map, depth + 1, web_client) {
                    Ok(res) => { passed += res.0; failed += res.1 },
                    Err(e) => { print_function_error(e, depth); errored += 1 }
                }
            },
            BlockContents::Teardown(mut teardown_block) => {
                // TODO: Swap any Value::Variable uses in each statement for a Value::Literal to "stabilize" the
                //       teardown statement against any variable changes during the test
                teardown_stack.append(&mut teardown_block.statements);
            },
            BlockContents::Statement(statement) => {
                // Run statement
                // Match on the specific kind of runtime failure. If we have a TestFailure then we want to mark
                // this_test_passed, print the failure, and continue.
                // If we have any other runtime error, just return the error
                let statement_result = match statement {
                    Statement::AssertCommand(assert_command) => {
                        crate::commands::assert::assert_command(&context, assert_command, variable_map)
                    },
                    Statement::AssignmentExpr(assert_expr) => {
                        crate::commands::assignment::assignment_command(&context, assert_expr, variable_map, web_client)
                    },
                    Statement::PrintCommand(print_cmd) => {
                        crate::commands::print::print_command(&context, print_cmd, variable_map, depth)
                    },
                    Statement::Expression(expr) => {
                        // We are running an expression without assigning it, we can toss the result
                        match crate::commands::expression::expression_command(&context, expr, variable_map, web_client) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e)
                        }
                    }
                };
                match statement_result {
                    Ok(_) => (),
                    Err(runtime_error) => {
                        match runtime_error {
                            ChimeraRuntimeFailure::TestFailure(_, _) => {
                                this_test_passed = false;
                                print_function_error(runtime_error, depth);
                                break;
                            },
                            // TODO: Need to still process teardown even here
                            _ => return Err(runtime_error)
                        }
                    }
                }
            }
        };
        context.current_line += 1;
    };

    // TODO: When the test function ends, process the teardown stack

    match this_test_passed {
        true => {
            passed += 1;
            match is_expected_failure {
                true => print_in_function(&format!("TEST {} UNEXPECTED SUCCESS", function_name), depth),
                false => print_in_function(&format!("TEST {} PASSED", function_name), depth)
            }
        },
        false => {
            match is_expected_failure {
                true => {
                    passed += 1;
                    print_in_function(&format!("TEST {} EXPECTED FAILURE", function_name), depth);
                },
                false => {
                    failed += 1;
                    print_in_function(&format!("TEST {} FAILED", function_name), depth)
                }
            }
        }
    }
    Ok((passed, failed, errored))
}

fn handle_ast_err(e: pest::error::Error<Rule>) -> ChimeraCompileError {
    let line_col = match e.line_col {
        pest::error::LineColLocation::Pos(pos) => pos,
        pest::error::LineColLocation::Span(start, _end) => start
    };
    ChimeraCompileError::new("Failed to parse ChimeraScript", line_col)
}
