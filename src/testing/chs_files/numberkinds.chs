- case: test-numberkinds
  steps:
    - var negative = LITERAL -20
    - var unsigned = LITERAL 15
    - var float = LITERAL 3.14159
    - var negative_float = LITERAL -20.052332

    - ASSERT EQUALS (float) 3.14159
    - ASSERT EQUALS (negative) -20
    - ASSERT EQUALS (unsigned) 15

    - ASSERT GT (unsigned) (negative)
    - ASSERT GT (unsigned) (float)
    - ASSERT GT (float) (negative)
    - ASSERT GT (float) (negative_float)

    - ASSERT LT (negative) (unsigned)
    - ASSERT LT (float) (unsigned)
    - ASSERT LT (negative) (float)
    - ASSERT LT (negative_float) (float)

- case: test-numberkinds-in-list
  steps:
    - var my_numbers = LIST NEW [1, 2.5, -10, -10.5]
    - ASSERT EQUALS (my_numbers.0) 1
    - ASSERT EQUALS (my_numbers.1) 2.5
    - ASSERT EQUALS (my_numbers.2) -10
    - ASSERT EQUALS (my_numbers.3) -10.5
