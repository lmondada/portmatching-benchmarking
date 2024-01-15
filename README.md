# Benchmarks of the portmatching

portmatching: https://github.com/lmondada/portmatching

## Reproduce the benchmarking
### Dependencies and setup
1. Compile the `quartz_runtime` library of the Quartz project (https://github.com/quantum-compiler/quartz) and copy the resulting library file to a new `lib` folder.
2. Compile the minimal quartz bindings used in this project using
   `
   clang++ -O3 -shared -fPIC -o lib/libquartz_bindings.dylib --std=c++17 quartz_bindings/bindings.cpp -I../quartz/src -Llib -lquartz_runtime -rpath @loader_path
   `
   where QUARTZ_REPO is the path to the root of the quartz git repo on your machine.
   This was run on MacOS 14.2. Note that the command (specifically regarding rpath) might change on other OSes.
3. Setup and activate a python environment with `pytket` and `seaborn` installed. This will be used
   to convert `qasm` datasets to a TKET `json` format. If the datasets are large,
   consider using the `qasm_to_json` script manually.

### Benchmarking
4. The first time `cargo build` is run, it will create a `bindings.rs` file in the `quartz_bindings` folder.
5. Run `cargo run generate` to generate the datasets. For large datasets
   (such as the default ones), it is worth running the script `py-scripts/qasm_to_json.py`
   manually, as the baked-in tool is very slow. For that, run the `generate` command
   with the `--save-files` flag. Once the program starts generating `JSON` files,
   interrupt the program and run `qasm_to_json.py` manually on that folder instead.
6. Run `cargo run --release -- run` to run the benchmarks. By default, results are stored in
   a new `results` folder. See `--help` for more options.

### Plot results
7. Run `cargo run plot` to view the results as a plot. Note that this is equivalent to
   calling the `py-script/plot.py` script.