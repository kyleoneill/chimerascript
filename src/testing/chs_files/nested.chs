- case: outermost-test
  steps:
    - ASSERT EQUALS 1 1
    - case: middle-test
      steps:
        - ASSERT EQUALS 2 2
        - case: innermost-test
          steps:
            - ASSERT EQUALS 3 3
