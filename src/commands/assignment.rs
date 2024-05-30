use crate::abstract_syntax_tree::AssignmentExpr;
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

pub fn assignment_command(
    context: &mut Context,
    assignment_command: AssignmentExpr,
) -> Result<(), ChimeraRuntimeFailure> {
    let val_to_store =
        crate::commands::expression::expression_command(context, assignment_command.expression)?;
    context.store_data(assignment_command.var_name, val_to_store);
    Ok(())
}
