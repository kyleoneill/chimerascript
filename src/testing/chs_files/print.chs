[test]
case print_case() {
  // Print a normal string
  PRINT "Hello world";

  // Print a variable
  var num = LITERAL 5;
  PRINT (num);

  // Test formatted strings
  var one = LITERAL "test";
  var two = LITERAL "test";

  // Test a formatted string using a single variable
  PRINT "test (one) test";

  // Test a formatted string using multiple variables
  PRINT "test (one) (two) test";
}
