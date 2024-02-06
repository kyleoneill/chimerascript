use std::fmt::{Display, Formatter};
use std::iter::Sum;
use pest::iterators::Pairs;
use pest::Parser;
use pest_derive::Parser;
use crate::abstract_syntax_tree::{ChimeraScriptAST, Statement, Function, BlockContents};
use crate::err_handle::{ChimeraCompileError, ChimeraRuntimeFailure};
use crate::util::Timer;
use crate::variable_map::VariableMap;

pub struct Context {
    pub current_line: i32
}

impl Context {
    pub fn new() -> Self {
        Self { current_line: 0 }
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct ResultCount {
    success: usize,
    failure: usize,
    error: usize,
    total_tests: usize
}

impl Display for ResultCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ran {} tests with {} successes, {} failures, and {} errors\n\n{}", self.total_tests, self.success, self.failure, self.error, self.overall_result())
    }
}

impl ResultCount {
    fn new_empty() -> Self {
        Self { success: 0, failure: 0, error: 0, total_tests: 0 }
    }
    pub fn new(input: (usize, usize, usize, usize)) -> Self {
        Self { success: input.0, failure: input.1, error: input.2, total_tests: input.3 }
    }
    pub fn overall_result(&self) -> &str {
        if self.failure == 0 && self.error == 0 {"PASSED"} else {"FAILED"}
    }
    fn print_with_time(&self, time_taken: &str) {
        println!("Ran {} tests in {} with {} successes, {} failures, and {} errors\n\n{}", self.total_tests, time_taken, self.success, self.failure, self.error, self.overall_result())
    }
    pub fn print_test_result(results: Vec<TestResult>, maybe_time_taken: Option<&str>) {
        // Not really a fan of how printing with or without time is being handled here, and how the Display impl is
        // being mostly ignored. Should be a way to refactor this
        let result_count: ResultCount = results.iter().map(|x| x.get_result_counts()).sum();
        match maybe_time_taken {
            Some(time_taken) => result_count.print_with_time(time_taken),
            None => println!("{}", result_count)
        }
    }
}

impl std::ops::Add for ResultCount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            success: self.success + rhs.success,
            failure: self.failure + rhs.failure,
            error: self.error + rhs.error,
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
#[allow(dead_code)] // "dead" field used by tests
#[derive(Debug)]
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
pub fn run_functions(ast: ChimeraScriptAST) -> Vec<TestResult> {
    let mut results: Vec<TestResult> = Vec::new();
    for function in ast.functions {
        if function.is_test_function() {
            let mut function_variables = VariableMap::new();
            results.push(run_test_function(function, &mut function_variables, 0));
        }
    }
    results
}

pub fn print_in_function(thing: &impl Display, depth: usize) {
    // This formats an empty string to be padded rightwards by `depth`
    // Cannot directly add padding to `thing` because padding is conditionally added to things shorter than the
    // padding amount, so an empty string is used instead to act as padding
    println!("{:indent$}{}", "", thing, indent=depth);
}

// TODO: Should variable scoping be added? How will this impact the teardown stack (if teardown is added by called non-
//       test functions)?
pub fn run_test_function(function: Function, variable_map: &mut VariableMap, depth: usize) -> TestResult {
    print_in_function(&format!("STARTING TEST - {}", function.name), depth);
    let timer = Timer::new();
    let mut context = Context::new();
    // TODO: If the ability to call functions is added (like calling an init function) the teardown stack needs to be
    //       passed as a mut reference into that function so it can add teardown to the stack. Should only be able
    //       to call non-test functions with no parents?
    let mut teardown_stack: Vec<Statement> = Vec::new();

    // Get these two variables here as they are needed at the end and the for..in.. is about to consume function
    let is_expected_failure = function.is_expected_failure();
    let function_name = function.name;

    let mut subtest_results: Vec<TestResult> = Vec::new();
    let mut runtime_failure: Option<ChimeraRuntimeFailure> = None;

    for block_contents in function.block {
        match block_contents {
            BlockContents::Function(nested_function) => subtest_results.push(run_test_function(nested_function, variable_map, depth + 1)),
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
                        crate::commands::assert::assert_command(&context, &assert_command, variable_map)
                    },
                    Statement::AssignmentExpr(assert_expr) => {
                        crate::commands::assignment::assignment_command(&context, assert_expr, variable_map)
                    },
                    Statement::PrintCommand(print_cmd) => {
                        crate::commands::print::print_command(&context, print_cmd, variable_map, depth)
                    },
                    Statement::Expression(expr) => {
                        // We are running an expression without assigning it, we can toss the result
                        match crate::commands::expression::expression_command(&context, expr, variable_map) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e)
                        }
                    }
                };
                match statement_result {
                    Ok(_) => (),
                    Err(runtime_error) => {
                        runtime_error.print_error(depth);
                        runtime_failure = Some(runtime_error);
                        break;
                    }
                }
            }
        };
        context.current_line += 1;
    };

    // TODO: When the test function ends, process the teardown stack

    let status = match runtime_failure {
        Some(failure_reason) => {
            match failure_reason {
                ChimeraRuntimeFailure::TestFailure(_, _) => match is_expected_failure {
                    true => Status::ExpectedFailure,
                    false => Status::Failure(failure_reason)
                },
                _ => Status::Error(failure_reason)
            }
        },
        None => {
            match is_expected_failure {
                true => Status::UnexpectedSuccess,
                false => Status::Success
            }
        }
    };
    let time_to_run = timer.finish();
    print_in_function(&format!("FINISHED TEST - {} - {} - {}", function_name.as_str(), time_to_run, &status), depth);
    TestResult::new(function_name, status, subtest_results)
}

fn handle_ast_err(e: pest::error::Error<Rule>) -> ChimeraCompileError {
    let line_col = match e.line_col {
        pest::error::LineColLocation::Pos(pos) => pos,
        pest::error::LineColLocation::Span(start, _end) => start
    };
    ChimeraCompileError::new("Invalid ChimeraScript", line_col)
}
