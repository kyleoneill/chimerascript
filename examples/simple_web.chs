- case: simple-web-test
  steps:
    - var web_res = GET /test_resource
    - ASSERT EQUALS (web_res.status_code) 200
