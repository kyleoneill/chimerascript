use pest::error::InputLocation;
use pest::Parser;
use pest_derive::Parser;
use yaml_rust::Yaml;
use crate::err_handle::ChimeraError;
use crate::abstract_syntax_tree::ChimeraScriptAST;

#[derive(Parser, Debug)]
#[grammar = "grammar.pest"]
pub struct CScriptTokenPairs;

/// A TestCase consists of an optional expected_failure, a setup step which will run before the test,
/// a set of steps which make up a test, and a set of teardown steps which run after the test. The
/// setup and teardown steps are a vec of TestLine but the main test, steps, is a vec of Operations
/// as a test can contain a sub-test.
#[derive(Debug)]
pub struct TestCase {
    name: String,
    expected_failure: bool,
    setup: Option<Vec<TestLine>>,
    steps: Vec<Operation>,
    teardown: Option<Vec<TestLine>>
}

impl TestCase {
    fn from_yaml(yaml: Yaml) -> Result<Self, ChimeraError> {
        match yaml {
            Yaml::Hash(mut case) => {
                // the smallest possible test file needs at least 2 Yaml items, ex
                // - case: foo  <-- this is one item
                //   steps:     <-- This is the second item
                //     - ASSERT EQUALS 1 1 <-- This is under the second item
                if case.len() < 2 {
                    return Err(ChimeraError::InvalidChimeraFile("TestCase must have at least one case and its steps.".to_owned()))
                }

                // TODO: The below needs to be refactored, there _has_ to be a cleaner way to do this

                // Get Yaml string versions of our test-case keys
                let name_key = Yaml::from_str("case");
                let expected_key = Yaml::from_str("expected-failure");
                let setup_key = Yaml::from_str("setup");
                let step_key = Yaml::from_str("steps");
                let teardown_key = Yaml::from_str("teardown");

                // Grab our test-case keys from the yaml. The case name and steps are mandatory, error if they are not present
                // expected_failure, setup, and teardown are optional. Default to false and empty arrays if they aren't present
                let name = if case.contains_key(&name_key) {case.get(&name_key).unwrap().as_str().unwrap().to_owned()} else {return Err(ChimeraError::InvalidChimeraFile("TestCase must have a 'case' key which contains its name.".to_owned()))};
                let expected_failure_yaml = if case.contains_key(&expected_key) {case.get(&expected_key).unwrap().as_bool()} else {None};
                let expected_failure = if expected_failure_yaml.is_some() {expected_failure_yaml.unwrap()} else {false};
                let setup_yaml = if case.contains_key(&setup_key) {case.remove(&setup_key).unwrap()} else {Yaml::Array(vec![])};
                let steps_yaml = if case.contains_key(&step_key) {case.remove(&step_key).unwrap()} else {return Err(ChimeraError::InvalidChimeraFile("TestCase must have a 'steps' key which contains its steps.".to_owned()))};
                let teardown_yaml = if case.contains_key(&teardown_key) {case.remove(&teardown_key).unwrap()} else {Yaml::Array(vec![])};

                // Convert setup and teardown from Yaml::Array into Vec<TestLine>
                // Convert steps from Yaml::Array into Vec<Operation>
                // setup and teardown do not support sub-testing, so they can only contain test lines and no further tests
                let setup_vec = TestLine::vec_from_yaml_array(setup_yaml)?;
                let steps = Operation::vec_from_yaml_array(steps_yaml)?;
                let teardown_vec = TestLine::vec_from_yaml_array(teardown_yaml)?;

                Ok(TestCase {
                    name,
                    expected_failure,
                    setup: if setup_vec.len() > 0 {Some(setup_vec)} else {None},
                    steps,
                    teardown: if teardown_vec.len() > 0 {Some(teardown_vec)} else {None}
                })
            }
            _ => {
                Err(ChimeraError::InvalidChimeraFile("A yaml TestCase must begin with a Yaml::Hash variant.".to_owned()))
            }
        }
    }

    /// Runs a test case
    pub fn run_test_case(self) -> bool {
        println!("{:#?}", self);
        // Will need a hashmap in this method to track variables to be accessed in setup, steps, and teardown
        todo!()
    }
}

/// An Operation is an instruction within a test, it can be either a TestLine or a nested TestCase.
#[derive(Debug)]
enum Operation {
    Test {test_case: TestCase},
    Line {test_line: TestLine}
}

impl Operation {
    /// Convert a Yaml::Array into a Vec<Operation>
    pub fn vec_from_yaml_array(input: Yaml) -> Result<Vec<Self>, ChimeraError> {
        match input {
            Yaml::Array(yaml_arr) => {
                let mut res: Vec<Self> = Vec::new();
                // Iterate through the Yaml::Array elements
                for yaml in yaml_arr.to_vec() {
                    match yaml {
                        // If the element is a Yaml::Hash then it's a nested test-case
                        Yaml::Hash(nested_test_case) => {
                            let test_case = TestCase::from_yaml(Yaml::Hash(nested_test_case))?;
                            res.push(Operation::Test {test_case});
                        }
                        // If the element is a Yaml::String then it's a test-case line
                        Yaml::String(yaml_line) => {
                            let stringified_line = yaml_line.as_str();
                            let parsed = CScriptTokenPairs::parse(Rule::Statement, stringified_line);
                            match parsed {
                                Ok(parsed_line) => {
                                    let ast = ChimeraScriptAST::from_pairs(parsed_line)?;
                                    let test_line = TestLine { line: ast };
                                    res.push(Operation::Line { test_line });
                                }
                                Err(e) => {
                                    return Err(handle_ast_err(e));
                                }
                            }
                        }
                        _ => return Err(ChimeraError::InvalidChimeraFile("A test case line must contain either a string to parse into ChimeraScript or a nested TestCase, but got neither.".to_owned()))
                    }
                }
                Ok(res)
            }
            _ => return Err(ChimeraError::InvalidChimeraFile("Cannot convert a Yaml to a vec unless it's a Yaml::Array variant.".to_owned()))
        }
    }
}

/// A TestLine is a line of ChimeraScript
#[derive(Debug)]
struct TestLine {
    line: ChimeraScriptAST
}

impl TestLine {
    /// Convert a Yaml::Array into a Vec<TestLine>
    pub fn vec_from_yaml_array(input: Yaml) -> Result<Vec<Self>, ChimeraError> {
        // TODO: There is a lot of overlap between this and the Operation method of the same name,
        //       I should make this DRY
        match input {
            Yaml::Array(yaml_arr) => {
                let mut res: Vec<Self> = Vec::new();
                // Iterate through the Yaml::Array elements
                for yaml in yaml_arr.to_vec() {
                    match yaml {
                        // If the element is a Yaml::Hash then it's a nested test-case
                        Yaml::Hash(_nested_test_case) => {
                            return Err(ChimeraError::InvalidChimeraFile("Setup and teardown sections cannot contain a nested sub-case.".to_owned()));
                        }
                        // If the element is a Yaml::String then it's a test-case line
                        Yaml::String(yaml_line) => {
                            let stringified_line = yaml_line.as_str();
                            let parsed = CScriptTokenPairs::parse(Rule::Statement, stringified_line);
                            match parsed {
                                Ok(parsed_line) => {
                                    let ast = ChimeraScriptAST::from_pairs(parsed_line)?;
                                    let test_line = TestLine { line: ast };
                                    res.push(test_line);
                                }
                                Err(e) => {
                                    return Err(handle_ast_err(e))
                                }
                            }
                        }
                        _ => return Err(ChimeraError::InvalidChimeraFile("A test case setup or teardown line can only contain a string to parse into ChimeraScript, but got something else.".to_owned()))
                    }
                }
                Ok(res)
            }
            _ => return Err(ChimeraError::InvalidChimeraFile("Cannot convert a Yaml to a vec unless it's a Yaml::Array variant.".to_owned()))
        }
    }
}

fn handle_ast_err(e: pest::error::Error<Rule>) -> ChimeraError {
    let position = match e.location {
        InputLocation::Pos(pos) => pos,
        InputLocation::Span((start, _end)) => start
    };
    ChimeraError::FailedParseAST(format!("Failed to parse ChimeraScript at position {} of line: {}", position, e.line()).to_owned())
    // match e.variant {
    //     pest::error::ErrorVariant::ParsingError {
    //         positives,
    //         negatives
    //     } => {
    //         ChimeraError::FailedParseAST("".to_owned())
    //     }
    //     _ => ChimeraError::FailedParseAST("UNHANDLED CUSTOM ERR MSG".to_owned())
    // }
}

pub fn iterate_yaml(yaml_doc: Yaml) -> Result<Vec<TestCase>, ChimeraError> {
    match yaml_doc {
        Yaml::Array(yaml_arr) => {
            let mut ret: Vec<TestCase> = Vec::new();
            for yaml in yaml_arr {
                let test_case = TestCase::from_yaml(yaml)?;
                ret.push(test_case);
            }
            Ok(ret)
        }
        _ => {
            Err(ChimeraError::InvalidChimeraFile("chs file should begin with a list of test cases but it did not.".to_owned()))
        }
    }
}
