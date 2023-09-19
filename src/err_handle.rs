#[derive(Debug)]
pub enum ChimeraError {
    FailedToParseFromYaml,
    InvalidChimeraFile,
    ChimeraFileNoName,
    ChimeraFileNoSteps,
    SubtestInSetupOrTeardown
}

pub fn print_error(err_msg: &str) {
    eprintln!("ERROR: {}", err_msg);
}
