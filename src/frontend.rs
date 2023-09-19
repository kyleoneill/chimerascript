use yaml_rust::{Yaml, yaml};
use crate::err_handle::{ChimeraError, print_error};

/// A TestCase consists of an optional expected_failure, a setup step which will run before the test,
/// a set of steps which make up a test, and a set of teardown steps which run after the test. The
/// setup and teardown steps are a vec of TestLine but the main test, steps, is a vec of Operations
/// as a test can contain a sub-test.
#[derive(Debug)]
struct TestCase {
    name: String,
    expected_failure: bool,
    setup: Option<Vec<TestLine>>,
    steps: Vec<Operation>,
    teardown: Option<Vec<TestLine>>
}

impl TestCase {
    pub fn from_yaml(yaml: Yaml) -> Result<Self, ChimeraError> {
        match yaml {
            Yaml::Hash(mut case) => {
                // the smallest possible test file needs at least 2 Yaml items, ex
                // - case: foo  <-- this is one item
                //   steps:     <-- This is the second item
                //     - ASSERT EQUALS 1 1 <-- This is under the second item
                if case.len() < 2 {
                    return Err(ChimeraError::InvalidChimeraFile)
                }

                // TODO: The below needs to be refactored, there _has_ to be a cleaner way to do this

                let name_key = Yaml::from_str("case");
                let expected_key = Yaml::from_str("expected-failure");
                let setup_key = Yaml::from_str("setup");
                let step_key = Yaml::from_str("steps");
                let teardown_key = Yaml::from_str("teardown");

                let name = if case.contains_key(&name_key) {case.get(&name_key).unwrap().as_str().unwrap().to_owned()} else {return Err(ChimeraError::ChimeraFileNoName)};
                let expected_failure_yaml = if case.contains_key(&expected_key) {case.get(&expected_key).unwrap().as_bool()} else {None};
                let expected_failure = if expected_failure_yaml.is_some() {expected_failure_yaml.unwrap()} else {false};
                let setup_yaml = if case.contains_key(&setup_key) {case.remove(&setup_key).unwrap()} else {Yaml::Array(vec![])};
                let steps_yaml = if case.contains_key(&step_key) {case.remove(&step_key).unwrap()} else {return Err(ChimeraError::ChimeraFileNoSteps)};
                let teardown_yaml = if case.contains_key(&teardown_key) {case.remove(&teardown_key).unwrap()} else {Yaml::Array(vec![])};

                // Convert setup and teardown from Yaml::Array into Vec<TestLine>
                // Convert steps from Yaml::Array into Vec<Operation>
                // setup and teardown do not support sub-testing, so they can only contain test lines and no further tests
                let setup = Operation::vec_to_line(Operation::operation_vec_from_yaml(setup_yaml)?)?;
                let steps = Operation::operation_vec_from_yaml(steps_yaml)?;
                let teardown = Operation::vec_to_line(Operation::operation_vec_from_yaml(teardown_yaml)?)?;

                Ok(TestCase {
                    name,
                    expected_failure,
                    setup,
                    steps,
                    teardown
                })
            }
            _ => {
                Err(ChimeraError::InvalidChimeraFile)
            }
        }
    }
}

/// An Operation is an instruction within a test, it can be either a TestLine or a nested TestCase.
#[derive(Debug)]
enum Operation {
    Test {test_case: TestCase},
    Line {test_line: TestLine}
}

impl Operation {
    pub fn operation_vec_from_yaml(input: Yaml) -> Result<Vec<Self>, ChimeraError> {
        match input {
            Yaml::Array(yaml_arr) => {
                let mut res: Vec<Self> = Vec::new();
                for yaml in yaml_arr.to_vec() {
                    match yaml {
                        Yaml::Hash(nested_test_case) => {
                            let test_case = TestCase::from_yaml(Yaml::Hash(nested_test_case))?;
                            res.push(Operation::Test {test_case});
                        }
                        Yaml::String(yaml_line) => {
                            let test_line = TestLine {line: yaml_line.as_str().to_owned()};
                            res.push(Operation::Line {test_line})
                        }
                        _ => return Err(ChimeraError::InvalidChimeraFile)
                    }
                }
                Ok(res)
            }
            _ => return Err(ChimeraError::InvalidChimeraFile)
        }
    }

    pub fn vec_to_line(input: Vec<Self>) -> Result<Option<Vec<TestLine>>, ChimeraError> {
        if input.len() == 0 {
            return Ok(None);
        }
        let mut res: Vec<TestLine> = Vec::new();
        for item in input.into_iter() {
            match item {
                Operation::Test {test_case} => {
                    return Err(ChimeraError::SubtestInSetupOrTeardown);
                }
                Operation::Line{test_line} => {
                    res.push(test_line);
                }
            }
        }
        Ok(Some(res))
    }
}

/// A TestLine is a line of ChimeraScript
#[derive(Debug)]
struct TestLine {
    line: String
}

pub fn iterate_yaml(yaml_doc: Yaml) -> Result<(i32, i32), ChimeraError> {
    // TODO: See comment in main.rs, iterate_yaml should _NOT_ be running the test, this file
    //       should just be parsing a test struct out of the yaml
    match yaml_doc {
        Yaml::Array(yaml_arr) => {
            let mut passed_tests = 0;
            let mut failed_tests = 0;
            // yaml_arr here should be a list of TestCases
            for yaml in yaml_arr {
                // Try to parse `yaml` into a TestCase
                let test_case = TestCase::from_yaml(yaml)?;
                println!("{:?}\n", test_case);

                // After we've parsed the tests, run them
            }
            Ok((passed_tests, failed_tests))
        }
        _ => {
            print_error("chs YML file should begin with a list of test cases but it did not.");
            Err(ChimeraError::InvalidChimeraFile)
        }
    }
}
