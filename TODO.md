- Need a config file
    - Needs to specify the web address being pointed at
    - Need to add config file info to README
- Add support to pass a directory of test files
- Variable data should be available across sections of a test
    - `setup` variables should be accessible in `steps` and `teardown`
    - `steps` variables should be accessible in `teardown`
    - I think this is done? But I think variables are _too_ permissive. Ex, a nested test can change
      a var in a parent test. A test can alter a var set in setup? Is this okay?
- Testing
  - frontend
  - Simple Python webserver vs test harness?
    - Web server already done and is simple, but using it would require a test-script that starts the
      server, runs the rust tests, and then reports results
    - Could try to make a test harness if it's possible to "intercept" web requests
      - Ex, test has a line to make a request to `http://localhost:5000/some_endpoint` with query and body
        params. The harness can intercept this request and just return what the web service is expected
        to return from the request to the test
  - Setup
  - Teardown
    - Teardown running when the test fails
  - Basic functionality
    - Nested tests
    - Variable assignment
      - Literal
      - HttpWeb
    - Variable access
      - Access variable subfield
      - Trying to access variable that does not exist
      - Trying to access an invalid subfield of a variable
      - Trying to access a list with a non int index
    - Negated assertion (ASSERT NOT)
    - PRINT
      - Both just a passed value and variables
    - Standalone expression
      - Literal (no-op?)
      - HttpWeb
- Ability to send Http requests to full paths so requests can go to endpoints
  other than just the one specified in config
- Support for comments
- Support for running a test by name
  - Accessed with args.name in main.rs
- Test tagging?
- Ability to create a list
  - `var my_list = LITERAL '[1,2,3]'`
  - `var my_list = LIST NEW '[1,2,3]'`
