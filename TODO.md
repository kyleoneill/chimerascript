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
    - Variable access
      - Trying to access a list with a non int index
    - Standalone expression
      - Literal (no-op?)
      - HttpWeb
- Ability to send Http requests to full paths so requests can go to endpoints
  other than just the one specified in config
- Support for comments
- Support for running a test by name
  - Accessed with args.name in main.rs
- Test tagging?
- Script documentation

----

- Lexing?
  - Pest rule pairs contain metadata about the matched token, like the
    start and stop position in the string where it matched from. Should
    this information be stored? Is there error-handling/debugging use for it?
- Refactor AST file
  - Break up large functions into more `parse_rule_to_x` functions
  - Rename variables that don't really describe the rule pairs correctly
  - Generally make it more readable
- Update README as progress moves
