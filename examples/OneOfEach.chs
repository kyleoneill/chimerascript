- case: oneOfEach
  steps:
    - ASSERT EQUALS 1 1
    - var foo = LITERAL 5
    - var bar = LITERAL "hello"
    - var baz = LITERAL false
    - PRINT "hello"
    - GET /test
