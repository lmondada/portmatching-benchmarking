// use indoc::indoc;
// use lazy_static::lazy_static;
// use pyo3::prelude::*;

// use super::pyo3_black_magic::init_pyo3_with_venv;

// fn py_fn() -> Py<PyAny> {
//     pyo3::prepare_freethreaded_python();

//     Python::with_gil(|py| {
//         PyModule::from_code(
//             py,
//             indoc! {"
//             import json
//             from pytket.qasm import circuit_from_qasm_str
//             def qasm_to_json(qasm):
//               circuit = circuit_from_qasm_str(qasm)
//               return json.dumps(circuit.to_dict())
//             "},
//             "",
//             "",
//         )
//         .unwrap()
//         .getattr("qasm_to_json")
//         .unwrap()
//         .into()
//     })
// }

// lazy_static! {
//     static ref PY_FN: Py<PyAny> = py_fn();
// }

use std::{io, process::Command};

/// Converts a QASM string to JSON string using tket1.
///
/// It would be much faster if we could use pyo3 bindings.
pub(crate) fn qasm_to_json(qasm: &str) -> io::Result<String> {
    Command::new("python")
        .arg("py-scripts/single_qasm_to_json.py")
        .arg(qasm)
        .output()
        .and_then(|output| {
            String::from_utf8(output.stdout).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        })
}

/// Converts a TKET JSON string to a QASM string.
///
/// It would be much faster if we could use pyo3 bindings.
pub(crate) fn json_to_qasm(json: &str) -> io::Result<String> {
    Command::new("python")
        .arg("py-scripts/single_json_to_qasm.py")
        .arg(json)
        .output()
        .and_then(|output| {
            String::from_utf8(output.stdout).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        })
}
