use std::ops::{Deref, DerefMut};
use crate::variable_map::VariableMap;
use crate::literal::{Literal, NumberKind, Data};
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
            Ok(Data::from_literal(res_obj))
        },
        Expression::ListExpression(list_expression) => {
            match list_expression {
                ListExpression::New(new_list) => {
                    let mut literal_list: Vec<Data> = Vec::new();
                    for value in new_list {
                        let literal_val = value.resolve(context, variable_map)?;
                        literal_list.push(literal_val);
                    }
                    Ok(Data::from_literal(Literal::List(literal_list)))
                },
                ListExpression::ListArgument(list_command) => {
                    match list_command.operation {
                        ListCommandOperations::MutateOperations(ref mutable_operation) => {
                            match variable_map.get_mut(context, list_command.list_name.as_str())?.deref_mut() {
                                Literal::List(mutable_list) => {
                                    // Cloning in these arms is fine, a clone on Data increments its underlying Rc count
                                    // rather than actually copying the data
                                    match mutable_operation {
                                        MutateListOperations::Append(append_val) => {
                                            let literal = append_val.resolve(context, variable_map)?;
                                            mutable_list.push(literal.clone());
                                            Ok(literal)
                                        },
                                        MutateListOperations::Remove(remove_val) => {
                                            match remove_val.resolve(context, variable_map)?.borrow(context)?.deref() {
                                                Literal::Number(num) => {
                                                    let index = num.to_usize().ok_or_else(|| return ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))?;
                                                    if index >= mutable_list.len() {
                                                        return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                                                    }
                                                    Ok(mutable_list.remove(index))
                                                },
                                                _ => Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                                            }
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
                        ListCommandOperations::Length => {
                            match variable_map.get(context, list_command.list_name.as_str())?.borrow(context)?.deref() {
                                Literal::List(list_ref) => {
                                    let list_len = Data::from_literal(Literal::Number(NumberKind::U64(list_ref.len().try_into().expect("Failed to convert usize to u64"))));
                                    Ok(list_len)
                                },
                                _ => Err(ChimeraRuntimeFailure::VarWrongType(list_command.list_name.clone(), VarTypes::List, context.current_line))
                            }
                        }
                    }
                }
            }
        }
    }
}
