[test]
case formatted_strings() {
    // Test a basic formatted string
    var some_val = LITERAL "some str";
    var formatted_string = FORMAT_STR "tacking on (some_val)";
    ASSERT EQUALS (formatted_string) "tacking on some str";

    // Test a formatted string built with a non-string variable
    var numerical_val = LITERAL 5;
    var another_fmt_str = FORMAT_STR "5 is (numerical_val)";
    ASSERT EQUALS (another_fmt_str) "5 is 5";

    // Test using a formatted string in an error message
    var foo = LITERAL 1;
    ASSERT EQUALS 1 (foo) "Expected 1 to equal (foo)";

    // Print a formatted string
    PRINT (formatted_string);
}

[test]
case special_characters() {
    var some_str = LITERAL "asdf 123 !@#$%^&*,.?[]{}";
    ASSERT EQUALS (some_str) "asdf 123 !@#$%^&*,.?[]{}";
}
