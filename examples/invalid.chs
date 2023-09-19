- case: my-test
  setup:
    - POST /some/url name=bla desc=blabla timeout=>60
    - GET /some/url/bla?desc=blabla timeout=>60
  steps:
    - foo = PUT /some/other/url some_body_field=6 timeout=>30
    - ASSERT STATUS (foo) 200 "foo did not succeed."
    - ASSERT EQUALS (foo.body.field) 6 "Field was not updated correctly, it was (foo.body.field)"
    case: my-subtest
      steps:
        - bar = GET /some/other/url
        - ASSERT EQUALS (bar.field) 70 "failed to do something"
  teardown:
    - DELETE /some/url/bla
- case: another-one
  steps:
    - PRINT "I'm just another test"
