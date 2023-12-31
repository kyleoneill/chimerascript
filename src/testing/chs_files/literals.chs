[test]
case test-literal-values() {
  ASSERT EQUALS 1 1;
  ASSERT EQUALS true true;
  ASSERT EQUALS true True;
  ASSERT EQUALS "test" "test";
  ASSERT EQUALS "hello world" "hello world";
}

[test]
case test-literal-variables() {
  var num = LITERAL 5;
  var bigger_num = LITERAL 100;
  var empty = LITERAL null;
  var another_empty = LITERAL Null;
  var boolean = LITERAL true;
  var some_str = LITERAL "hello world";

  ASSERT EQUALS (empty) (another_empty);
  ASSERT NOT EQUALS (num) (boolean);
  ASSERT GTE (num) (num);
  ASSERT GTE (bigger_num) (num);
  ASSERT GT (bigger_num) (num);
  ASSERT LTE (num) (num);
  ASSERT LTE (num) (bigger_num);
  ASSERT LT (num) (bigger_num);
}
