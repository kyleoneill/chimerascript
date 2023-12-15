#[cfg(test)]
mod testing {
    use std::fs;
    use std::path::Path;
    use std::sync::{OnceLock, Once};
    use reqwest::blocking::Client;
    use yaml_rust::YamlLoader;
    use crate::err_handle::ChimeraCompileError;
    use crate::frontend;
    use crate::frontend::TestCase;
    use crate::WEB_REQUEST_DOMAIN;

    static INIT: Once = Once::new();

    static WEB_CLIENT: OnceLock<Client> = OnceLock::new();

    // TODO: Tests currently rely on there being a local server running which requests are made
    //       against. Would be a lot better to have some sort of test harness which "captured"
    //       the requests being made and providing the expected response, so the use of a server
    //       is avoided. Maybe using a mock Client for tests?

    fn initialize() -> &'static Client {
        // The INIT: Once will "lock" this function so its logic can only ever be run once
        // This is needed to do setup that each test needs, running it multiple times causes a panic
        INIT.call_once(|| {
            WEB_REQUEST_DOMAIN.set("http://127.0.0.1:5000".to_owned()).unwrap();
            WEB_CLIENT.set(Client::new()).unwrap();
        });
        WEB_CLIENT.get().expect("Failed to get Client")
    }

    fn read_cs_file(filename: &str) -> Vec<TestCase> {
        let full_filename = format!("./src/testing/chs_files/{}", filename);
        let file_contents = fs::read_to_string(Path::new(&full_filename)).expect("Failed to read chs file when setting up test");
        let mut test_file_yaml = YamlLoader::load_from_str(file_contents.as_str()).expect("Failed to parse chs file when setting up test");
        frontend::iterate_yaml(test_file_yaml.pop().expect("Failed to pass a vec of yaml to frontend::iterate_yaml")).expect("Failed to convert yaml to a vec of TestCase")
    }

    #[test]
    /// Assert that a test file which begins with a test line and not a test case will error
    fn bad_yaml() {
        let file_contents = fs::read_to_string(Path::new("./src/testing/chs_files/bad_yaml.chs")).expect("Failed to read chs file when setting up test");
        let mut test_file_yaml = YamlLoader::load_from_str(file_contents.as_str()).expect("Failed to parse chs file when setting up test");
        match frontend::iterate_yaml(test_file_yaml.pop().unwrap()) {
            Ok(_) => panic!("A .chs file which does not contain an outermost test case should error during iteration but it did not"),
            Err(kind) => {
                match kind {
                    ChimeraCompileError::InvalidChimeraFile(_) => (),
                    _ => panic!("A .chs file which does not contain an outermost test case should return an InvalidChimeraFile error but it returned a different one")
                }
            }
        }
    }

    #[test]
    /// Test the simplest possible .ch, an assertion that 1 == 1
    fn simple_assertion() {
        let client = initialize();
        let filename = "simplest_test.chs";
        let tests = read_cs_file(filename);
        assert_eq!(tests.len(), 1, "Should only get a single test for a test file which contains one test case but got multiple");
        let res = TestCase::run_outermost_test_case(tests, client);
        assert_eq!(res.0, 1, "{} failed, which asserted that 1 equals 1", filename)
    }

    #[test]
    /// Test that each Literal variant works for assignment and assertion checking
    fn literals() {
        let client = initialize();
        let filename = "literals.chs";
        let tests = read_cs_file(filename);
        let res = TestCase::run_outermost_test_case(tests, client);
        assert_eq!(res.0, 2, "{} failed, which tests that assertions can be made using literal values both directly and as variables", filename)
    }

    #[test]
    /// Test that we can do logical inversion in our tests, both in asserting that something is NOT a value
    /// and that a test can be expected to fail and not have its failure count towards failing
    fn logical_inversion() {
        let client = initialize();
        let filename = "test_negation.chs";
        let tests = read_cs_file(filename);
        let res = TestCase::run_outermost_test_case(tests, client);
        assert_eq!(res.0, 3, "{} failed, which tests that assertions can be negated with NOT and tests can be marked as expected failures", filename)
    }

    #[test]
    /// Test that failed assertions result in a test being marked as failing
    fn failing_tests() {
        let client = initialize();
        let filename = "failing_test.chs";
        let tests = read_cs_file(filename);
        let res = TestCase::run_outermost_test_case(tests, client);
        assert_eq!(res.0, 0, "{} should have zero passing tests but had {}", filename, res.0);
        assert_eq!(res.1, 5, "{} should have 5 failing tests but had {}", filename, res.1)
    }

    #[test]
    /// Test that test-cases can be nested
    fn nested_tests() {
        let client = initialize();
        let filename = "nested.chs";
        let tests = read_cs_file(filename);
        let res = TestCase::run_outermost_test_case(tests, client);
        assert_eq!(res.0, 3, "{} should have 3 passing nested tests but had {}", filename, res.0);
    }

    #[test]
    /// Test that test-cases can make web requests
    fn web_requests() {
        let client = initialize();
        let filename = "web_request.chs";
        let tests = read_cs_file(filename);
        let res = TestCase::run_outermost_test_case(tests, client);
        assert_eq!(res.0, 4, "{} should have 4 passing web request tests but had {}", filename, res.0);
    }

    // TODO: Test printing. Print might need to be given a writer (like write!()) so we can
    //       substitute in where it's writing to in its test so we can assert on what it writes
}