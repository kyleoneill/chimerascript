use crate::abstract_syntax_tree::{HTTPVerb, HttpCommand};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;
use crate::literal::{Collection, Data, DataKind, Literal, NumberKind};
use crate::util::client::WebClient;
use std::collections::HashMap;

#[derive(Debug)]
pub struct FakeClient {
    domain: String,
}

impl FakeClient {
    #[allow(dead_code)] // Used in test
    pub fn new(s: &str) -> Self {
        let domain = s.to_owned();
        Self { domain }
    }
}

impl WebClient for FakeClient {
    fn get_domain(&self) -> &str {
        self.domain.as_str()
    }
    fn make_request(
        &self,
        context: &Context,
        http_command: HttpCommand,
    ) -> Result<DataKind, ChimeraRuntimeFailure> {
        let mut response_obj: HashMap<String, Data> = HashMap::new();
        response_obj.insert(
            "status_code".to_owned(),
            Data::from_literal(Literal::Number(NumberKind::U64(match http_command.verb {
                HTTPVerb::Get => 200,
                HTTPVerb::Delete => 200,
                HTTPVerb::Post => 201,
                HTTPVerb::Put => 200,
            }))),
        );

        // Take a request and extract the query and body params from it
        let mut resolved_body: HashMap<String, Data> = HashMap::new();
        for assignment in &http_command.http_assignments {
            let key = assignment.lhs.clone();
            let value = assignment.rhs.resolve(context)?;
            resolved_body.insert(key, value);
        }
        let mut query_params: HashMap<String, Data> = HashMap::new();
        for query_param in &http_command.query_params {
            let key = query_param.lhs.clone();
            let value = query_param.rhs.resolve(context)?;
            query_params.insert(key, value);
        }
        let raw_headers = &http_command.resolve_header(context)?;
        let mut headers: HashMap<String, Data> = HashMap::new();
        for (key, value) in raw_headers.iter() {
            let deserializable_value = format!("\"{}\"", value.to_str().unwrap());
            let data = Data::new(serde_json::from_slice(deserializable_value.as_bytes()).unwrap());
            headers.insert(key.to_string(), data);
        }

        // Construct a response struct out of the request params
        let mut body_data: HashMap<String, Data> = HashMap::new();
        let resolved_path = http_command.resolve_path(context)?;
        body_data.insert(
            "path".to_owned(),
            Data::new(DataKind::Literal(Literal::String(resolved_path))),
        );
        if !resolved_body.is_empty() || !query_params.is_empty() || !headers.is_empty() {
            body_data.extend(query_params);
            body_data.extend(resolved_body);
            body_data.extend(headers);
        }

        response_obj.insert(
            "body".to_owned(),
            Data::new(DataKind::Collection(Collection::Object(body_data))),
        );
        Ok(DataKind::Collection(Collection::Object(response_obj)))
    }
}
