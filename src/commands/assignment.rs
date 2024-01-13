use crate::variable_map::VariableMap;
use crate::abstract_syntax_tree::AssignmentExpr;
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

pub fn assignment_command(context: &Context, assignment_command: AssignmentExpr, variable_map: &mut VariableMap, web_client: &reqwest::blocking::Client) -> Result<(), ChimeraRuntimeFailure> {
    let val_to_store = crate::commands::expression::expression_command(context, assignment_command.expression, variable_map, web_client)?;
    variable_map.insert(assignment_command.var_name, val_to_store);
    Ok(())
}
