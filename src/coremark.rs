use std::collections::BTreeMap;
use wasmi_benchmarks::{read_benchmark_file, vms_under_test, InputEncoding};

fn main() {
    let coremark_wasm = read_benchmark_file(InputEncoding::Wasm, "coremark-minimal");
    let mut scores = <BTreeMap<String, f32>>::new();
    for vm in vms_under_test() {
        let name = vm.name();
        println!(
            "\
            Running Coremark 1.0\n\
            \tusing {name} ...\
        "
        );
        let score = vm.coremark(&coremark_wasm[..]);
        scores.insert(name.into(), score);
        println!("\tscore = {score}\n");
    }
    let json = serde_json::to_value(&scores).unwrap();
    println!("Scores Summary (JSON):\n{json:#}\n");
}
