use std::fmt::{Display, Formatter};

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
pub enum VarTypes {
    Int,
    String,
    HttpResponse,
    List,
    Containable,
    Literal
}

impl Display for VarTypes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VarTypes::Int => write!(f, "Int"),
            VarTypes::String => write!(f, "String"),
            VarTypes::HttpResponse => write!(f, "HttpResponse"),
            VarTypes::List => write!(f, "List"),
            VarTypes::Containable => write!(f, "List or Object"),
            VarTypes::Literal => write!(f, "null, number, bool, string, array, or object")
        }
    }
}

#[derive(Debug)]
pub enum ChimeraRuntimeFailure {
    VarNotFound(String, i32),
    VarWrongType(String, VarTypes, i32),
    TestFailure(String, i32),
    InternalError(String),
    WebRequestFailure(String, i32),
    BadSubfieldAccess(Option<String>, String, i32),
    TriedToIndexWithNonNumber(i32),
    OutOfBounds(i32)
}

impl Display for ChimeraRuntimeFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // TODO: There are now a lot of runtime failure variants. These should probably be broken up into different runtime categories
            //       like "array access" errors for TriedToIndexWithNonNumber and OutOfBounds or "variable errors" for
            //       VarNotFound, VarWrongType, and BadSubfieldAccess
            ChimeraRuntimeFailure::TestFailure(msg, line) => write!(f, "FAILURE on line {}: {}", line, msg),
            ChimeraRuntimeFailure::VarNotFound(var_name, line) => write!(f, "ERROR on line {}: var '{}' was accessed but is not set", line, var_name),
            ChimeraRuntimeFailure::VarWrongType(var_name, expected_type, line) => write!(f, "ERROR on line {}: '{}' was expected to be of type {} but it was not", line, var_name, expected_type),
            ChimeraRuntimeFailure::InternalError(action) => write!(f, "Internal error while {}", action),
            ChimeraRuntimeFailure::WebRequestFailure(endpoint, line) => write!(f, "ERROR on line {}: Failed to make request for endpoint '{}'", line, endpoint),
            ChimeraRuntimeFailure::BadSubfieldAccess(var_name, subfield, line) => {
                // This is not ideal, should fix it later. Issue here is passing around the variable name through helper functions which do not need
                // the original variable name JUST so we can error handle
                match var_name {
                    Some(v_name) => write!(f, "ERROR on line {}: Failed to access subfield '{}' for variable '{}'", line, subfield, v_name),
                    None => write!(f, "ERROR on line {}: Failed to access subfield '{}'", line, subfield)
                }
            }
            ChimeraRuntimeFailure::TriedToIndexWithNonNumber(line) => write!(f, "ERROR on line {}: Tried to index an array with a non-numerical value", line),
            ChimeraRuntimeFailure::OutOfBounds(line) => write!(f, "ERROR on line {}: Tried to access an array with an out-of-bounds value", line)
        }
    }
}

impl ChimeraRuntimeFailure {
    pub fn print_error(&self) {
        eprintln!("{}", self);
    }
}

pub fn print_error(err_msg: &str) {
    eprintln!("ERROR: {}", err_msg);
}
