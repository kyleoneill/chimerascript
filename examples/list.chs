- case: simple-test
  steps:
    - var my_var = LITERAL 5
    - var my_other_var = LITERAL 10
    - var my_list = LIST NEW [1, 2, "hello world", (my_var), (my_other_var)]
