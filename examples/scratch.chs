- case: list-new
  steps:
    - var foo = LIST NEW []
    - PRINT (foo)
    - case: nested
      steps:
        - ASSERT EQUALS 2 2
