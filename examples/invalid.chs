- case: variable-in-endpoint
  steps:
    - var partial_request_name = LITERAL "resource"
    - var response = PUT /test_(partial_request_name)?foo="bar"&baz="bash" name="new_name"
    - ASSERT STATUS (response) 200
