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

[test]
case list-out-of-bounds() {
    var my_list = LIST NEW [1,2];
    ASSERT EQUALS (my_list.50) 50;
}

[test]
case index-non-number-subfield() {
    var my_list = LIST NEW [1,2];
    ASSERT EQUALS (my_list.idontexist) 6;
}

[test]
case index-non-number() {
    var my_list = LIST NEW [1,2];
    LIST REMOVE (my_list) "some_str";
}
