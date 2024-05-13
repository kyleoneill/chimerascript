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

[test]
case headers() {
    // Verify a basic request with a header works
    var header_res = GET /test_resource authorization:"foo";
    ASSERT STATUS (header_res) 200;
    ASSERT CONTAINS (header_res.body) "authorization";
    ASSERT EQUALS (header_res.body.authorization) "foo";

    // Verify a request with a header set to a variable works
    var foo = LITERAL 5;
    var header_with_var = GET /test_resource authorization:(foo);
    ASSERT STATUS (header_with_var) 200;
    ASSERT CONTAINS (header_with_var.body) "authorization";
    ASSERT EQUALS (header_with_var.body.authorization) 5;

    // Verify a request with a custom header in the expected format works
    var header_with_var = GET /test_resource foo:5;
    ASSERT STATUS (header_with_var) 200;
    ASSERT CONTAINS (header_with_var.body) "foo";
    ASSERT EQUALS (header_with_var.body.foo) 5;
}

[test]
case path_variables() {
    var some_var = LITERAL "test";
    var id = LITERAL 50;
    var res = GET /endpoint_(some_var)/(id);
    ASSERT CONTAINS (res.body) "path";
    ASSERT EQUALS (res.body.path) "http://127.0.0.1:5000/endpoint_test/50";
}
