[test]
case outermost-test() {
  ASSERT EQUALS 1 1;
  case middle-test() {
    ASSERT EQUALS 2 2;
    case innermost-test() {
      ASSERT EQUALS 3 3;
    }
  }
}
