[test, expected-failure]
case test-expect-failure() {
  ASSERT EQUALS 1 2;
}

[test]
case test-not-equals() {
  ASSERT NOT EQUALS 1 2;
}

[test, expected-failure]
case test-expected-failure-pass() {
    ASSERT EQUALS 1 1;
}
