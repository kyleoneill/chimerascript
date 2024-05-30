use crate::abstract_syntax_tree::Value;
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::{print_in_function, Context};
use crate::literal::DataKind;
use std::io::Write;
use std::ops::Deref;

pub fn print_command<W: Write>(
    context: &Context,
    writer: &mut W,
    print_cmd: Value,
    depth: usize,
) -> Result<(), ChimeraRuntimeFailure> {
    let resolved = print_cmd.resolve(context)?;
    let borrowed = resolved.borrow()?;
    match borrowed.deref() {
        DataKind::Literal(literal) => print_in_function(writer, literal, depth),
        DataKind::Collection(collection) => print_in_function(writer, collection, depth),
    }
    Ok(())
}
