use crate::abstract_syntax_tree::{Expression, ListCommandOperations, ListExpression, MutateListOperations, Value};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;
use crate::literal::{Collection, Data, DataKind, Literal, NumberKind};
use crate::variable_map::VariableMap;
use crate::CLIENT;
use std::ops::{Deref, DerefMut};

pub fn expression_command(
    context: &Context,
    expression: Expression,
    variable_map: &VariableMap,
) -> Result<Data, ChimeraRuntimeFailure> {
    match expression {
        Expression::Literal(literal) => Ok(Data::from_literal(literal)),
        Expression::HttpCommand(http_command) => {
            let client = CLIENT
                .get()
                .expect("Failed to get web client while resolving an http command");
            let res_obj = client.make_request(context, http_command, variable_map)?;
            Ok(Data::new(res_obj))
        }
        Expression::List(list_expression) => {
            match list_expression {
                ListExpression::New(new_list) => {
                    let mut list: Vec<Data> = Vec::new();
                    for value in new_list {
                        let literal_val = value.resolve(context, variable_map)?;
                        list.push(literal_val);
                    }
                    Ok(Data::from_vec(list))
                }
                ListExpression::ListArgument(list_command) => {
                    match list_command.operation {
                        ListCommandOperations::MutateOperations(ref mutable_operation) => {
                            match variable_map
                                .get_mut(context, list_command.list_name.as_str())?
                                .deref_mut()
                            {
                                DataKind::Collection(Collection::List(mutable_list)) => {
                                    // Cloning in these arms is fine, a clone on Data increments its underlying Rc count
                                    // rather than actually copying the data
                                    match mutable_operation {
                                        MutateListOperations::Append(append_val) => {
                                            let data = append_val.resolve(context, variable_map)?;
                                            mutable_list.push(data.clone());
                                            Ok(data)
                                        }
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
                                                return Err(ChimeraRuntimeFailure::OutOfBounds(
                                                    context.current_line,
                                                ));
                                            }
                                            Ok(mutable_list.remove(index))
                                        }
                                        MutateListOperations::Pop => {
                                            match mutable_list.pop() {
                                                Some(popped_val) => Ok(popped_val),
                                                // Should this be a more precise error? OutOfBounds is technically correct
                                                // but not precise, is it worth making a new error for this specific case?
                                                None => Err(ChimeraRuntimeFailure::OutOfBounds(
                                                    context.current_line,
                                                )),
                                            }
                                        }
                                    }
                                }
                                _ => Err(ChimeraRuntimeFailure::VarWrongType(
                                    list_command.list_name.clone(),
                                    VarTypes::List,
                                    context.current_line,
                                )),
                            }
                        }
                        ListCommandOperations::Length => {
                            let length = variable_map
                                .get(context, list_command.list_name.as_str())?
                                .borrow(context)?
                                .deref()
                                .try_into_list(list_command.list_name.clone(), context)?
                                .len();
                            Ok(Data::from_literal(Literal::Number(NumberKind::U64(
                                length.try_into().expect("Failed to convert usize to u64"),
                            ))))
                        }
                    }
                }
            }
        },
        Expression::FormattedString(formatted_string) => {
            let mut built_str = String::new();
            for value in formatted_string {
                match value {
                    Value::Literal(literal_val) => {
                        match literal_val.to_str() {
                            Some(as_str) => built_str.push_str(as_str),
                            None => return Err(ChimeraRuntimeFailure::InternalError("converting a literal value into a string".to_owned()))
                        }
                    },
                    Value::Variable(ref _var_name) => {
                        let resolved = value.resolve(context, variable_map)?;
                        let binding = resolved.borrow(context)?;
                        built_str.push_str(binding.to_string().as_str());
                    },
                    Value::FormattedString(_) => return Err(ChimeraRuntimeFailure::InternalError("resolving a recursive formatted string".to_owned()))
                }

            }
            Ok(Data::from_literal(Literal::String(built_str)))
        }
    }
}
