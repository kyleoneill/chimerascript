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
use util::client::Timer;
use util::client::{RealClient, WebClient};
use util::config::Config;

extern crate reqwest;
extern crate serde;
extern crate serde_json;
use crate::abstract_syntax_tree::ChimeraScriptAST;
use clap::Parser;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

const FILE_EXTENSION: &str = "chs";

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
    name: Option<String>,
    /// Path to a config file
    #[arg(short)]
    config: String,
}

static CLIENT: OnceLock<&(dyn WebClient + Sync)> = OnceLock::new();
static REAL_CLIENT: OnceLock<RealClient> = OnceLock::new();

fn system_checks() {
    if !cfg!(target_pointer_width = "64") {
        panic!("This application must be run on a 64 bit platform.");
    }
}

fn main() {
    system_checks();
    let args = Args::parse();

    let config = match Config::from_path_str(&args.config) {
        Ok(config) => config,
        Err(err_msg) => {
            print_error(&err_msg);
            return;
        }
    };

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
        print_error(&format!(
            "{} has an invalid extension, expected it to be '.chs'",
            &args.path
        ));
        return;
    }

    let file_contents = match fs::read_to_string(&args.path) {
        Ok(res) => res,
        Err(_) => {
            print_error(&format!("Failed to read file {}", &args.path));
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

    match ChimeraScriptAST::new(file_contents.as_str()) {
        Ok(ast) => {
            let timer = Timer::new();
            let test_results = frontend::run_functions(ast);
            let run_time = timer.finish();
            ResultCount::print_test_result(test_results, Some(run_time.as_str()));
        }
        Err(e) => e.print_error(),
    }
}
