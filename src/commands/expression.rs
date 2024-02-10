use std::ops::{Deref, DerefMut};
use crate::variable_map::VariableMap;
use crate::literal::{Literal, NumberKind, Data, DataKind, Collection};
use crate::abstract_syntax_tree::{Expression, ListExpression, ListCommandOperations, MutateListOperations};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;
use crate::CLIENT;

pub fn expression_command(context: &Context, expression: Expression, variable_map: &VariableMap) -> Result<Data, ChimeraRuntimeFailure> {
    match expression {
        Expression::LiteralExpression(literal) => { Ok(Data::from_literal(literal)) },
        Expression::HttpCommand(http_command) => {
            let client = CLIENT.get().expect("Failed to get web client while resolving an http command");
            let res_obj = client.make_request(context, http_command, variable_map)?;
            Ok(Data::new(res_obj))
        },
        Expression::ListExpression(list_expression) => {
            match list_expression {
                ListExpression::New(new_list) => {
                    let mut list: Vec<Data> = Vec::new();
                    for value in new_list {
                        let literal_val = value.resolve(context, variable_map)?;
                        list.push(literal_val);
                    }
                    Ok(Data::from_vec(list))
                },
                ListExpression::ListArgument(list_command) => {
                    match list_command.operation {
                        ListCommandOperations::MutateOperations(ref mutable_operation) => {
                            match variable_map.get_mut(context, list_command.list_name.as_str())?.deref_mut() {
                                DataKind::Collection(c) => {
                                    match c {
                                        Collection::List(mutable_list) => {
                                            // Cloning in these arms is fine, a clone on Data increments its underlying Rc count
                                            // rather than actually copying the data
                                            match mutable_operation {
                                                MutateListOperations::Append(append_val) => {
                                                    let data = append_val.resolve(context, variable_map)?;
                                                    mutable_list.push(data.clone());
                                                    Ok(data)
                                                },
                                                MutateListOperations::Remove(remove_val) => {
                                                    let index = match remove_val
                                                        .resolve(context, variable_map)?
                                                        .borrow(context)?
                                                        .deref()
                                                        .try_into_usize(remove_val, context) {
                                                        Ok(i) => i,
                                                        Err(_) => return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                                                    };
                                                    if index >= mutable_list.len() {
                                                        return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                                                    }
                                                    Ok(mutable_list.remove(index))
                                                },
                                                MutateListOperations::Pop => {
                                                    match mutable_list.pop() {
                                                        Some(popped_val) => Ok(popped_val),
                                                        // Should this be a more precise error? OutOfBounds is technically correct
                                                        // but not precise, is it worth making a new error for this specific case?
                                                        None => Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                                                    }
                                                }
                                            }
                                        },
                                        _ => Err(ChimeraRuntimeFailure::VarWrongType(list_command.list_name.clone(), VarTypes::List, context.current_line))
                                    }
                                },
                                _ => Err(ChimeraRuntimeFailure::VarWrongType(list_command.list_name.clone(), VarTypes::List, context.current_line))
                            }
                        },
                        ListCommandOperations::Length => {
                            let length = variable_map.get(context, list_command.list_name.as_str())?
                                .borrow(context)?
                                .deref()
                                .try_into_list(list_command.list_name.clone(), context)?
                                .len();
                            Ok(Data::from_literal(Literal::Number(NumberKind::U64(length.try_into().expect("Failed to convert usize to u64")))))
                        }
                    }
                }
            }
        }
    }
}
