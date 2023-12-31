- Need a config file
    - Needs to specify the web address being pointed at
    - Need to add config file info to README
- Add support to pass a directory of test files
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
- Support for running a test by name
  - Accessed with args.name in main.rs
- Test tagging?
- Script documentation
- Finish implementing comments
  - Comments do not work if they take up an entire line
  - This is due to both the parsing spitting out 0 tokens and the
    YAML handling not returning an expected Array object
  - Might have to get rid of the YAML structure and make the entire file just
    the script to make this work? That is going to be a huge refactor,
    but doesn't need to affect most of the internals (everything including
    and after the AST)
- Add JSON support
  - Ex, `var foo = LITERAL JSON {"test":5};`
  - Allow this to be multiline
- Add ability for a Literal to be used as a request query/body param
  - Ex, `GET /foo (bar)` will use a Literal stored in `bar` as the body
    - Will need to assert here that `bar` is a Literal::Object?
    - Same sort of thing for a query param `GET /foo?(bar)`
- Variable scoping
  - I believe variables currently have no scope inside an outermost test where
    the variable hashmap is instantiated. Should implement scoping?
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
