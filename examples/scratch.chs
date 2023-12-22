- case: list-new
  steps:
    - var my_var = LITERAL 5
    - var other_var = LITERAL 10
    - var my_list = LIST NEW [1, 2, "hello world", (my_var), (other_var)]
    - case: list-access
      steps:
        - ASSERT EQUALS (my_list.0) 1
