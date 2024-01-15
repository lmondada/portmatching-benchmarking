#include <filesystem>
#include <iostream>
#include "quartz/context/context.h"
#include "quartz/gate/gate_utils.h"
#include "quartz/parser/qasm_parser.h"
#include "quartz/tasograph/substitution.h"
#include "quartz/tasograph/tasograph.h"

using namespace quartz;

// Context CTX({GateType::input_qubit, GateType::input_param, GateType::h,
//                GateType::cx, GateType::x, GateType::rz, GateType::add});
Context voqc(voqc_gate_set());
Context input_CTX({GateType::input_qubit, GateType::input_param});
Context CTX = union_contexts(&voqc, &input_CTX);

// std::vector<std::string> get_sorted_qasm_files(const std::string &folder) {
//   std::vector<std::filesystem::directory_entry> qasm_files;
//   for (const auto &entry : std::filesystem::directory_iterator(folder)) {
//     if (entry.is_regular_file() && entry.path().extension() == ".qasm") {
//       qasm_files.push_back(entry);
//     }
//   }
//   std::sort(qasm_files.begin(), qasm_files.end(),
//             [](const std::filesystem::directory_entry &a,
//                const std::filesystem::directory_entry &b) {
//               return std::stoi(a.path().filename().stem()) <
//                      std::stoi(b.path().filename().stem());
//             });
//   std::vector<std::string> qasm_file_names;
//   for (const auto &entry : qasm_files) {
//     qasm_file_names.push_back(entry.path());
//   }
//   return qasm_file_names;
// }

Graph *load_graph(const char *qasm_str) {
  auto graph = Graph::from_qasm_str(&CTX, std::string(qasm_str));
  return new Graph(*graph);
}

void free_graph(const quartz::Graph *graph) { delete graph; }

quartz::GraphXfer **load_xfers(const char* const* qasm_str_array,
                               unsigned n_xfers) {
  auto circs = std::vector<CircuitSeq *>();
  for (unsigned i = 0; i < n_xfers; ++i) {
    std::string qasm_str(qasm_str_array[i]);
    auto circ = CircuitSeq::from_qasm_style_string(&CTX, qasm_str).release();
    assert(circ != nullptr);
    circs.push_back(circ);
  }
  auto xfers = new GraphXfer *[n_xfers];
  for (unsigned i = 0; i < n_xfers; ++i) {
    auto circ = circs[i];
    auto empty_circ = new CircuitSeq(circ->get_num_qubits(),
                                     circ->get_num_input_parameters());
    auto xfer = GraphXfer::create_GraphXfer(&CTX, circ, empty_circ, true);
    assert(xfer != nullptr);
    xfers[i] = xfer;
    // free up memory
    delete empty_circ;
    delete circ;
    circ = nullptr;
  }
  return xfers;
}

void free_xfers(quartz::GraphXfer **xfers, const unsigned n_xfers) {
  for (unsigned i = 0; i < n_xfers; ++i) {
    delete xfers[i];
  }
  delete[] xfers;
}

quartz::Op *get_ops(const quartz::Graph *const graph, unsigned &n_ops) {
  auto all_ops = std::vector<Op>();
  graph->topology_order_ops(all_ops);
  assert(all_ops.size() == (size_t)graph->gate_count());
  n_ops = all_ops.size();

  auto ops_c = new quartz::Op[all_ops.size()];
  std::memcpy(ops_c, all_ops.data(), all_ops.size() * sizeof(quartz::Op));
  return ops_c;
}
void free_ops(quartz::Op *ops) { delete[] ops; }

unsigned pattern_match(const quartz::Graph *const graph,
                       const quartz::Op *const ops, const unsigned n_ops,
                       quartz::GraphXfer *const *const xfers,
                       const unsigned n_xfers) {
  auto cnt = 0;
  for (unsigned i = 0; i < n_ops; ++i) {
    auto &op = ops[i];
    for (unsigned j = 0; j < n_xfers; ++j) {
      auto xfer = xfers[j];
      // pattern matching + convexity test
      cnt += graph->xfer_appliable(xfer, op);
    }
  }
  return cnt;
}

template <class Tp>
inline void black_box(Tp const& value) {
  asm volatile("" : : "r,m"(value) : "memory");
}

void ecc_to_qasm(char const* ecc_file, char const* out_folder) {
  EquivalenceSet eqs;
  // Load equivalent dags from file
  if (!eqs.load_json(&CTX, ecc_file)) {
    std::cout << "Failed to load equivalence file." << std::endl;
    assert(false);
  }

  // Get xfer from the equivalent set
  auto ecc = eqs.get_all_equivalence_sets();
  std::vector<GraphXfer *> xfers;
  unsigned i = 0;
  for (auto eqcs : ecc) {
    for (auto circ : eqcs) {
      circ->to_qasm_file(&CTX, std::string(out_folder) + '/' + std::to_string(i++) + ".qasm");
    }
  }
}