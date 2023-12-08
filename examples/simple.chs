- case: simple-test
  steps:
    - ASSERT EQUALS 1 1
    - var foo = LITERAL 5
    - PRINT (foo)
    - ASSERT EQUALS (foo) 5
    - ASSERT EQUALS 5 (foo)
    - ASSERT GT 10 (foo)
