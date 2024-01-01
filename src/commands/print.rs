use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssignmentValue, Value};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::{Context, print_in_function};

pub fn print_command(context: &Context, print_cmd: Value, variable_map: &HashMap<String, AssignmentValue>, depth: usize) -> Result <(), ChimeraRuntimeFailure> {
    let resolved = print_cmd.resolve(context, variable_map)?;
    print_in_function(&resolved, depth);
    Ok(())
}
