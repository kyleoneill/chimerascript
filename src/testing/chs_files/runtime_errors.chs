- case: non-existent-var
  steps:
    - ASSERT EQUALS 1 (foobar)

- case: bad-subfield-access
  steps:
    - var res_with_query_param = GET /test_resource?first=1&second=2
    - ASSERT EQUALS (res_with_query_param.body.test.thing.doesnt.exist) 5

- case: wrong-type
  steps:
    - ASSERT GT 5 "foo"
