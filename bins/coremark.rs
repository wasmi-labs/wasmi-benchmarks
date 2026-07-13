use benchmark_utils::{ExecuteTestId, FuncType, InputEncoding, Val, ValType, read_benchmark_file};
use std::collections::BTreeMap;
use wasmi_benchmarks::vms_under_test;

/// Used to query elapsed time since last time this has been called. Used for Coremark benchmark.
fn elapsed_ms() -> u32 {
    use std::time::Instant;
    static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed();
    elapsed.as_millis() as u32
}

/// The `env.clock_ms` host function imported by the Coremark Wasm.
///
/// Defined as a plain `fn` (no captured state) so it can be linked through the generic
/// [`benchmark_utils::RuntimeInstance::link_func`] interface, which every runtime supports.
fn clock_ms(_params: &[Val], results: &mut [Val]) {
    results[0] = Val::I32(elapsed_ms() as i32);
}

fn main() {
    let coremark_wasm = read_benchmark_file(InputEncoding::Wasm, "coremark-minimal");
    let mut scores = <BTreeMap<String, f32>>::new();
    for vm in vms_under_test() {
        let Some(mut rt) = vm.setup(ExecuteTestId::CoreMark.into()) else {
            continue;
        };
        let id = vm.id();
        println!(
            "\
            Running Coremark 1.0\n\
            \tusing {id} ...\
        "
        );
        rt.link_func(
            "env",
            "clock_ms",
            FuncType::new([], [ValType::I32]),
            clock_ms,
        );
        let mut instance = rt.instantiate(&coremark_wasm[..]);
        let mut results = [Val::F32(0.0)];
        instance.call("run", &[], &mut results[..]).unwrap();
        let score = results[0].unwrap_f32();
        scores.insert(id.into(), score);
        println!("\tscore = {score}\n");
    }
    let json = serde_json::to_value(&scores).unwrap();
    println!("Scores Summary (JSON):\n{json:#}\n");
}
