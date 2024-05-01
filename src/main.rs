mod abstract_syntax_tree;
mod commands;
mod err_handle;
mod frontend;
mod literal;
mod testing;
mod util;
mod variable_map;

use err_handle::print_error;
use frontend::ResultCount;
use util::client::{RealClient, WebClient};
use util::config::Config;
use util::timer::Timer;

extern crate reqwest;
extern crate serde;
extern crate serde_json;
use crate::abstract_syntax_tree::ChimeraScriptAST;
use crate::err_handle::CLIError;
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;

const FILE_EXTENSION: &str = "chs";

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Filepath for the ChimeraScript tests
    #[arg()]
    path: String,
    /// Name of a specific test case to run
    #[arg(short, long)]
    name: Option<String>,
    /// Path to a config file
    #[arg(short, long)]
    config: String,
}

static CLIENT: OnceLock<&(dyn WebClient + Sync)> = OnceLock::new();
static REAL_CLIENT: OnceLock<RealClient> = OnceLock::new();

static TEST_NAME: OnceLock<Option<String>> = OnceLock::new();

fn system_checks() {
    if !cfg!(target_pointer_width = "64") {
        panic!("This application must be run on a 64 bit platform.");
    }
}

fn walk_directory(path: &Path) -> Result<ResultCount, CLIError> {
    let mut result_count = ResultCount::new((0, 0, 0, 0));
    if !path.is_dir() {
        return Err(CLIError::new(
            format!(
                "Expected {} to be a directory, but it was not",
                path.display()
            ),
            false,
        ));
    }

    // Here flatten() is flattening read_dir()'s iterator, removing the non-Ok values
    for entry in path.read_dir().expect("Failed to read directory contents").flatten() {
        let entry_path = entry.path();
        if entry_path.is_file() {
            // Check if the file has a .chs extension, ignore it if it doesn't
            if entry_path.extension().is_none()
                || entry_path.extension().unwrap() != FILE_EXTENSION
            {
                continue;
            }
            // Run a test file and get its results
            let count = run_test_file(entry_path.as_path())?;
            result_count = result_count + count;
        } else if entry_path.is_dir() {
            let count = walk_directory(entry_path.as_path())?;
            result_count = result_count + count;
        }
    }
    Ok(result_count)
}

fn run_test_file(path: &Path) -> Result<ResultCount, CLIError> {
    // The file should have a .chs extension if we want to run it
    let extension = path.extension();
    if extension.is_none() || extension.unwrap() != FILE_EXTENSION {
        return Err(CLIError::new(
            format!(
                "{} has an invalid extension, expected it to be '.chs'",
                path.display()
            ),
            false,
        ));
    }

    let file_contents = match fs::read_to_string(path) {
        Ok(res) => res,
        Err(_) => {
            return Err(CLIError::new(
                format!("Failed to read file {}", path.display()),
                false,
            ))
        }
    };
    match ChimeraScriptAST::new(file_contents.as_str()) {
        Ok(ast) => {
            let test_name = TEST_NAME.get().expect("TEST_NAME OnceLock was not set");
            // TODO: What happens if multiple files have the same name?
            //       Could pass the entire directory name here rather than a file name
            //       Could instead indent on the folder name as we recurse into a folder?
            let results = if test_name.is_some() {
                let test_name = test_name.clone().unwrap();
                frontend::run_function_by_name(
                    ast,
                    path.file_name()
                        .expect("Failed to get file name when running functions"),
                    test_name.as_str(),
                )
            } else {
                frontend::run_functions(
                    ast,
                    path.file_name()
                        .expect("Failed to get file name when running functions"),
                )
            };
            Ok(ResultCount::from_test_results(results))
        }
        Err(e) => {
            e.print_error();
            // This is a hack, see the comment in main() and the to-do in err_handle.rs
            Err(CLIError::new("".to_string(), true))
        }
    }
}

fn main() {
    system_checks();
    let args = Args::parse();

    // Get config from args
    let config = match Config::from_path_str(&args.config) {
        Ok(config) => config,
        Err(err_msg) => {
            print_error(&err_msg);
            return;
        }
    };

    // Set the domain for our web requests. The value held by the OnceLock must have a static
    // lifetime, so the client must be placed into its own OnceLock. A little hacky, but functional.
    // The purpose of CLIENT is so the web client can be mocked by tests
    // TODO: make a client builder here, configure it, then build the client
    let client = RealClient::new(
        config.get_target_address(),
        reqwest::blocking::Client::new(),
    );
    REAL_CLIENT
        .set(client)
        .expect("Failed to set up web client");
    match CLIENT.set(REAL_CLIENT.get().unwrap()) {
        Ok(_) => (),
        Err(_) => panic!("Failed to set up web client"),
    }

    // Set TEST_NAME OnceLock which will check if we are running a test by name
    TEST_NAME
        .set(args.name)
        .expect("Failed to set TEST_NAME OnceLock");

    // Get path from args
    let path = PathBuf::from_str(args.path.as_str())
        .expect("infallible method failed when creating PathBuf from str");
    let path_slice = path.as_path();
    if !path.exists() {
        print_error(&format!("{} is not a valid path", path_slice.display()));
        return;
    }

    let timer = Timer::new();

    // Check if we were given a directory or a file
    let results = if path_slice.is_dir() {
        walk_directory(path_slice)
    } else if path_slice.is_file() {
        run_test_file(path_slice)
    } else {
        Err(CLIError::new(
            "The given path was not a directory or file, when it must be one of those two"
                .to_string(),
            false,
        ))
    };

    match results {
        Ok(res) => {
            let run_time = timer.finish();
            res.print_with_time(run_time.as_str());
        }
        Err(e) => {
            // This is a hack to reconcile reporting both the CLI errors in this file and the
            // compilation errors from ChimeraScriptAST::new
            // If we reported a compile time error, then we don't want to report a CLI error
            if !e.already_reported {
                e.print_error()
            }
        }
    }
}

#[cfg(test)]
mod main_tests {
    use crate::{run_test_file, walk_directory, TEST_NAME};
    use std::path::Path;

    fn initialize() {
        if TEST_NAME.get().is_none() {
            TEST_NAME.set(None).unwrap()
        }
    }

    #[test]
    fn test_run_test_file() {
        initialize();

        // Verify we get an error when running a test file with a file that doesn't exist
        let bad_path = Path::new("./src/testing/chs_files/idontexist.chs");
        let res = run_test_file(bad_path);
        assert!(
            res.is_err(),
            "Trying to run a file that does not exist should error"
        );

        // Verify we get an error when running a non chs file
        let wrong_type_path = Path::new("./src/testing/chs_files/directory_tests/skipme.json");
        let res = run_test_file(wrong_type_path);
        assert!(
            res.is_err(),
            "Trying to run a file that does not have a .chs extension should error"
        );

        // Verify we get an error when running a test file with a directory
        let wrong_type_path = Path::new("./src/testing/chs_files/directory_tests/");
        let res = run_test_file(wrong_type_path);
        assert!(
            res.is_err(),
            "Trying to run a test file but passing a directory should error"
        );

        // Run a simple test
        let literal_test_path = Path::new("./src/testing/chs_files/simplest_test.chs");
        let res = run_test_file(literal_test_path);
        assert!(res.is_ok(), "Running a simple chs test should Ok");
    }

    #[test]
    fn test_walk_directory() {
        initialize();

        // Try to walk directory on a file, which should fail
        let literal_test_path = Path::new("./src/testing/chs_files/simplest_test.chs");
        let res = walk_directory(literal_test_path);
        assert!(res.is_err(), "Passing walk_directory a file should Err");

        // Walk a directory, should only run .chs files
        let path = Path::new("./src/testing/chs_files/directory_tests/");
        let res = walk_directory(path);
        assert!(res.is_ok(), "Passing walk_directory a directory should Ok");
        let result_count = res.unwrap();
        assert_eq!(result_count.success_count(), 3);
    }
}
