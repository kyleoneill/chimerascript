- case: test-expect-failure
  expected-failure: true
  steps:
    - ASSERT EQUALS 1 2

- case: test-not-equals
  steps:
    - ASSERT NOT EQUALS 1 2

- case: test-expected-failure-pass
  expected-failure: true
  steps:
    - ASSERT EQUALS 1 1
