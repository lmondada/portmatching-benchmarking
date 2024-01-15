"""
Converts a QASM string to a TKET JSON string using the
pytket.qasm.circuit_from_qasm converter.

Usage: python single_qasm_to_json.py <qasm string>
"""

import os
import sys
import json
from pytket.qasm import circuit_from_qasm_str

if len(sys.argv) != 2:
    print("Usage: python single_qasm_to_json.py <folder>")
    sys.exit(1)

qasm = sys.argv[1]

circuit = circuit_from_qasm_str(qasm)
print(json.dumps(circuit.to_dict()))