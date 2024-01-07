mod err_handle;
mod frontend;
mod abstract_syntax_tree;
mod commands;
mod literal;
mod testing;

use err_handle::print_error;
use frontend::ResultCount;

extern crate reqwest;
extern crate serde;
extern crate serde_json;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use clap::Parser;
use crate::abstract_syntax_tree::ChimeraScriptAST;

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

fn system_checks() {
    if !cfg!(target_pointer_width = "64") {
        panic!("This application must be run on a 64 bit platform.");
    }
}

fn main() {
    system_checks();
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

    let file_contents = match fs::read_to_string(&args.path) {
        Ok(res) => res,
        Err(_) => {
            print_error(&format!("Failed to read file {}", &args.path));
            return;
        }
    };

    // TODO: make a client builder here, configure it, then build the client
    // TODO: I should use a global to store the web client so I can access it from
    //       anywhere without passing it through a bunch of functions that don't need it.
    //       See what was done a few lines below for the domain.
    //       Will need to figure out if we want the client to be set once here like the domain
    //       or modified later at some point, ex like changing the timeout it uses for a request
    let web_client = reqwest::blocking::Client::new();
    // Set the domain for our web requests
    // TODO: Set this from a config value
    WEB_REQUEST_DOMAIN.set("http://127.0.0.1:5000".to_owned()).expect("Failed to set static global for web domain");

    match ChimeraScriptAST::new(file_contents.as_str()) {
        Ok(ast) => {
            let test_results = frontend::run_functions(ast, &web_client);
            ResultCount::print_test_result(test_results);
        },
        Err(e) => e.print_error()
    }
}
