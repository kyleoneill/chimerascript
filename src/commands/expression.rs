use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssignmentValue, Expression};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;
use crate::WEB_REQUEST_DOMAIN;
use crate::abstract_syntax_tree::HTTPVerb;

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

            // Add our query params to the request URL
            let mut has_added_a_param = false;
            for param_pair in http_command.http_assignments {
                let pair_key = param_pair.lhs;
                let pair_val = param_pair.rhs.resolve(context, variable_map)?.to_string();
                // TODO: Convert pair_val with URL escapes
                //       ex, space must be replaced with %20
                if !has_added_a_param {
                    let formatted = format!("?{}={}", pair_key, pair_val);
                    resolved_path.push_str(formatted.as_str());
                    has_added_a_param = true;
                }
                else {
                    let formatted = format!("&{}={}", pair_key, pair_val);
                    resolved_path.push_str(formatted.as_str());
                }
            }

            // Make the web request
            let res = match http_command.verb {
                HTTPVerb::GET => {
                    web_client.get(resolved_path).send()
                },
                HTTPVerb::DELETE => {

                },
                HTTPVerb::POST => {
                    // TODO: Body params
                },
                HTTPVerb::PUT => {
                    // TODO: Body params
                }
            };
            match res {
                Ok(successful_res) => {
                    todo!()
                    // convert this into an AssignmentValue
                    // have to add some new type to represent the val here
                },
                Err(_) => Err(ChimeraRuntimeFailure::WebRequestFailure(http_command.path.clone(), context.current_line))
            }
        }
    }
}
