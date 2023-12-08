use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssignmentExpr, AssignmentValue};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

pub fn assignment_command(context: &Context, assignment_command: AssignmentExpr, variable_map: &mut HashMap<String, AssignmentValue>) -> Result<(), ChimeraRuntimeFailure> {
    let var_name = assignment_command.var_name;
    let val_to_store = crate::commands::expression::expression_command(context, assignment_command.expression)?;
    variable_map.insert(var_name, val_to_store);
    Ok(())
}
