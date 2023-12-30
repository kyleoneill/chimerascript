- case: list-new
  steps:
    - var foo = LIST NEW []
    - PRINT (foo)

YAML-like but with curly blocks and semicolon lines
gives me what I need but it's ugly and mixes being whitespace delimited and ; delimited
-----------------------------------------------
- case: test
  expected-failure: true
  setup: {
    ASSERT EQUALS 1 1;
    var something = LITERAL 5;
  }
  steps : {
    ASSERT EQUALS (something) 5;
    PRINT "Hello!";
  }
  teardown: {
    DELETE /some/endpoint;
  }

  function-like
  how does sub-casing and setup/teardown work here?
  -----------------------------------------------
  case test() {
    ASSERT EQUALS 1 1;
    var foo = LITERAL "Hello world!";
  }

  case another_one(expected_failure: true) {
    ASSERT EQUALS 1 2;
  }

function-like-extended
-----------------------------------------------
#[expected-failure:true]
case test() {
  ASSERT EQUALS 1 1;
  var foo = LITERAL 10;
}
