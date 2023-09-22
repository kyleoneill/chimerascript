#[derive(Debug)]
pub enum ChimeraError {
    InvalidChimeraFile(String),
    FailedParseAST(String)
}

pub fn print_error(err_msg: &str) {
    eprintln!("ERROR: {}", err_msg);
}
