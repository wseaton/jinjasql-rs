build-python-extention:
    cd minijinjasql-python && cargo build --release

setup-python: build-python-extention
    cd minijinjasql-python && poetry run python ../scripts/python-helper.py copy-extension
    
test-python +opts="": setup-python
    cd minijinjasql-python && poetry run pytest minijinjasql/tests -v -s {{opts}}

benchmark-python +opts="": setup-python
    cd minijinjasql-python && poetry run py.test minijinjasql/tests/benchmark.py -v -s {{opts}}