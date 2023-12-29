- case: test-comments
  steps:
    - ASSERT EQUALS 1 1 // what a comment, i can say anything in here
    - ASSERT EQUALS 1 /* lemme interrupt you to say that I think asserting 1 == 1 is extremely overdone */ 1
