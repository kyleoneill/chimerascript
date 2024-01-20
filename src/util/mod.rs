use std::collections::HashMap;
use std::time::Duration;
use std::time::Instant;
use reqwest;
use crate::abstract_syntax_tree::{HttpCommand, HTTPVerb};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;
use crate::literal::{Data, Literal, NumberKind};
use crate::variable_map::VariableMap;

pub trait WebClient {
    fn get_domain(&self) -> &str;
    fn make_request(&self, context: &Context, http_command: HttpCommand, variable_map: &VariableMap) -> Result<Literal, ChimeraRuntimeFailure>;
}

#[derive(Debug)]
pub struct RealClient {
    domain: String,
    client: reqwest::blocking::Client
}

impl RealClient {
    pub fn new(domain: String, client: reqwest::blocking::Client) -> Self {
        Self { domain, client }
    }
}

impl WebClient for RealClient {
    fn get_domain(&self) -> &str {
        self.domain.as_str()
    }
    fn make_request(&self, context: &Context, http_command: HttpCommand, variable_map: &VariableMap) -> Result<Literal, ChimeraRuntimeFailure> {
        let resolved_path = http_command.resolve_path(context, variable_map)?;
        let body_map = http_command.resolve_body(context, variable_map)?;

        // Make the web request
        let res = match http_command.verb {
            HTTPVerb::GET => {
                self.client.get(resolved_path.as_str()).send()
            },
            HTTPVerb::DELETE => {
                self.client.delete(resolved_path.as_str()).send()
            },
            HTTPVerb::POST => {
                self.client.post(resolved_path.as_str()).json(&body_map).send()
            },
            HTTPVerb::PUT => {
                self.client.put(resolved_path.as_str()).json(&body_map).send()
            }
        };
        match res {
            Ok(response) => {
                // Have to store the status here as reading the body consumes the response
                let status_code: u64 = response.status().as_u16().try_into().expect("Failed to convert a u16 to a u64");
                let body: Literal = response.json().unwrap_or_else(|_| Literal::Null);
                let mut http_response_obj: HashMap<String, Data> = HashMap::new();
                http_response_obj.insert("status_code".to_owned(), Data::from_literal(Literal::Number(NumberKind::U64(status_code))));
                http_response_obj.insert("body".to_owned(), Data::from_literal(body));
                Ok(Literal::Object(http_response_obj))
            },
            Err(_) => Err(ChimeraRuntimeFailure::WebRequestFailure(resolved_path, context.current_line))
        }
    }
}

pub struct Timer {
    start: Instant
}

impl Timer {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }
    pub fn finish(&self) -> String {
        let duration = self.start.elapsed();
        duration_to_readable_time(duration)
    }
}

fn duration_to_readable_time(duration: Duration) -> String {
    let as_secs = duration.as_secs();
    match as_secs {
        0 => {
            let as_millis = duration.as_millis();
            match as_millis {
                0 => {
                    format!("{}μ", duration.as_micros())
                },
                _ => format!("{}ms", as_millis)
            }
        },
        _ => format!("{}s", as_secs)
    }
}
