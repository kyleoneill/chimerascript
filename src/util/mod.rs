use serde_json::Value;

pub fn serde_json_to_string(json_object: &Value) -> String {
    let mut str_build = String::new();
    match json_object {
        Value::Null => str_build.push_str("null"),
        Value::Bool(b) => {
            if *b {
                str_build.push_str("true")
            }
            else {
                str_build.push_str("false")
            }
        },
        Value::Number(n) => {
            match n.as_i64() {
                Some(i) => str_build.push_str(i.to_string().as_str()),
                // TODO: Need error handling here
                None => str_build.push_str(i64::MAX.to_string().as_str())
            }
        },
        Value::String(s) => str_build.push_str(s.as_str()),
        Value::Array(a) => {
            let mut b_string = "[".to_owned();
            for val in a {
                let formatted = format!("{}, ", serde_json_to_string(val));
                b_string.push_str(formatted.as_str());
            }
            // We want to remove the last two chars to get rid of the last ", "
            let slice: &str = &b_string[0..b_string.len() - 2];
            str_build.push_str(slice);
            str_build.push(']');
        },
        Value::Object(json_obj) => {
            for (key, val) in json_obj.iter() {
                let val_string = serde_json_to_string(val);
                let mut b_string = format!("{{\"{}}}\":\"{{{}}}\"", key, val_string);
                str_build.push_str(b_string.as_str());
            }
        }
    }
    str_build
}