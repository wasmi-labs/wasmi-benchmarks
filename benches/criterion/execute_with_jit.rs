use criterion::Criterion;
use wasmi_benchmarks::{read_benchmark_file, vms_under_test, wat2wasm, InputEncoding, TestFilter};

fn execute_benchmark(
    c: &mut Criterion,
    name: &str,
    input: i64,
    encoding: InputEncoding,
    filter: impl Fn(&TestFilter) -> bool,
) {
    let wasm = read_benchmark_file(encoding, name);
    let mut g = c.benchmark_group(format!("execute_jit/{name}"));
    for vm in vms_under_test() {
        if !filter(&vm.test_filter()) {
            continue;
        }
        let id = format!("{}/{}", vm.name(), input);
        g.bench_function(&id, |b| {
            let wasm = wat2wasm(&wasm[..]);
            let mut runtime = vm.load(&wasm[..]);
            b.iter(|| {
                runtime.call(input);
            });
        });
    }
}

pub fn bench_greeting(c: &mut Criterion) {
    execute_benchmark(c, "rwasm-jit", 0, InputEncoding::Wasm, |filter| {
        filter.execute.counter
    })
}

