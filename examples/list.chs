- case: simple-test
  steps:
    - var my_var = LITERAL 5
    - var my_other_var = LITERAL 10
    - var my_list = LIST NEW [1, 2, "hello world", (my_var), (my_other_var)]
    - PRINT (my_list)
    - LIST APPEND (my_list) 5
    - var len = LIST LENGTH (my_list)
    - ASSERT EQUALS (my_list.0) 1
    - PRINT (len)
