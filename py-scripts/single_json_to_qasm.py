"""
Converts a TKET JSON string to a QASM string using the
pytket.qasm.circuit_to_qasm converter.

Usage: python single_qasm_to_json.py <qasm string>
"""

import os
import sys
import json
from pytket import Circuit
from pytket.qasm import circuit_to_qasm_str

if len(sys.argv) != 2:
    print("Usage: python single_json_to_qasm.py <folder>")
    sys.exit(1)

json_str = sys.argv[1]

circuit = Circuit.from_dict(json.loads(json_str))
print(circuit_to_qasm_str(circuit))