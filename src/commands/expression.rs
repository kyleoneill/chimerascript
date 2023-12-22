use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssignmentValue, Expression, HTTPVerb, HttpResponse, ListExpression, Literal, ListCommandOperations, MutateListOperations};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;
use crate::WEB_REQUEST_DOMAIN;

pub fn expression_command(context: &Context, expression: Expression, variable_map: &mut HashMap<String, AssignmentValue>, web_client: &reqwest::blocking::Client, variable_name: Option<String>) -> Result<AssignmentValue, ChimeraRuntimeFailure> {
    match expression {
        Expression::LiteralExpression(literal) => {
            Ok(AssignmentValue::Literal(literal))
        },
        Expression::HttpCommand(http_command) => {
            // Build URL from the domain and path
            let domain = WEB_REQUEST_DOMAIN.get().expect("Failed to get static global domain when resolving an HTTP expression");
            let mut resolved_path: String = domain.clone();
            resolved_path.push_str(http_command.path.as_str());

            // TODO: need to go through resolved_path and URL escape anything that has to be
            //       escaped, ex space has to be replaced with %20
            // TODO: need to go through resolved_path and fill in any variable query params, ex
            //       - GET /foo?count=(my_count_var)

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
                    web_client.get(resolved_path).send()
                },
                HTTPVerb::DELETE => {
                    web_client.delete(resolved_path).send()
                },
                HTTPVerb::POST => {
                    web_client.post(resolved_path).json(&body_map).send()
                },
                HTTPVerb::PUT => {
                    web_client.put(resolved_path).json(&body_map).send()
                }
            };
            match res {
                Ok(response) => {
                    // Have to store the status here as reading the body consumes the response
                    let status_code = response.status().as_u16();
                    let body: Literal = response.json().unwrap_or_else(|_| Literal::Null);
                    let http_response = HttpResponse{ status_code, body, var_name: variable_name.expect("Resolved an expression to set a variable without passing the variable name") };
                    Ok(AssignmentValue::HttpResponse(http_response))
                },
                Err(_) => Err(ChimeraRuntimeFailure::WebRequestFailure(http_command.path.clone(), context.current_line))
            }
        },
        Expression::ListExpression(list_expression) => {
            // TODO: Add a LIST POP
            // TODO: Add the ability to make an empty list, it currently _must_ be initialized with one value.
            //       In grammar.pest will need to edit ListNew to look like { ... ~ Value? } to allow the last value to be present
            //       0 or 1 times and then will need to update the AST to handle the now optional token correctly
            // TODO: Add a LIST EMPTY to empty a list in one op rather than removing repeatedly?
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
                                            let index = num as usize;
                                            let list = list_command.list_mut_ref(variable_map, context)?;
                                            if index >= list.len() {
                                                return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                                            }
                                            let removed_val = list.remove(index);
                                            Ok(AssignmentValue::Literal(removed_val))
                                        },
                                        _ => return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                                    }
                                }
                            }
                        },
                        ListCommandOperations::Length => {
                            let list = list_command.list_ref(variable_map, context)?;
                            Ok(AssignmentValue::Literal(Literal::Number(list.len() as i64)))
                        }
                    }
                }
            }
        }
    }
}
