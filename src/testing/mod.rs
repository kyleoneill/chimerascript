#[cfg(test)]
mod testing {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use std::sync::{Once, OnceLock};
    use crate::frontend::{Context, run_functions, TestResult};
    use crate::CLIENT;
    use crate::abstract_syntax_tree::{ChimeraScriptAST, HttpCommand, HTTPVerb};
    use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
    use crate::literal::{Collection, Data, DataKind, Literal, NumberKind};
    use crate::util::WebClient;
    use crate::variable_map::VariableMap;

    #[derive(Debug)]
    struct FakeClient {
        domain: String
    }

    impl FakeClient {
        pub fn new(s: &str) -> Self {
            let domain = s.to_owned();
            Self { domain }
        }
    }

    impl WebClient for FakeClient {
        fn get_domain(&self) -> &str {
            self.domain.as_str()
        }
        fn make_request(&self, context: &Context, http_command: HttpCommand, variable_map: &VariableMap) -> Result<DataKind, ChimeraRuntimeFailure> {
            let mut response_obj: HashMap<String, Data> = HashMap::new();
            response_obj.insert("status_code".to_owned(), Data::from_literal(Literal::Number(NumberKind::U64(match http_command.verb {
                HTTPVerb::GET => 200,
                HTTPVerb::DELETE => 200,
                HTTPVerb::POST => 201,
                HTTPVerb::PUT => 200
            }))));

            // Take a request and extract the query and body params from it
            let mut resolved_body: HashMap<String, Data> = HashMap::new();
            for assignment in &http_command.http_assignments {
                let key = assignment.lhs.clone();
                let value = assignment.rhs.resolve(context, variable_map)?;
                resolved_body.insert(key, value);
            }
            let mut query_params: HashMap<String, Data> = HashMap::new();
            for query_param in &http_command.query_params {
                let key = query_param.lhs.clone();
                let value = query_param.rhs.resolve(context, variable_map)?;
                query_params.insert(key, value);
            }

            // Construct a response struct out of the request params
            let body: DataKind = if resolved_body.is_empty() && query_params.is_empty() {
                DataKind::Literal(Literal::Null)
            }
            else {
                let mut body_map: HashMap<String, Data> = HashMap::new();
                body_map.extend(query_params);
                body_map.extend(resolved_body);
                DataKind::Collection(Collection::Object(body_map))
            };
            response_obj.insert("body".to_owned(), Data::new(body));
            Ok(DataKind::Collection(Collection::Object(response_obj)))
        }
    }

    static INIT: Once = Once::new();
    // turn this into a oncelock, the init will set if it hasn't been set yet

    static FAKE_CLIENT: OnceLock<FakeClient> = OnceLock::new();

    fn initialize() {
        // The `INIT: Once` will "lock" this part of the function so its logic can only ever be run once
        // This is needed to do setup that each test needs, running it multiple times causes a panic
        INIT.call_once(|| {
            FAKE_CLIENT.set(FakeClient::new("http://127.0.0.1:5000")).unwrap();
            match CLIENT.set(FAKE_CLIENT.get().unwrap()) {
                Ok(_) => (),
                Err(_) => panic!("Failed to set fake client during test init")
            }
        });
    }

    fn read_cs_file(filename: &str) -> ChimeraScriptAST {
        let full_filename = format!("./src/testing/chs_files/{}", filename);
        let file_contents = fs::read_to_string(Path::new(&full_filename)).expect("Failed to read chs file when setting up test");
        match ChimeraScriptAST::new(file_contents.as_str()) {
            Ok(ast) => ast,
            Err(_) => panic!("Failed to parse a file into an AST")
        }
    }

    fn results_from_filename(filename: &str) -> Vec<TestResult> {
        initialize();
        let ast = read_cs_file(filename);
        run_functions(ast)
    }

    fn assert_test_pass(result: &TestResult, filename: &str, while_doing: &str) {
        assert!(result.passed(), "Test case {} of file {} failed {}", result.test_name(), filename, while_doing);
    }

    fn assert_subtest_length(result: &TestResult, expected_len: usize, filename: &str) {
        assert_eq!(result.subtest_results.len(), expected_len, "Test case {} of file {} should have {} subtest results but had {}", result.test_name(), filename, expected_len, result.subtest_results.len());
    }

    fn assert_test_fail(result: &TestResult, filename: &str, while_doing: &str, should_fail_as: ChimeraRuntimeFailure) {
        assert_eq!(result.passed(), false, "Test case {} of file {} should fail {}", result.test_name(), filename, while_doing);
        match result.error_kind() {
            Some(failure) => {
                assert_eq!(failure, &should_fail_as, "Test case {} of file {} should fail with error {} but got {}", result.test_name(), filename, should_fail_as.get_variant_name(), failure.get_variant_name());
                match failure {
                    ChimeraRuntimeFailure::VarWrongType(_, got_var_type, _) => {
                        match should_fail_as {
                            ChimeraRuntimeFailure::VarWrongType(_, ref should_be_var_type, _) => assert_eq!(got_var_type, should_be_var_type, "Test case {} of file {} should fail with a {} error saying that the expected type should be a {} but it was {}", result.test_name(), filename, should_fail_as.get_variant_name(), should_be_var_type, got_var_type),
                            _ => ()
                        }
                    },
                    _ => ()
                }
            },
            None => panic!("Test case {} of file {} failed but the test result did not contain an error kind", result.test_name(), filename)
        }
    }

    // TODO: Add tests here for a test-case functions, decorators, teardown, nested functions
    // TODO: Add tests for statements being broken up into multiple lines

    // These tests use assert!() to assert that a boolean field is true but assert_eq!() to assert that a boolean
    // field is false, rather than assert!(!...), for ease of reading

    #[test]
    /// Test that a file with invalid ChimeraScript does not compile
    fn invalid_file() {
        let file_contents = fs::read_to_string(Path::new("./src/testing/chs_files/invalid_file.chs")).expect("Failed to read chs file when setting up test");
        match ChimeraScriptAST::new(file_contents.as_str()) {
            Ok(_) => panic!("Trying to parse an invalid ChimeraScript file should result in a compile error"),
            Err(_e) => ()
        }
    }

    #[test]
    /// Test the simplest possible .ch, an assertion that 1 == 1
    fn simple_assertion() {
        initialize();
        let filename = "simplest_test.chs";
        let ast = read_cs_file(filename);
        assert_eq!(ast.functions.len(), 1, "Should only get a single test for a test file which contains one test case but got multiple");
        let res = run_functions(ast);
        assert_eq!(res.len(), 1, "Expected to get a single test result when running a chs file with one test case");
        assert_eq!(res[0].subtest_results.len(), 0, "Test case {} of file {} should have 0 subtests", res[0].test_name(), filename);
        assert_test_pass(&res[0], filename, "when asserting that 1 == 1");
    }

    #[test]
    /// Test that each Literal variant works for assignment and assertion checking
    fn literals() {
        let filename = "literals.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 2, "Expected to get 2 test results when running a chs file with 2 test cases");
        assert_test_pass(&res[0], filename, "when making a basic equality assertion for literal values");
        assert_test_pass(&res[1], filename, "when using literals as variables and running assertions against them");
    }

    #[test]
    /// Test that we can do logical inversion in our tests, both in asserting that something is NOT a value
    /// and that a test can be expected to fail and not have its failure count towards failing
    fn logical_inversion() {
        let filename = "test_negation.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 3, "Expected to get 3 test results when running a chs file with 3 test cases");
        assert_test_pass(&res[0], filename, "when using an expected-failure on a failing assertion");
        assert_test_pass(&res[1], filename, "when using an ASSERT NOT EQUALS");
        assert_test_pass(&res[2], filename, "when using an expected-failure on a passing assertion");
    }

    #[test]
    /// Test that failed assertions result in a test being marked as failing
    fn failing_tests() {
        let filename = "failing_test.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 5, "Expected to get 5 test results when running {} which has 5 outermost test cases", filename);
        assert_eq!(res[0].subtest_results.len(), 0, "Test case {} of file {} should have no subtest_results even though it has a nested test case, as it should have failed before reaching the nested case", res[0].test_name(), filename);
        assert_test_fail(&res[0], filename, "on a bad equality assertion", ChimeraRuntimeFailure::TestFailure("".to_owned(), 0));
        assert_test_fail(&res[1], filename, "on a bad GTE assertion", ChimeraRuntimeFailure::TestFailure("".to_owned(), 0));
        assert_test_fail(&res[2], filename, "on a bad GT assertion", ChimeraRuntimeFailure::TestFailure("".to_owned(), 0));
        assert_test_fail(&res[3], filename, "on a bad LTE assertion", ChimeraRuntimeFailure::TestFailure("".to_owned(), 0));
        assert_test_fail(&res[4], filename, "on a bad LT assertion", ChimeraRuntimeFailure::TestFailure("".to_owned(), 0));
    }

    #[test]
    /// Test that test-cases can be nested
    fn nested_tests() {
        let filename = "nested.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 2, "Expected to get 2 test results when running {} which has two outermost tests which both contain nested tests", filename);

        // First outer test verifies that deeply nested tests pass
        assert_subtest_length(&res[0], 1, filename);
        assert_test_pass(&res[0], filename, "when making a simple assertion and having a nested subtest");
        assert_subtest_length(&res[0].subtest_results[0], 1, filename);
        assert_test_pass(&res[0].subtest_results[0], filename, "when making a simple assertion as a subtest with a subtest of its own");
        assert_subtest_length(&res[0].subtest_results[0].subtest_results[0], 0, filename);
        assert_test_pass(&res[0].subtest_results[0].subtest_results[0], filename, "when making a simple assertion as a deeply nested subtest");

        // Second outer test verifies that a child test failing does not prevent a parent test from passing
        assert_subtest_length(&res[1], 1, filename);
        assert_test_fail(&res[1].subtest_results[0], filename, "when making an assertion that 1==2", ChimeraRuntimeFailure::TestFailure("".to_string(), 0));
        assert_test_pass(&res[1], filename, "when it should pass, even if it has a failing child test");
    }

    #[test]
    /// Test that test-cases can make web requests
    fn web_requests() {
        let filename = "web_request.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 6);

        // Test GET
        assert_test_pass(&res[0], filename, "to confirm basic usage of a GET request");
        assert_subtest_length(&res[0], 1, filename);
        assert_test_pass(&res[0].subtest_results[0], filename, "to confirm that CONTAINS can be used on a web response");

        // Test PUT
        assert_test_pass(&res[1], filename, "to confirm basic usage of a PUT request");
        assert_subtest_length(&res[1], 1, filename);
        assert_test_pass(&res[1].subtest_results[0], filename, "to use a variable in an endpoint path");

        // Test DELETE
        assert_test_pass(&res[2], filename, "to confirm basic usage of a DELETE request");

        // Test POST
        assert_test_pass(&res[3], filename, "to confirm basic usage of a POST request");

        // Test PRINT
        assert_test_pass(&res[4], filename, "to confirm basic usage of PRINT on a request");

        // Test query params
        assert_test_pass(&res[5], filename, "to confirm basic usage of query params in a request");
        assert_subtest_length(&res[5], 1, filename);
        assert_test_pass(&res[5].subtest_results[0], filename, "to confirm usage of variables and strings in request query params");
    }

    #[test]
    /// Test runtime errors
    fn runtime_errors() {
        let filename = "runtime_errors.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 6);

        // Non-existent var
        assert_test_fail(&res[0], filename, "when using a non-existent variable", ChimeraRuntimeFailure::VarNotFound("".to_owned(), 0));

        // Bad subfield access
        assert_test_fail(&res[1], filename, "when making a bad subfield access", ChimeraRuntimeFailure::BadSubfieldAccess(None, "".to_owned(), 0));

        // Wrong type
        assert_test_fail(&res[2], filename, "when using a GT assertion on a non-numeric type", ChimeraRuntimeFailure::VarWrongType("".to_owned(), VarTypes::Number, 0));

        // Index a list with an out-of-bounds value
        assert_test_fail(&res[3], filename, "when accessing a list with an out of bounds value", ChimeraRuntimeFailure::OutOfBounds(0));

        // Index a list with a non-existent subfield and a non number
        assert_test_fail(&res[4], filename, "when accessing a list via a non-existent subfield", ChimeraRuntimeFailure::TriedToIndexWithNonNumber(0));
        assert_test_fail(&res[5], filename, "when accessing a list with a non-numerical index", ChimeraRuntimeFailure::TriedToIndexWithNonNumber(0));
    }

    #[test]
    /// Test that the print command causes no errors
    fn print_command() {
        // This test is not complete, it just checks that the print command causes no failures.
        // Actually testing what it does involves re-directing the project writer from std
        // output. This seems to require a mutable static reference, which then requires
        // an unsafe block both here and in our write method which is not great
        let filename = "print.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 1);
        assert_test_pass(&res[0], filename, "when printing a literal and a variable");
    }

    #[test]
    /// Test that the list command works. This includes making a new list, getting the length of a
    /// list, accessing the list, appending to a list, removing from a list, and printing a list
    fn list_command() {
        let filename = "list.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 6);

        // Test general list functionality
        assert_eq!(res[0].subtest_results.len(), 10);
        assert_test_pass(&res[0], filename, "when making a new list");
        assert_test_pass(&res[0].subtest_results[0], filename, "when getting a list length");
        assert_test_pass(&res[0].subtest_results[1], filename, "when making an empty list");
        assert_test_pass(&res[0].subtest_results[2], filename, "when printing a list");
        assert_test_pass(&res[0].subtest_results[3], filename, "when accessing a list by index");
        assert_test_pass(&res[0].subtest_results[4], filename, "when appending to a list");
        assert_test_pass(&res[0].subtest_results[5], filename, "when removing from a list by index");
        assert_test_pass(&res[0].subtest_results[6], filename, "when using LENGTH assertion on a list");
        assert_test_pass(&res[0].subtest_results[7], filename, "when using CONTAINS assertion on a list");
        assert_test_pass(&res[0].subtest_results[8], filename, "when popping from a list");
        assert_test_pass(&res[0].subtest_results[9], filename, "when checking equality between lists");

        // Remove a value from list out of bounds
        assert_test_fail(&res[1], filename, "when removing a value from a list with an out-of-bounds index", ChimeraRuntimeFailure::OutOfBounds(0));

        // Append to a list that doesn't exist
        assert_test_fail(&res[2], filename, "when appending to a list that does not exist", ChimeraRuntimeFailure::VarNotFound("".to_owned(), 0));

        // ASSERT LENGTH on a non-list
        assert_test_fail(&res[3], filename, "when asserting length on a non-list", ChimeraRuntimeFailure::VarWrongType("".to_owned(), VarTypes::List, 0));

        // ASSERT CONTAINS on a literal value
        assert_test_fail(&res[4], filename, "when asserting CONTAINS on a literal value", ChimeraRuntimeFailure::VarWrongType("".to_owned(), VarTypes::Containable, 0));

        // LIST POP on an empty list
        assert_test_fail(&res[5], filename, "when using POP on an empty list", ChimeraRuntimeFailure::OutOfBounds(0));
    }

    #[test]
    /// Test that different number kinds work in different scenarios. Each of the number kinds should be able to be
    /// made from the LITERAL command, should work in lists, and should be comparable for equality and ordering
    fn number_kinds() {
        let filename = "numberkinds.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 2);
        assert_test_pass(&res[0], filename, "when testing assertions on the different types of numbers");
        assert_test_pass(&res[1], filename, "when using the different kinds of numbers in a list");
    }

    #[test]
    /// Test that comments can be included in test script
    fn comments() {
        let filename = "comments.chs";
        let res = results_from_filename(filename);
        assert_eq!(res.len(), 1);
        assert_test_pass(&res[0], filename, "when running 1==1 assertions while using comments");
    }

    // TODO: Test for get_result_counts. Test something with multiple outer cases, nested tests, passes, errors, and failures
    //       Make sure some nested cases are reached and others are not (they are nested after a failure of parent)

    // TODO: Test for error messages when a test fails
}
