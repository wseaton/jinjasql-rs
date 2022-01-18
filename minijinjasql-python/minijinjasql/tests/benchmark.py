from minijinjasql_python import prepare_query
from jinjasql import JinjaSql


TEST_SQL = """
SELECT * FROM mytable
where letters in {{ hey | inclause }}
"""

TEST_DATA = {"hey": ["a", "b", "c"] * 100}

def bench_minijinjasql():
    return prepare_query(TEST_SQL, TEST_DATA)



def bench_jinjasql():
    j = JinjaSql()
    
    return j.prepare_query(TEST_SQL, TEST_DATA)


def test_minijinjasql(benchmark):
    benchmark(bench_minijinjasql)
    
    

def test_jinjasql(benchmark):
    benchmark(bench_jinjasql)