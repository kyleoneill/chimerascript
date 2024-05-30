[test]
case failing-equals() {
  ASSERT EQUALS 1 2;
  case nested-test-that-doesnt-run() {
    ASSERT EQUALS 1 1;
  }
}

[test]
case failing-gte() {
  ASSERT GTE 5 10;
}

[test]
case failing-gt() {
  ASSERT GT 5 5;
}

[test]
case failing-lte() {
  ASSERT LTE 10 5;
}

[test]
case failing-lt() {
  ASSERT LT 5 5;
}

[test]
case failing_error_message_string() {
    // Assertion with a string error message
    ASSERT EQUALS 1 2 "Custom error message";
}

[test]
case failing_error_message_formatted_string() {
    // Assertion with a formatted string error message
    var num = LITERAL 2;
    ASSERT EQUALS 1 2 "Expected 1 to equal (num)";
}

[test]
case assert_on_formatted_string() {
    var foo = LITERAL "hello";
    ASSERT EQUALS "foo" "thing '(foo)'";
}
