- case: oneOfEach
  steps:
    - GET /test/thing?field="val"&another="bla" thing=5 somekey=>value
    - ASSERT EQUALS 1 1
    - ASSERT NOT EQUALS 1 2 "msg"
    - var foo = LITERAL 5
    - var bar = LITERAL "hello"
    - var baz = LITERAL false
    - PRINT "hello"
    - PRINT (foo)
