# ChimeraScript
ChimeraScript domain specific language used to write tests for HTTP
services. ChimeraScript makes it simple to quickly design tests that
can be run against a deployed web server.

Example
```yaml
- case: my-test
  steps:
    - res = GET /my_endpoint
    - ASSERT STATUS (res) 200 "Failed to make a GET to /my_endpoint" 
    - ASSERT EQUALS (res.body.my_field) 7 "Expected response to contain my_field set to 7" 
```

Tests can contain subtests
```yaml
- case: my-test
  steps:
    - GET /my_endpoint
    - case: my_subtest
      steps:
        - foo = PUT /foo my_field=7
        - ASSERT EQUALS (foo.body.my_field) 7 "Failed to update field" 
```

Tests can also contain setup and teardown sections. Setup will be performed
before the test and teardown will run after, even if the test fails.
```yaml
- case: my-test
  setup:
    - new_resource = POST /some_endpoint
  steps:
    - res = GET /my_endpoint
    - ASSERT STATUS (res) 200 "Failed to make a GET to /my_endpoint" 
    - ASSERT EQUALS (res.body.my_field) 7 "Expected response to contain my_field set to 7" 
  teardown:
    - DELETE /some_endpoint/(new_resource.body.id)
```
