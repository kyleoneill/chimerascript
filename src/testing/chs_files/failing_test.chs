[test]
case failing-equals() {
  ASSERT EQUALS 1 2;
  case nested-test-that-doesnt-run() {
    ASSERT EQUALS 1 1;
  }
}

[test]
case failing-gte() {
  ASSERT GTE 5 10;
}

[test]
case failing-gt() {
  ASSERT GT 5 5;
}

[test]
case failing-lte() {
  ASSERT LTE 10 5;
}

[test]
case failing-lt() {
  ASSERT LT 5 5;
}
