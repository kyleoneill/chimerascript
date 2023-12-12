use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssignmentValue, Expression};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;
use crate::WEB_REQUEST_DOMAIN;
use crate::abstract_syntax_tree::{HTTPVerb, HttpResponse};
use serde_json::Value;

pub fn expression_command(context: &Context, expression: Expression, variable_map: &mut HashMap<String, AssignmentValue>, web_client: &reqwest::blocking::Client) -> Result<AssignmentValue, ChimeraRuntimeFailure> {
    match expression {
        Expression::LiteralExpression(literal) => {
            Ok(AssignmentValue::Literal(literal))
        },
        Expression::HttpCommand(http_command) => {
            // Build our URL from the domain and our path
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
                    web_client.post(resolved_path).json(&body_map).send()
                }
            };
            match res {
                Ok(response) => {
                    let http_response = HttpResponse{ status_code: response.status().as_u16(), body: response.json().ok() };
                    Ok(AssignmentValue::HttpResponse(http_response))
                },
                Err(_) => Err(ChimeraRuntimeFailure::WebRequestFailure(http_command.path.clone(), context.current_line))
            }
        }
    }
}
