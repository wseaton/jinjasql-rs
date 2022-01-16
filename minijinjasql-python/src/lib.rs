#![feature(generic_associated_types)]
#![allow(incomplete_features)]

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyTuple};
use pyo3::wrap_pyfunction;
use pyo3::{FromPyObject, ToPyObject};

use minijinjasql::JinjaSqlBuilder;

// use hashbrown::HashMap;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;

use minijinja::value::Value;
use minijinja::{context, Source};

#[pymodule]
fn minijinjasql_python(_: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(prepare_query, m)?).unwrap();

    Ok(())
}

#[pyfunction]
#[pyo3(text_signature = "(query, data, format_style)")]
pub fn prepare_query<'a>(
    py: Python<'a>,
    query: &str,
    data: HashMap<String, Vec<String>>,
    format_style: Option<&str>
) -> PyResult<(String, Vec<String>)> {
    
    let mut j = JinjaSqlBuilder::new();

    if let Some(ps) = format_style {
        j = j.set_param_style(ps);    
    } 

    let bj = j.build();
    
    let ctx = Value::from_serializable(&data);

    let (res, params) = bj.render_query(Some(query), None, ctx).unwrap();

    Ok((res, params))
}
