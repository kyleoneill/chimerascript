use crate::abstract_syntax_tree::{HTTPVerb, HttpCommand};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;
use crate::literal::{Collection, Data, DataKind, Literal, NumberKind};
use reqwest;
use std::collections::HashMap;

pub trait WebClient {
    fn get_domain(&self) -> &str;
    fn make_request(
        &self,
        context: &Context,
        http_command: HttpCommand,
    ) -> Result<DataKind, ChimeraRuntimeFailure>;
}

#[derive(Debug)]
pub struct RealClient {
    domain: String,
    client: reqwest::blocking::Client,
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
    fn make_request(
        &self,
        context: &Context,
        http_command: HttpCommand,
    ) -> Result<DataKind, ChimeraRuntimeFailure> {
        let resolved_path = http_command.resolve_path(context)?;
        let body_map = http_command.resolve_body(context)?;
        let headers = http_command.resolve_header(context)?;

        // Make the web request
        let res = match http_command.verb {
            HTTPVerb::Get => self
                .client
                .get(resolved_path.as_str())
                .headers(headers)
                .send(),
            HTTPVerb::Delete => self
                .client
                .delete(resolved_path.as_str())
                .headers(headers)
                .send(),
            HTTPVerb::Post => self
                .client
                .post(resolved_path.as_str())
                .json(&body_map)
                .headers(headers)
                .send(),
            HTTPVerb::Put => self
                .client
                .put(resolved_path.as_str())
                .json(&body_map)
                .headers(headers)
                .send(),
        };
        match res {
            Ok(response) => {
                // Have to store the status here as reading the body consumes the response
                let status_code: u64 = response.status().as_u16().into();
                let body: DataKind = response.json().unwrap_or(DataKind::Literal(Literal::Null));
                let mut http_response_obj: HashMap<String, Data> = HashMap::new();
                http_response_obj.insert(
                    "status_code".to_owned(),
                    Data::from_literal(Literal::Number(NumberKind::U64(status_code))),
                );
                http_response_obj.insert("body".to_owned(), Data::new(body));
                Ok(DataKind::Collection(Collection::Object(http_response_obj)))
            }
            Err(_) => Err(ChimeraRuntimeFailure::WebRequestFailure(
                resolved_path,
                context.current_line,
            )),
        }
    }
}
