- case: failing-equals
  steps:
    - ASSERT EQUALS 1 2
    - case: nested-test-that-doesnt-run
      steps:
        - ASSERT EQUALS 1 1

- case: failing-gte
  steps:
    - ASSERT GTE 5 10

- case: failing-gt
  steps:
    - ASSERT GT 5 5

- case: failing-lte
  steps:
    - ASSERT LTE 10 5

- case: failing-lt
  steps:
    - ASSERT LT 5 5
