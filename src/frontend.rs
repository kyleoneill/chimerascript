use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::iter::Sum;
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

pub enum Status {
    Success,
    Failure(ChimeraRuntimeFailure),
    Error(ChimeraRuntimeFailure),
    ExpectedFailure,
    UnexpectedSuccess,
    Skip
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Success => write!(f, "SUCCESS"),
            Status::Failure(_) => write!(f, "FAILURE"),
            Status::Error(_) => write!(f, "ERROR"),
            Status::ExpectedFailure => write!(f, "EXPECTED FAILURE"),
            Status::UnexpectedSuccess => write!(f, "UNEXPECTED SUCCESS"),
            Status::Skip => write!(f, "SKIP")
        }
    }
}

pub struct ResultCount {
    success: usize,
    failure: usize,
    error: usize,
    total_tests: usize
}

impl Display for ResultCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let overall_result = if self.failure == 0 && self.error == 0 {"PASSED"} else {"FAILED"};
        write!(f, "Ran {} tests with {} successes, {} failures, and {} errors\n\nTEST SUITE {}", self.total_tests, self.success, self.failure, self.error, overall_result)
    }
}

impl ResultCount {
    fn new_empty() -> Self {
        Self { success: 0, failure: 0, error: 0, total_tests: 0 }
    }
    pub fn new(input: (usize, usize, usize, usize)) -> Self {
        Self { success: input.0, failure: input.1, error: input.2, total_tests: input.3 }
    }
    pub fn print_test_result(results: Vec<TestResult>) {
        let result_count: ResultCount = results.iter().map(|x| x.get_result_counts()).sum();
        println!("{}", result_count);
    }
}

impl std::ops::Add for ResultCount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            success: self.success + rhs.success,
            failure: self.failure + rhs.failure,
            error: self.failure + rhs.failure,
            total_tests: self.total_tests + rhs.total_tests
        }
    }
}

impl Sum for ResultCount {
    fn sum<I: Iterator<Item=Self>>(iter: I) -> Self {
        iter.fold(Self::new_empty(), |acc, x| acc + x)
    }
}

// TODO: Ability to turn this into structured output for testing/CI? Ex, convert this into JSON
#[allow(dead_code)] // "dead" field by tests
pub struct TestResult {
    name: String,
    status: Status,
    pub subtest_results: Vec<TestResult>
}

impl TestResult {
    pub fn new(name: String, status: Status, subtest_results: Vec<Self>) -> Self {
        Self { name, status, subtest_results }
    }
    pub fn get_result_counts(&self) -> ResultCount {
        let res = ResultCount::new(match self.status {
            Status::Success => (1,0,0,1),
            Status::Failure(_) => (0,1,0,1),
            Status::Error(_) => (0,0,1,1),
            Status::ExpectedFailure => (1,0,0,1),
            Status::UnexpectedSuccess => (1,0,0,1),
            Status::Skip => (0,0,0,0)
        });
        res + self.subtest_results.iter().map(|x| x.get_result_counts()).sum()
    }

    #[allow(dead_code)] // Used by tests
    pub fn passed(&self) -> bool {
        match self.status {
            Status::Success => true,
            Status::UnexpectedSuccess => true,
            Status::ExpectedFailure => true,
            _ => false
        }
    }

    #[allow(dead_code)] // Used by tests
    pub fn error_kind(&self) -> Option<&ChimeraRuntimeFailure> {
        match &self.status {
            Status::Error(e) => Some(e),
            Status::Failure(f) => Some(f),
            _ => None
        }
    }

    #[allow(dead_code)] // Used by tests
    pub fn test_name(&self) -> &str {
        self.name.as_str()
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
pub fn run_functions(ast: ChimeraScriptAST, web_client: &reqwest::blocking::Client) -> Vec<TestResult> {
    let mut results: Vec<TestResult> = Vec::new();
    for function in ast.functions {
        if function.is_test_function() {
            let mut function_variables: HashMap<String, AssignmentValue> = HashMap::new();
            results.push(run_test_function(function, &mut function_variables, 0, web_client));
        }
    }
    results
}

pub fn print_in_function(thing: &impl Display, depth: usize) {
    // TODO: Is there a better way to display in a function? Should be using a formatter
    for _ in 0..depth {
        print!(" ");
    }
    println!("{}", thing);
}

pub fn print_function_error(e: &ChimeraRuntimeFailure, depth: usize) {
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
pub fn run_test_function(function: Function, variable_map: &mut HashMap<String, AssignmentValue>, depth: usize, web_client: &reqwest::blocking::Client) -> TestResult {
    print_in_function(&format!("RUNNING TEST {}", function.name), depth);
    let mut context = Context::new();
    // TODO: If the ability to call functions is added (like calling an init function) the teardown stack needs to be
    //       passed as a mut reference into that function so it can add teardown to the stack. Should only be able
    //       to call non-test functions with no parents?
    let mut teardown_stack: Vec<Statement> = Vec::new();
    let mut this_test_passed = true;

    // Get these two variables here as they are needed at the end and the for..in.. is about to consume function
    let is_expected_failure = function.is_expected_failure();
    let function_name = function.name;

    let mut subtest_results: Vec<TestResult> = Vec::new();
    let mut failure_message: Option<ChimeraRuntimeFailure> = None;

    for block_contents in function.block {
        match block_contents {
            BlockContents::Function(nested_function) => subtest_results.push(run_test_function(nested_function, variable_map, depth + 1, web_client)),
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
                                print_function_error(&runtime_error, depth);
                                failure_message = Some(runtime_error);
                                break;
                            },
                            // TODO: Need to still process teardown even here
                            _ => return TestResult::new(function_name, Status::Error(runtime_error), subtest_results)
                        }
                    }
                }
            }
        };
        context.current_line += 1;
    };

    // TODO: When the test function ends, process the teardown stack

    let (status, result_message) = match this_test_passed {
        true => {
            match is_expected_failure {
                true => (Status::UnexpectedSuccess, format!("TEST {} UNEXPECTED SUCCESS", function_name.as_str())),
                false => (Status::Success, format!("TEST {} PASSED", function_name.as_str()))
            }
        },
        false => {
            match is_expected_failure {
                true => (Status::ExpectedFailure, format!("TEST {} EXPECTED FAILURE", function_name.as_str())),
                false => (Status::Failure(failure_message.expect("Failed to get a message for a failed assertion")), format!("TEST {} FAILED", function_name.as_str()))
            }
        }
    };
    print_in_function(&result_message, depth);
    TestResult::new(function_name, status, subtest_results)
}

fn handle_ast_err(e: pest::error::Error<Rule>) -> ChimeraCompileError {
    let line_col = match e.line_col {
        pest::error::LineColLocation::Pos(pos) => pos,
        pest::error::LineColLocation::Span(start, _end) => start
    };
    ChimeraCompileError::new("Failed to parse ChimeraScript", line_col)
}
