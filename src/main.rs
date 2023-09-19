mod err_handle;
mod frontend;

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
    // But it SHOULD error when being passed a single file
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
            // TODO: iterate_yaml should just be parsing the yaml, _NOT_ running the test
            match frontend::iterate_yaml(file_yaml.remove(0)) {
                Ok(test_result) => {
                    let res = if test_result.1 == 0 {"PASSED"} else {"FAILED"};
                    println!("TEST {} WITH {} SUCCESSES AND {} FAILURES", res, test_result.0, test_result.1);
                }
                Err(f) => {
                    // TODO: Need error handling here
                    print_error("TODO: GIVE ME AN ERROR MSG");
                }
            }
        },
        Err(e) => {
            let marker = e.marker();
            print_error(&format!("Failed to parse {} at line {} col {} with error '{}'", &args.path, marker.line(), marker.col(), e.to_string()));
        }
    }
}
