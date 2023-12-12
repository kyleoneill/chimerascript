mod err_handle;
mod frontend;
mod abstract_syntax_tree;
mod commands;
mod util;

use std::collections::HashMap;
use err_handle::print_error;

extern crate reqwest;
extern crate yaml_rust;
extern crate serde_json;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use clap::Parser;
use yaml_rust::YamlLoader;

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

static WEB_REQUEST_DOMAIN: OnceLock<String> = OnceLock::new();

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
                    // TODO: make a client builder here, configure it, then build the client
                    // TODO: I should use a global to store the web client so I can access it from
                    //       anywhere without passing it through a bunch of functions that don't need it
                    //       see what was done a few lines below for the domain
                    //       Will need to figure out if we want the client to be set once here like the domain
                    //       or modified later at some point, ex like changing the timeout it uses for a request
                    let web_client = reqwest::blocking::Client::new();
                    // Set the domain for our web requests
                    // TODO: Set this from a config value
                    WEB_REQUEST_DOMAIN.set("http://127.0.0.1:5000".to_owned()).expect("Failed to set static global for web domain");
                    println!("RUNNING TESTS");
                    for test in tests {
                        let mut test_case_variables: HashMap<String, abstract_syntax_tree::AssignmentValue> = HashMap::new();
                        match test.run_test_case(&mut test_case_variables, &mut tests_passed, &mut tests_failed, 1, &web_client) {
                            Ok(_) => continue,
                            Err(err) => {
                                err.print_error();
                                break;
                            }
                        }
                    }
                    let overall_result = if tests_failed == 0 {"PASSED"} else {"FAILED"};
                    println!("TEST {} WITH {} SUCCESSES AND {} FAILURES", overall_result, tests_passed, tests_failed);
                }
                Err(err) => {
                    err.print_error();
                }
            }
        },
        Err(e) => {
            let marker = e.marker();
            print_error(&format!("Failed to parse {} at line {} col {} with error '{}'", &args.path, marker.line(), marker.col(), e.to_string()));
        }
    }
}
