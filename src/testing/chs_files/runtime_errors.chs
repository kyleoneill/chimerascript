[test]
case non-existent-var() {
  ASSERT EQUALS 1 (foobar);
}

[test]
case bad-subfield-access() {
  var res_with_query_param = GET /test_resource?first=1&second=2;
  ASSERT EQUALS (res_with_query_param.body.test.thing.doesnt.exist) 5;
}

[test]
case wrong-type() {
  ASSERT GT 5 "foo";
}
