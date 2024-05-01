use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct CLIError {
    error_msg: String,
    // TODO: This is a hack to reconcile resolving two different error types in main.rs
    //       Should really handle this in a cleaner way
    pub already_reported: bool
}

impl CLIError {
    pub fn new(error_msg: String, already_reported: bool) -> Self {
        Self {
            error_msg,
            already_reported
        }
    }
    pub fn print_error(&self) {
        print_error(self.error_msg.as_str());
    }
}

#[derive(Debug)]
pub struct ChimeraCompileError {
    error_msg: String,
    line: usize,
    column: usize,
}

impl ChimeraCompileError {
    pub fn new(error_str: &str, line_col: (usize, usize)) -> Self {
        ChimeraCompileError {
            error_msg: error_str.to_owned(),
            line: line_col.0,
            column: line_col.1,
        }
    }

    pub fn print_error(&self) {
        eprintln!(
            "Failed to compile ChimeraScript with error '{}' on line {} column {}",
            self.error_msg, self.line, self.column
        );
    }
}

#[derive(Debug, PartialEq)]
pub enum VarTypes {
    Number,
    Unsigned,
    String,
    HttpResponse,
    List,
    Containable,
    Literal,
}

impl Display for VarTypes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VarTypes::Number => write!(f, "Number"),
            VarTypes::Unsigned => write!(f, "Unsigned Integer"),
            VarTypes::String => write!(f, "String"),
            VarTypes::HttpResponse => write!(f, "HttpResponse"),
            VarTypes::List => write!(f, "List"),
            VarTypes::Containable => write!(f, "List or Object"),
            VarTypes::Literal => write!(f, "Literal (number, bool, string, or null)"),
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
    OutOfBounds(i32),
    BorrowError(i32, String),
    InvalidHeader(i32, String),
}

impl Display for ChimeraRuntimeFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // TODO: There are now a lot of runtime failure variants. These should probably be broken up into different runtime categories
            //       like "array access" errors for TriedToIndexWithNonNumber and OutOfBounds or "variable errors" for
            //       VarNotFound, VarWrongType, and BadSubfieldAccess
            ChimeraRuntimeFailure::TestFailure(msg, line) => {
                write!(f, "FAILURE on line {}: {}", line, msg)
            }
            ChimeraRuntimeFailure::VarNotFound(var_name, line) => write!(
                f,
                "ERROR on line {}: var '{}' was accessed but is not set",
                line, var_name
            ),
            ChimeraRuntimeFailure::VarWrongType(var_name, expected_type, line) => write!(
                f,
                "ERROR on line {}: '{}' was expected to be of type {} but it was not",
                line, var_name, expected_type
            ),
            ChimeraRuntimeFailure::InternalError(action) => {
                write!(f, "Internal error while {}", action)
            }
            ChimeraRuntimeFailure::WebRequestFailure(endpoint, line) => write!(
                f,
                "ERROR on line {}: Failed to make request for endpoint '{}'",
                line, endpoint
            ),
            ChimeraRuntimeFailure::BadSubfieldAccess(var_name, subfield, line) => {
                // This is not ideal, should fix it later. Issue here is passing around the variable name through helper functions which do not need
                // the original variable name JUST so we can error handle
                match var_name {
                    Some(v_name) => write!(
                        f,
                        "ERROR on line {}: Failed to access subfield '{}' for variable '{}'",
                        line, subfield, v_name
                    ),
                    None => write!(
                        f,
                        "ERROR on line {}: Failed to access subfield '{}'",
                        line, subfield
                    ),
                }
            }
            ChimeraRuntimeFailure::TriedToIndexWithNonNumber(line) => write!(
                f,
                "ERROR on line {}: Arrays can only be indexed with an unsigned integer",
                line
            ),
            ChimeraRuntimeFailure::OutOfBounds(line) => write!(
                f,
                "ERROR on line {}: Tried to access an array with an out-of-bounds value",
                line
            ),
            ChimeraRuntimeFailure::BorrowError(line, reason) => {
                write!(f, "ERROR on line {}: {}", line, reason)
            }
            ChimeraRuntimeFailure::InvalidHeader(line, header) => write!(
                f,
                "ERROR on line {}: Header '{}' is not valid",
                line, header
            ),
        }
    }
}

impl PartialEq for ChimeraRuntimeFailure {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ChimeraRuntimeFailure::VarNotFound(_, _) => {
                matches!(other, ChimeraRuntimeFailure::VarNotFound(_, _))
            }
            ChimeraRuntimeFailure::VarWrongType(_, _, _) => {
                matches!(other, ChimeraRuntimeFailure::VarWrongType(_, _, _))
            }
            ChimeraRuntimeFailure::TestFailure(_, _) => {
                matches!(other, ChimeraRuntimeFailure::TestFailure(_, _))
            }
            ChimeraRuntimeFailure::InternalError(_) => {
                matches!(other, ChimeraRuntimeFailure::InternalError(_))
            }
            ChimeraRuntimeFailure::WebRequestFailure(_, _) => {
                matches!(other, ChimeraRuntimeFailure::WebRequestFailure(_, _))
            }
            ChimeraRuntimeFailure::BadSubfieldAccess(_, _, _) => {
                matches!(other, ChimeraRuntimeFailure::BadSubfieldAccess(_, _, _))
            }
            ChimeraRuntimeFailure::TriedToIndexWithNonNumber(_) => {
                matches!(other, ChimeraRuntimeFailure::TriedToIndexWithNonNumber(_))
            }
            ChimeraRuntimeFailure::OutOfBounds(_) => {
                matches!(other, ChimeraRuntimeFailure::OutOfBounds(_))
            }
            ChimeraRuntimeFailure::BorrowError(_, _) => {
                matches!(other, ChimeraRuntimeFailure::BorrowError(_, _))
            }
            ChimeraRuntimeFailure::InvalidHeader(_, _) => {
                matches!(other, ChimeraRuntimeFailure::InvalidHeader(_, _))
            }
        }
    }
}

impl ChimeraRuntimeFailure {
    pub fn print_error(&self, padding: usize) {
        eprintln!("{:indent$}{}", "", self, indent = padding + 1);
    }

    #[allow(dead_code)] // Used by tests
    pub fn get_variant_name(&self) -> &str {
        match self {
            ChimeraRuntimeFailure::VarNotFound(_, _) => "VarNotFound",
            ChimeraRuntimeFailure::VarWrongType(_, _, _) => "VarWrongType",
            ChimeraRuntimeFailure::TestFailure(_, _) => "TestFailure",
            ChimeraRuntimeFailure::InternalError(_) => "InternalError",
            ChimeraRuntimeFailure::WebRequestFailure(_, _) => "WebRequestFailure",
            ChimeraRuntimeFailure::BadSubfieldAccess(_, _, _) => "BadSubfieldAccess",
            ChimeraRuntimeFailure::TriedToIndexWithNonNumber(_) => "TriedToIndexWithNonNumber",
            ChimeraRuntimeFailure::OutOfBounds(_) => "OutOfBounds",
            ChimeraRuntimeFailure::BorrowError(_, _) => "BorrowError",
            ChimeraRuntimeFailure::InvalidHeader(_, _) => "InvalidHeader",
        }
    }
}

pub fn print_error(err_msg: &str) {
    eprintln!("ERROR: {}", err_msg);
}
