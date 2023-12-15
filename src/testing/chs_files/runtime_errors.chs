- case: non-existent-var
  steps:
    - ASSERT EQUALS 1 (foobar)

- case: bad-subfield-access
  steps:
    - var res = GET /test_resource
    - ASSERT EQUALS (res.foo) 1

- case: wrong-type
  steps:
    - ASSERT GT 5 "foo"
