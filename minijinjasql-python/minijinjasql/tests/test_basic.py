from minijinjasql_python import prepare_query


def test_basic():
    (res, params) = prepare_query("""
                  SELECT * FROM mytable
                  where letters in {{ hey | inclause }}
                  """, {"hey": ["a", "b", "c"]}, format_style="format")

    print(res)
    print(params)
