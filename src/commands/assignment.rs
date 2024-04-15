use crate::abstract_syntax_tree::AssignmentExpr;
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;
use crate::variable_map::VariableMap;

pub fn assignment_command(
    context: &Context,
    assignment_command: AssignmentExpr,
    variable_map: &mut VariableMap,
) -> Result<(), ChimeraRuntimeFailure> {
    let val_to_store = crate::commands::expression::expression_command(
        context,
        assignment_command.expression,
        variable_map,
    )?;
    variable_map.insert(assignment_command.var_name, val_to_store);
    Ok(())
}
