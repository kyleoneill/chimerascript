- case: list-new
  steps:
    - var my_var = LITERAL 5
    - var other_var = LITERAL 10
    - var my_list = LIST NEW [1, 2, "hello world", (my_var), (other_var)]
    - case: print-list
      steps:
        - PRINT (my_list)
    - case: list-access
      steps:
        - ASSERT EQUALS (my_list.0) 1
        - ASSERT EQUALS (my_list.2) "hello world"
        - ASSERT EQUALS (my_list.3) 5
        - ASSERT EQUALS (my_list.4) (other_var)
    - case: list-length
      steps:
        - var list_length = LIST LENGTH (my_list)
        - ASSERT EQUALS (list_length) 5
    - case: list-append
      steps:
        - LIST APPEND (my_list) 10
        - var new_list_len = LIST LENGTH (my_list)
        - ASSERT EQUALS (new_list_len) 6
    - case: list-remove
      steps:
        - var first_len = LIST LENGTH (my_list)
        - var removed = LIST REMOVE (my_list) 0
        - ASSERT EQUALS (removed) 1
        - var new_len = LIST LENGTH (my_list)
        - ASSERT NOT EQUALS (first_len) (new_len)
    - case: list-assert-length
      steps:
        - var list_len = LIST LENGTH (my_list)
        - ASSERT LENGTH (my_list) (list_len)
    - case: list-contains-pass
      steps:
        - ASSERT CONTAINS (my_list) 2
        - ASSERT CONTAINS (my_list) "hello world"
        - var two = LITERAL 2
        - ASSERT CONTAINS (my_list) (two)
- case: list-bad-remove-index
  steps:
    - var my_list = LIST NEW [6]
    - LIST REMOVE (my_list) 100
- case: list-bad-access
  steps:
    - LIST APPEND (i_dont_exist) 10
- case: list-assert-length-to-non-list
  steps:
    - var my_num = LITERAL 5
    - ASSERT LENGTH (my_num) 5
- case: list-contains-fail
  steps:
    - var foo = LITERAL 5
    - ASSERT CONTAINS (foo) 10
