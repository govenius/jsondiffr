//! Python bindings for the `jsondiffr` patcher.
//!
//! Exposes:
//! * `JsonValue` — immutable wrapper around `jsondiffr::JsonValue` with
//!   `from_json(str)` and `to_python(path=None)` methods.
//! * `patch(source, diff)` — apply a marshalled-format diff (str) to a
//!   source (JsonValue or str), returning a new `JsonValue`.
//! * `diff(source, target)` — compute a diff between a source
//!   (JsonValue) and a target (JsonValue or str).
//! * `ParseError`, `PatchError` — module-level exceptions.

use pyo3::exceptions::{PyIndexError, PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict, PyList, PyTuple};
use pyo3::Bound;

use ::jsondiffr as core;

pyo3::create_exception!(jsondiffr, ParseError, pyo3::exceptions::PyValueError);
pyo3::create_exception!(jsondiffr, PatchError, pyo3::exceptions::PyException);

// ── JsonValue wrapper ──────────────────────────────────────────────────────

#[pyclass(name = "JsonValue", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct JsonValue {
    inner: core::JsonValue,
}

#[pymethods]
impl JsonValue {
    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Self> {
        let v: serde_json::Value =
            serde_json::from_str(text).map_err(|e| ParseError::new_err(e.to_string()))?;
        Ok(JsonValue {
            inner: core::parse_source(v),
        })
    }

    /// Materialize as native Python data, optionally descending along `path`
    /// (a tuple of str keys and/or int indices).
    ///
    /// `path=None` and `path=()` both return the entire value.
    #[pyo3(signature = (path = None))]
    fn to_python<'py>(
        &self,
        py: Python<'py>,
        path: Option<Bound<'py, PyTuple>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let target = match path.as_ref() {
            Some(t) => descend(&self.inner, t)?,
            None => &self.inner,
        };
        json_value_to_py(py, target)
    }

    fn __eq__(&self, other: &JsonValue) -> bool {
        self.inner == other.inner
    }

    fn __repr__(&self) -> String {
        format!("JsonValue({:?})", self.inner)
    }
}

// ── Path descent ───────────────────────────────────────────────────────────

fn descend<'a>(
    root: &'a core::JsonValue,
    path: &Bound<'_, PyTuple>,
) -> PyResult<&'a core::JsonValue> {
    let mut cur = root;
    for (depth, component) in path.iter().enumerate() {
        cur = match cur {
            core::JsonValue::Object(m) => {
                let key: String = component.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "path[{depth}]: expected str key into object"
                    ))
                })?;
                m.get(key.as_str()).ok_or_else(|| {
                    PyKeyError::new_err(format!("path[{depth}]: missing key {key:?}"))
                })?
            }
            core::JsonValue::Array(a) => {
                let idx: usize = component.extract().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "path[{depth}]: expected non-negative int index into array"
                    ))
                })?;
                a.get(idx).ok_or_else(|| {
                    PyIndexError::new_err(format!(
                        "path[{depth}]: index {idx} out of range for array of length {}",
                        a.len()
                    ))
                })?
            }
            _ => {
                return Err(PyTypeError::new_err(format!(
                    "path[{depth}]: cannot descend into a scalar value"
                )));
            }
        };
    }
    Ok(cur)
}

// ── JsonValue → Python conversion ─────────────────────────────────────────

fn json_value_to_py<'py>(
    py: Python<'py>,
    v: &core::JsonValue,
) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::types::PyString;
    Ok(match v {
        core::JsonValue::Null => py.None().into_bound(py),
        core::JsonValue::Bool(b) => pyo3::types::PyBool::new(py, *b).to_owned().into_any(),
        core::JsonValue::Number(n) => {
            if let Some(i) = n.0.as_i64() {
                i.into_pyobject(py)?.into_any()
            } else if let Some(u) = n.0.as_u64() {
                u.into_pyobject(py)?.into_any()
            } else {
                let f = n.0.as_f64().expect("serde_json::Number always fits one of i64/u64/f64");
                f.into_pyobject(py)?.into_any()
            }
        }
        core::JsonValue::String(s) => PyString::new(py, s.as_ref()).into_any(),
        core::JsonValue::Array(a) => {
            let list = PyList::empty(py);
            for x in a.iter() {
                list.append(json_value_to_py(py, x)?)?;
            }
            list.into_any()
        }
        core::JsonValue::Object(m) => {
            let dict = PyDict::new(py);
            for (k, val) in m.iter() {
                dict.set_item(k.as_ref(), json_value_to_py(py, val)?)?;
            }
            dict.into_any()
        }
    })
}

// ── patch() ────────────────────────────────────────────────────────────────

#[pyfunction]
fn patch(source: &Bound<'_, PyAny>, diff: &str) -> PyResult<JsonValue> {
    let src = extract_source(source)?;
    let diff_value: serde_json::Value =
        serde_json::from_str(diff).map_err(|e| ParseError::new_err(e.to_string()))?;
    let parsed_diff =
        core::parse_diff(diff_value).map_err(|e| ParseError::new_err(e.to_string()))?;
    let out = core::patch(&src, &parsed_diff).map_err(|e| PatchError::new_err(e.to_string()))?;
    Ok(JsonValue { inner: out })
}

fn extract_source(source: &Bound<'_, PyAny>) -> PyResult<core::JsonValue> {
    if let Ok(jv) = source.extract::<PyRef<JsonValue>>() {
        Ok(jv.inner.clone())
    } else if let Ok(s) = source.extract::<String>() {
        let v: serde_json::Value =
            serde_json::from_str(&s).map_err(|e| ParseError::new_err(e.to_string()))?;
        Ok(core::parse_source(v))
    } else {
        Err(PyTypeError::new_err("source must be JsonValue or str"))
    }
}

// ── diff() ─────────────────────────────────────────────────────────────────

#[pyfunction]
fn diff(source: &Bound<'_, PyAny>, target: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    let src = extract_source(source)?;
    let target_val = extract_source(target)?;
    
    let diff_val = core::diff(&src, &target_val);
    
    // To return a Python-friendly dict/list, we convert the DiffValue back to a 
    // serde_json::Value. Since DiffValue is an internal AST, we'll use a 
    // helper to convert it to a JSON-compatible structure.
    Ok(diff_value_to_serde(diff_val))
}

/// Helper to convert the internal DiffValue AST into a serde_json::Value
/// that represents the marshalled format (using $sentinels).
fn diff_value_to_serde(dv: core::DiffValue) -> serde_json::Value {
    match dv {
        core::DiffValue::Empty => serde_json::json!({}),
        core::DiffValue::Replace(v) => serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
        core::DiffValue::Patch(p) => {
            let mut map = serde_json::Map::new();
            
            if let Some(replace) = p.replace {
                map.insert(serde_json::json!("$replace"), serde_json::to_value(replace).unwrap_or(serde_json::Value::Null));
            }
            if let Some(delete) = p.delete {
                map.insert(serde_json::json!("$delete"), serde_json::to_value(delete).unwrap_or(serde_json::Value::Null));
            }
            if let Some(insert) = p.insert {
                // Convert Vec<(usize, JsonValue)> to the marshalled format: [[pos, val], ...]
                let insert_list: Vec<serde_json::Value> = insert.into_iter()
                    .map(|(pos, val)| {
                        serde_json::json!([pos, serde_json::to_value(val).unwrap_or(serde_json::Value::Null)])
                    })
                    .collect();
                map.insert(serde_json::json!("$insert"), serde_json::Value::Array(insert_list));
            }
            // Add/Discard are not implemented in Rust yet
            
            for (k, sub_dv) in p.entries {
                map.insert(serde_json::Value::String(k.to_string()), diff_value_to_serde(sub_dv));
            }
            
            serde_json::Value::Object(map)
        }
    }
}

// ── Module registration ────────────────────────────────────────────────────

#[pymodule]
fn jsondiffr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add_class::<JsonValue>()?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    m.add("ParseError", py.get_type::<ParseError>())?;
    m.add("PatchError", py.get_type::<PatchError>())?;
    Ok(())
}
