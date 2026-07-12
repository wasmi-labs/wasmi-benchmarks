use benchmark_utils::{ExecuteTestId, InputEncoding, read_benchmark_file};
use std::collections::BTreeMap;
use wasmi_benchmarks::vms_under_test;

/// Used to query elapsed time since last time this has been called. Used for Coremark benchmark.
fn elapsed_ms() -> u32 {
    use std::time::Instant;
    static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed();
    elapsed.as_millis() as u32
}

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
