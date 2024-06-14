# ChimeraScript
ChimeraScript is a domain specific language designed to write tests for HTTP
services. ChimeraScript makes it simple to quickly design tests that
can be run against a deployed web server.

Tests allow you to make requests against an endpoint and then run
assertions against returned data.

```
[test]
case simple_test() {
 var res = GET /my_endpoint?page=7&fields=all;
 ASSERT STATUS (res) 200 "Failed to make a GET to /my_endpoint";
 ASSERT EQUALS (res.body.my_field) 7 "Expected response to contain my_field set to 7";
}
```

## Syntax
Functions take the structure of `case FUNCTION_NAME() { STATEMENTS }`. Statements inside a
function block are semicolon terminated.

###  Test Decorators
Functions can be marked with comma separated decorators above the function name.

#### \[test\]
The `[test]` decorator marks a function as a test, and causes it to be run automatically when a .chs file is run.

**Note**: Nested tests do not need the `[test]` decorator. If the parent of a function has the decorator, the
child function inherits it.

#### \[expected-failure\]
The `[expected-failure]` decorator marks a function as one that is expected to fail. This will cause a "failed"
test to register as an expected failure and a "successful" test to register as an unexpected success. Neither
of these will count towards a test failure.

```
[test, expected-failure]
case test_will_fail() {
  ASSERT EQUALS 1 2;
}
```

### Test Case Nesting

Tests can contain subtests
```
[test]
case outer_test() {
  var res = GET /my_endpoint;
  ASSERT STATUS (res) 200;
  
  case middle_test() {
    var foo = PUT /foo my_field=7;
    ASSERT EQUALS (foo.body.my_field) 7 "Failed to update field";
    
    case deeply_nested_test() {
      ASSERT EQUALS 1 1;
    }
  }
}
```
A nested test does not need a `[test]` decorator to be run automatically. If a `[test]` decorated
case contains a nested-case, the child function will inherit that decorator.

### String Formatting
There is not yet support for full string interpolation, but variables can be used inside strings. A variable can
be included in a string by putting the variable name in parentheses.

```
[test]
case string_formatting() {
    var planet = LITERAL "Earth";
    var continent_count = LITERAL 7;
    PRINT "Planet (planet) has (continent_count) continents";
    // ^ Prints "Planet Earth has 7 continents"
}
```

### HTTP Requests
HTTP requests take the format of `Method Path?QueryParams BodyParams Headers KeyValuePairs`

The following is an example of a PUT request with two query params, one body param, and a header:
```
PUT /some/endpoint?name="John"&id=42 new_address="some new data" authorization:"some_token";
```

`KeyValuePairs` represent configurable parameters which will be added at a later date. An example pair here
would be `timeout=>60` to indicate that the request should have a 60 second timeout.

Endpoints can contain variables:
```
var my_id = LITERAL "12345";
GET /users/by_id/(my_id);
```

## Collections
Data can be stored in type agnostic lists. List values can be accessed
by index and are 0-based. Lists can be appended to and values can be removed by index.

When a value is removed, all remaining values are shifted one position to
the left and the removed value is returned by the `REMOVE` statement.
When a value is appended, it is added to the end of the list.

```
[test]
case my-test() {
  var my_list = LIST NEW [200, 400];
  ASSERT EQUALS (my_list.0) 200;

  var removed_item = LIST REMOVE (my_list) 0;
  ASSERT EQUALS (removed_item) 200;
  ASSERT EQUALS (my_list.0) 400;

  var another_list = LIST NEW [10];
  LIST APPEND (another_list) (removed_item);
  LIST APPEND (another_list) "a string value";
  ASSERT EQUALS (another_list.2) "a string value";
}
```

There is support for a HashMap style object collection as well, but it can
currently only be used by accessing a field in a web response. Support for
creating an object will be added at a later date.

The below snippet asserts that the web response body contains a field with
the key "name".

```
[test]
case my-test() {
  var res = GET /some_endpoint;
  ASSERT CONTAINS (res.body) "name";
}
```

## Teardown

### NOTE: Teardown is still being implemented

Tests can contain teardown. Teardown allows for state to be cleaned up after a test is run,
even if the test fails. When a test is run, an empty teardown stack is allocated for that test.
When execution of a test finishes, or if the test errors or fails, the teardown stack is processed.
```
[test]
case my-test() {
  // Here we create some new resource on our web service
  var my_new_resource = POST /new_resource;
  
  // Next we add a statement to the teardown stack so the new resource is cleaned
  // up when our test ends, even if it terminates early due to an error or failure
  teardown {
    DELETE /new_resource/(my_new_resource.body.id);
  }
  
  // This assertion causes the test to fail and the teardown stack to run
  ASSERT EQUALS 1 2;
}
```

## Comments

Single line comments begin with a `//` to indicate that the rest of the following line
is a comment. Comments can also begin with `/*` and be closed with a `*/` to begin and
end a comment mid-line, or to make a multi-line comment.

```
[test]
case my-test() {
  // This is a single line comment
  ASSERT EQUALS 1 1; // This is a comment in an assertion
  ASSERT EQUALS 1 /* This comment is in the middle */ 1;
  /*
     This comment spans multiple lines
  */
  ASSERT EQUALS 1 1;
}
```
