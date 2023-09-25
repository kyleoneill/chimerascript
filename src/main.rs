mod err_handle;
mod frontend;
mod abstract_syntax_tree;

use err_handle::print_error;

extern crate yaml_rust;
use std::fs;
use std::path::Path;
use clap::Parser;
use yaml_rust::YamlLoader;
use crate::err_handle::ChimeraError;

const FILE_EXTENSION: &'static str = "chs";

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Path of the file to read
    // long means `path` can only be used as `--path <value>`
    #[arg(long)]
    path: String,
    /// Name of a specific test case to run
    // short means `name` can only be used as `-n <value>`
    #[arg(short)]
    name: Option<String>
}

fn main() {
    let args = Args::parse();
    let path = Path::new(&args.path);
    if !path.exists() {
        print_error(&format!("{} is not a valid path", &args.path));
        return;
    }

    // TODO: Handle being given a directory, iterate through files within it

    // TODO: When iterating through a directory, this should just pass on the iteration rather than err
    //       But it SHOULD error when being passed a single file
    let extension = path.extension();
    if extension.is_none() || extension.unwrap() != FILE_EXTENSION {
        print_error(&format!("{} has an invalid extension, expected it to be '.chs'", &args.path));
        return
    }

    let file_contents = fs::read_to_string(&args.path);
    if file_contents.is_err() {
        print_error(&format!("Failed to read file {}", &args.path));
        return;
    }
    let file_contents = file_contents.unwrap();
    let test_file = YamlLoader::load_from_str(&file_contents);
    match test_file {
        Ok(mut file_yaml) => {
            match frontend::iterate_yaml(file_yaml.remove(0)) {
                Ok(tests) => {
                    let mut tests_passed = 0;
                    let mut tests_failed = 0;
                    for test in tests {
                        match test.run_test_case() {
                            true => tests_passed += 1,
                            false => tests_failed += 1
                        }
                    }
                    let overall_result = if tests_failed == 0 {"PASSED"} else {"FAILED"};
                    println!("TEST {} WITH {} SUCCESSES AND {} FAILURES", overall_result, tests_passed, tests_failed);
                }
                Err(err) => {
                    match err {
                        ChimeraError::InvalidChimeraFile(msg) => print_error(&msg),
                        ChimeraError::FailedParseAST(msg) => print_error(&format!("Failed to parse tokens into AST, {}", &msg))
                    }
                }
            }
        },
        Err(e) => {
            let marker = e.marker();
            print_error(&format!("Failed to parse {} at line {} col {} with error '{}'", &args.path, marker.line(), marker.col(), e.to_string()));
        }
    }
}
