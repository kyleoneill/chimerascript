use crate::variable_map::VariableMap;
use crate::abstract_syntax_tree::Value;
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::{Context, print_in_function};

pub fn print_command(context: &Context, print_cmd: Value, variable_map: &VariableMap, depth: usize) -> Result <(), ChimeraRuntimeFailure> {
    let resolved = print_cmd.resolve(context, variable_map)?;
    print_in_function(&resolved, depth);
    Ok(())
}
