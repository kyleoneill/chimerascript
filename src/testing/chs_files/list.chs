[test]
case list-new() {
  var my_var = LITERAL 5;
  var other_var = LITERAL 10;
  var my_list = LIST NEW [1, 2, "hello world", (my_var), (other_var)];
  case list-length() {
    var list_length = LIST LENGTH (my_list);
    ASSERT EQUALS (list_length) 5;
  }
  case list-new-empty() {
    var empty_list = LIST NEW [];
    var empty_list_len = LIST LENGTH (empty_list);
    ASSERT EQUALS (empty_list_len) 0;
  }
  case print-list() {
    PRINT (my_list);
  }
  case list-access() {
    ASSERT EQUALS (my_list.0) 1;
    ASSERT EQUALS (my_list.2) "hello world";
    ASSERT EQUALS (my_list.3) 5;
    ASSERT EQUALS (my_list.4) (other_var);
  }
  case list-append() {
    LIST APPEND (my_list) 10;
    ASSERT EQUALS (my_list.5) 10;
    var new_list_len = LIST LENGTH (my_list);
    ASSERT EQUALS (new_list_len) 6;
  }
  case list-remove() {
    var first_len = LIST LENGTH (my_list);
    var removed = LIST REMOVE (my_list) 0;
    ASSERT EQUALS (removed) 1;
    var new_len = LIST LENGTH (my_list);
    ASSERT NOT EQUALS (first_len) (new_len);
  }
  case list-assert-length() {
    var list_len = LIST LENGTH (my_list);
    ASSERT LENGTH (my_list) (list_len);
  }
  case list-contains-pass() {
    ASSERT CONTAINS (my_list) 2;
    ASSERT CONTAINS (my_list) "hello world";
    var two = LITERAL 2;
    ASSERT CONTAINS (my_list) (two);
  }
  case list-pop() {
    LIST APPEND (my_list) 10;
    LIST APPEND (my_list) 20;
    var first_len = LIST LENGTH (my_list);
    LIST POP (my_list);
    var popped_val = LIST POP (my_list);
    ASSERT EQUALS (popped_val) 10;
    var new_len = LIST LENGTH (my_list);
    ASSERT NOT EQUALS (first_len) (new_len);
  }
  case list-equality() {
    var first_list = LIST NEW [1,2,3];
    var second_list = LIST NEW [1,2,3];
    ASSERT EQUALS (first_list) (second_list);
    var third_list = LIST NEW [4,5,6];
    var fourth_list = LIST NEW ["test"];
    ASSERT NOT EQUALS (third_list) (first_list);
    ASSERT NOT EQUALS (fourth_list) (first_list);
  }
}

[test]
case list-bad-remove-index() {
  var my_list = LIST NEW [6];
  LIST REMOVE (my_list) 100;
}

[test]
case list-doesnt-exist() {
  LIST APPEND (i_dont_exist) 10;
}

[test]
case list-assert-length-to-non-list() {
  var my_num = LITERAL 5;
  ASSERT LENGTH (my_num) 5;
}

[test]
case list-contains-fail() {
  var foo = LITERAL 5;
  ASSERT CONTAINS (foo) 10;
}

[test]
case list-pop-fail() {
  var empty_list = LIST NEW [];
  LIST POP (empty_list);
}
