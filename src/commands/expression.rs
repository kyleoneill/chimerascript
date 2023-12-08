use crate::abstract_syntax_tree::{AssignmentValue, Expression};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

pub fn expression_command(context: &Context, expression: Expression) -> Result<AssignmentValue, ChimeraRuntimeFailure> {
    match expression {
        Expression::LiteralExpression(literal) => {
            Ok(AssignmentValue::Literal(literal))
        },
        Expression::HttpCommand(http_command) => {
            todo!()
        }
    }
}
