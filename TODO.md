- Need to add config file info to README
- Add support to pass a directory of test files
- Testing
  - Teardown
    - Teardown running when the test fails
- Ability to send Http requests to full paths so requests can go to endpoints
  other than just the one specified in config
- Support for running a test by name
  - Accessed with args.name in main.rs
- Script documentation
- Add JSON support
  - Ex, `var foo = LITERAL JSON {"test":5};`
  - Allow this to be multiline
- Variable scoping
  - I believe variables currently have no scope inside an outermost test where
    the variable hashmap is instantiated. Should implement scoping?
- Lexing?
  - Pest rule pairs contain metadata about the matched token, like the
    start and stop position in the string where it matched from. This is
    used during construction of the AST but is not used for runtime errors.
    It should be incorporated for runtime errors too so those errors can
    have more precise and helpful error messaging
- Refactor AST file
  - Break up large functions into more `parse_rule_to_x` functions
  - Rename variables that don't really describe the rule pairs correctly
  - Generally make it more readable
- Update README as progress moves
- Implement an actual CI pipeline
  - Make sure every time a commit is made to a PR that
    - `cargo fmt` was run
    - `cargo clippy` was run and there is nothing further that needs changing
    - `cargo test` was run and all tests passed
