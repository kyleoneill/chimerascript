#[cfg(test)]
mod testing {
    use std::fs;
    use std::path::Path;
    use std::sync::{OnceLock, Once};
    use reqwest::blocking::Client;
    use yaml_rust::{Yaml, YamlLoader};
    use crate::err_handle::ChimeraCompileError;
    use crate::frontend;
    use crate::frontend::TestCase;

    static INIT: Once = Once::new();

    static WEB_REQUEST_DOMAIN: OnceLock<String> = OnceLock::new();

    static WEB_CLIENT: OnceLock<Client> = OnceLock::new();

    // TODO: Tests currently rely on there being a local server running which requests are made
    //       against. Would be a lot better to have some sort of test harness which "captured"
    //       the requests being made and providing the expected response, so the use of a server
    //       is avoided. Maybe using a mock Client for tests?

    fn initialize() {
        // The INIT: Once will "lock" this function so its logic can only ever be run once
        // This is needed to do setup that each test needs, running it multiple times causes a panic
        INIT.call_once(|| {
            WEB_REQUEST_DOMAIN.set("http://127.0.0.1:5000".to_owned()).unwrap();
            WEB_CLIENT.set(Client::new()).unwrap();
        });
    }

    fn read_cs_file(filename: &str) -> Vec<Yaml> {
        let full_filename = format!("./src/testing/{}", filename);
        let file_contents = fs::read_to_string(Path::new(&full_filename)).expect("Failed to read chs file when setting up test");
        YamlLoader::load_from_str(file_contents.as_str()).expect("Failed to parse chs file when setting up test")
    }

    #[test]
    /// Assert that a test file which begins with a test line and not a test case will error
    fn invalid_file() {
        initialize();
        let mut test_file_yaml = read_cs_file("invalid.chs");
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
        initialize();
        let mut test_file_yaml = read_cs_file("simplest_test.chs");
        let tests = frontend::iterate_yaml(test_file_yaml.pop().expect("Failed to pass a vec of yaml to iterate_yaml")).expect("Failed to convert yaml to a vec of TestCase");
        assert_eq!(tests.len(), 1, "Should only get a single test for a test file which contains one test case but got multiple");
        let client = WEB_CLIENT.get().expect("Failed to get Client");
        let res = TestCase::run_outermost_test_case(tests, client);
        assert_eq!(res.0, 1, "Failed to run a test case which asserted that 1 equals 1")
    }
}