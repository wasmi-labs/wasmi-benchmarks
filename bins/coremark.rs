use benchmark_utils::{ExecuteTestId, InputEncoding, elapsed_ms, read_benchmark_file};
use std::collections::BTreeMap;
use wasmi_benchmarks::vms_under_test;

fn main() {
    let coremark_wasm = read_benchmark_file(InputEncoding::Wasm, "coremark-minimal");
    let mut scores = <BTreeMap<String, f32>>::new();
    for vm in vms_under_test() {
        if !vm.can_run(ExecuteTestId::CoreMark.into()) {
            continue;
        }
        let id = vm.id();
        println!(
            "\
            Running Coremark 1.0\n\
            \tusing {id} ...\
        "
        );
        let score = vm.coremark(&coremark_wasm[..], elapsed_ms);
        scores.insert(id.into(), score);
        println!("\tscore = {score}\n");
    }
    let json = serde_json::to_value(&scores).unwrap();
    println!("Scores Summary (JSON):\n{json:#}\n");
}
