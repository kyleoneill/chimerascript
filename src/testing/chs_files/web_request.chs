[test]
case get_request() {
    var res = GET /test_resource;
    ASSERT STATUS (res) 200;

    case response_contains() {
        ASSERT CONTAINS (res) "body";
        ASSERT NOT CONTAINS (res) "foobarbaz";
    }
}

[test]
case put_request() {
  var res = PUT /test_resource name="new_name";
  ASSERT STATUS (res) 200;
  ASSERT EQUALS (res.body.name) "new_name";

  case variable_in_endpoint() {
    var partial_request_name = LITERAL "resource";
    var response = PUT /test_(partial_request_name)?foo="bar"&baz="bash" name="new_name";
    ASSERT STATUS (response) 200;
  }
}

[test]
case delete_request() {
  var res = DELETE /test_resource;
  ASSERT STATUS (res) 200;
}

[test]
case post_resource() {
  var res = POST /test_resource name="dog" location="dogville" endpoints=42 has_values=true;
  ASSERT STATUS (res) 201;
}

[test]
case print_request() {
    var res = GET /test_resource;
    PRINT (res);
}

[test]
case query_params() {
    // Verify basic query params work
    var res_with_query_param = GET /test_resource?first=1&second=true;
    ASSERT STATUS (res_with_query_param) 200;
    ASSERT EQUALS (res_with_query_param.body.first) 1;
    ASSERT EQUALS (res_with_query_param.body.second) true;

    case query_param_variables() {
        // Verify query params with chars that must be escaped work
        // Verify query params with variables work
        var some_var = LITERAL "i am a value";
        var res = GET /test_resource?first="test with url replacement"&second=(some_var);
        ASSERT STATUS (res) 200;
        ASSERT EQUALS (res.body.first) "test with url replacement";
        ASSERT EQUALS (res.body.second) "i am a value";
    }
}
