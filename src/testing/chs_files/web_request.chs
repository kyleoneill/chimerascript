[test]
case get-request() {
  var res = GET /test_resource;
  ASSERT STATUS (res) 200;

  var res_with_query_param = GET /test_resource?first=1&second=2;
  ASSERT STATUS (res_with_query_param) 200;
  ASSERT EQUALS (res_with_query_param.body.extras.first) "1";
  ASSERT EQUALS (res_with_query_param.body.resource.has_values) true;

  case response-contains() {
    ASSERT CONTAINS (res_with_query_param.body) "extras";
    ASSERT CONTAINS (res_with_query_param.body) "resource";
    ASSERT NOT CONTAINS (res_with_query_param.body) "foobarbaz";
  }
}

[test]
case put-request() {
  var res = PUT /test_resource name="new_name";
  ASSERT STATUS (res) 200;
  ASSERT EQUALS (res.body.name) "new_name";

  case variable-in-endpoint() {
    var partial_request_name = LITERAL "resource";
    var response = PUT /test_(partial_request_name)?foo="bar"&baz="bash" name="new_name";
    ASSERT STATUS (response) 200;
  }
}

[test]
case delete-request() {
  var res = DELETE /test_resource;
  ASSERT STATUS (res) 200;
}

[test]
case post-resource() {
  var res = POST /test_resource name="dog" location="dogville" endpoints=42 has_values=true;
  ASSERT STATUS (res) 201;
}
