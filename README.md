# ChimeraScript
ChimeraScript is a domain specific language used to write tests for HTTP
services. ChimeraScript makes it simple to quickly design tests that
can be run against a deployed web server.

Tests allow you to make requests against an endpoint and then run
assertions against returned data.

```yaml
- case: my-test
  steps:
    - var res = GET /my_endpoint?page=7&fields=all
    - ASSERT STATUS (res) 200 "Failed to make a GET to /my_endpoint" 
    - ASSERT EQUALS (res.body.my_field) 7 "Expected response to contain my_field set to 7" 
```

Tests can contain subtests
```yaml
- case: my-test
  steps:
    - GET /my_endpoint
    - case: my-subtest
      steps:
        - var foo = PUT /foo my_field=7
        - ASSERT EQUALS (foo.body.my_field) 7 "Failed to update field" 
        - case: deeply-nested-subtest
          steps:
            - ASSERT EQUALS 1 1
```

Data can be stored in lists. Lists also allow for the storage of data with
different types. List values can be accessed by index and are 0-based.
Lists can be appended to and values can be removed by index. When a value is
removed, all remaining values are shifting one position to the left and
the removed value is returned by the `REMOVE` statement.

```yaml
- case: my-test
  steps:
    - var my_list = LIST NEW [200, 400]
    - var res = GET /some_endpoint
    - ASSERT EQUALS (res.status_code) (my_list.0)

    - LIST APPEND (my_list) (res.body.number_of_things)
    - LIST APPEND (my_list) "a string value"

    - var removed_item = LIST REMOVE (my_list) 0
    - ASSERT EQUALS (removed_item) 200
    - ASSERT EQUALS (my_list.0) 400
```

Tests can also contain setup and teardown sections. Setup will be performed
before the test and teardown will run after, even if the test fails.
```yaml
- case: my-test
  setup:
    - var new_resource = POST /some_endpoint name=foo desc=bar
  steps:
    - var res = GET /my_endpoint
    - ASSERT STATUS (res) 200 "Failed to make a GET to /my_endpoint" 
    - ASSERT EQUALS (res.body.my_field) 7 "Expected response to contain my_field set to 7" 
  teardown:
    - DELETE /some_endpoint/(new_resource.body.id)
```
