use std::collections::HashMap;
use crate::literal::{Literal, NumberKind};
use crate::abstract_syntax_tree::{AssignmentValue, Expression, HTTPVerb, HttpResponse, ListExpression, ListCommandOperations, MutateListOperations};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

pub fn expression_command(context: &Context, expression: Expression, variable_map: &mut HashMap<String, AssignmentValue>, web_client: &reqwest::blocking::Client) -> Result<AssignmentValue, ChimeraRuntimeFailure> {
    match expression {
        Expression::LiteralExpression(literal) => { Ok(AssignmentValue::Literal(literal)) },
        Expression::HttpCommand(http_command) => {
            let resolved_path = http_command.resolve_path(context, variable_map)?;
            // TODO: need to go through resolved_path and URL escape anything that has to be
            //       escaped, ex space has to be replaced with %20
            // TODO: need to go through resolved_path and fill in any variable query params, ex
            //       - GET /foo?count=(my_count_var)
            //       See the to do in abstract_syntax_tree::parse_rule_to_path about this

            // construct request body
            let mut body_map: HashMap<String, String> = HashMap::new();
            for assignment in http_command.http_assignments {
                let key = assignment.lhs;
                let val = assignment.rhs.resolve(context, variable_map)?.to_string();
                body_map.insert(key, val);
            }

            // Make the web request
            let res = match http_command.verb {
                HTTPVerb::GET => {
                    web_client.get(resolved_path.as_str()).send()
                },
                HTTPVerb::DELETE => {
                    web_client.delete(resolved_path.as_str()).send()
                },
                HTTPVerb::POST => {
                    web_client.post(resolved_path.as_str()).json(&body_map).send()
                },
                HTTPVerb::PUT => {
                    web_client.put(resolved_path.as_str()).json(&body_map).send()
                }
            };
            match res {
                Ok(response) => {
                    // Have to store the status here as reading the body consumes the response
                    let status_code: u64 = response.status().as_u16().try_into().expect("Failed to convert a u16 to a u64");
                    let body: Literal = response.json().unwrap_or_else(|_| Literal::Null);
                    let http_response = HttpResponse{ status_code, body };
                    Ok(AssignmentValue::HttpResponse(http_response))
                },
                Err(_) => Err(ChimeraRuntimeFailure::WebRequestFailure(resolved_path, context.current_line))
            }
        },
        Expression::ListExpression(list_expression) => {
            match list_expression {
                ListExpression::New(new_list) => {
                    let mut literal_list: Vec<Literal> = Vec::new();
                    for value in new_list {
                        let literal_val = value.resolve_to_literal(context, variable_map)?;
                        literal_list.push(literal_val);
                    }
                    Ok(AssignmentValue::Literal(Literal::List(literal_list)))
                },
                ListExpression::ListArgument(list_command) => {
                    match list_command.operation {
                        ListCommandOperations::MutateOperations(ref mutable_operation) => {
                            // can't get the list here because it causes an immutable borrow of variable_map
                            // variable map is also only used to get a lhs in some arms, so we can't get that with the
                            // list here either
                            match mutable_operation {
                                MutateListOperations::Append(append_val) => {
                                    let literal = append_val.resolve_to_literal(context, variable_map)?;
                                    let list = list_command.list_mut_ref(variable_map, context)?;
                                    list.push(literal.clone());
                                    Ok(AssignmentValue::Literal(literal))
                                },
                                MutateListOperations::Remove(remove_val) => {
                                    match remove_val.resolve_to_literal(context, variable_map)? {
                                        Literal::Number(num) => {
                                            let index = num.to_usize().ok_or_else(|| return ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))?;
                                            let list = list_command.list_mut_ref(variable_map, context)?;
                                            if index >= list.len() {
                                                return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                                            }
                                            let removed_val = list.remove(index);
                                            Ok(AssignmentValue::Literal(removed_val))
                                        },
                                        _ => return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                                    }
                                },
                                MutateListOperations::Pop => {
                                    let list = list_command.list_mut_ref(variable_map, context)?;
                                    match list.pop() {
                                        Some(popped_val) => Ok(AssignmentValue::Literal(popped_val)),
                                        // Should this be a more precise error? OutOfBounds is technically correct
                                        // but not precise, is it worth making a new error for this specific case?
                                        None => Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                                    }
                                }
                            }
                        },
                        ListCommandOperations::Length => {
                            let list = list_command.list_ref(variable_map, context)?;
                            Ok(AssignmentValue::Literal(Literal::Number(NumberKind::U64(list.len().try_into().expect("Failed to convert usize to u64")))))
                        }
                    }
                }
            }
        }
    }
}
