#[derive(Debug)]
pub enum ChimeraCompileError {
    InvalidChimeraFile(String),
    FailedParseAST(String)
}

impl ChimeraCompileError {
    pub fn print_error(&self) {
        eprint!("ERROR: ");
        match self {
            ChimeraCompileError::InvalidChimeraFile(msg) => eprintln!("Invalid ChimeraScript file. {}", msg),
            ChimeraCompileError::FailedParseAST(msg) => eprintln!("Failed to parse tokens into AST, {}", msg)
        }
    }
}

#[derive(Debug)]
pub enum ChimeraRuntimeFailure {
    VarNotFound(String, i32),
    VarWrongType(String, String, i32),
    TestFailure(String, i32)
}

impl ChimeraRuntimeFailure {
    pub fn print_error(&self) {
        match self {
            ChimeraRuntimeFailure::TestFailure(msg, line) => eprintln!("FAILURE on line {}: {}", line, msg),
            ChimeraRuntimeFailure::VarNotFound(var_name, line) => eprintln!("ERROR on line {}: var {} was accessed but is not set", line, var_name),
            ChimeraRuntimeFailure::VarWrongType(var_name, expected_type, line) => eprintln!("ERROR on line {}: var {} was expected to be of type {} but it was not", line, var_name, expected_type)
        }
    }
}

pub fn print_error(err_msg: &str) {
    eprintln!("ERROR: {}", err_msg);
}
