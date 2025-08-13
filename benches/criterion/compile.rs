use benchmark_utils::{InputEncoding, read_benchmark_file};
use benchmark_utils::{TestFilter, parse_module};
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

fn compile_benchmark(
    c: &mut Criterion,
    name: &str,
    encoding: InputEncoding,
    filter: impl Fn(&TestFilter) -> bool,
) {
    let wasm = read_benchmark_file(encoding, name);
    let module = parse_module(&wasm[..]);
    let mut g = c.benchmark_group(format!("compile/{name}"));
    for vm in vms_under_test() {
        if !filter(&vm.test_filter()) {
            continue;
        }
        let id = format!("{}", vm.name());
        g.bench_function(&id, |b| {
            b.iter(|| {
                vm.compile(&wasm[..], module.imports());
            });
        });
    }
}

pub fn bench_bz2(c: &mut Criterion) {
    compile_benchmark(c, "bz2", InputEncoding::Wasm, |filter| filter.compile.bz2)
}

pub fn bench_pulldown_cmark(c: &mut Criterion) {
    compile_benchmark(c, "pulldown-cmark", InputEncoding::Wasm, |filter| {
        filter.compile.pulldown_cmark
    })
}

pub fn bench_spidermonkey(c: &mut Criterion) {
    compile_benchmark(c, "spidermonkey", InputEncoding::Wasm, |filter| {
        filter.compile.spidermonkey
    })
}

pub fn bench_ffmpeg(c: &mut Criterion) {
    compile_benchmark(c, "ffmpeg", InputEncoding::Wasm, |filter| {
        filter.compile.ffmpeg
    })
}

pub fn bench_coremark_minimal(c: &mut Criterion) {
    compile_benchmark(c, "coremark-minimal", InputEncoding::Wasm, |filter| {
        filter.compile.coremark_minimal
    })
}

pub fn bench_argon2(c: &mut Criterion) {
    compile_benchmark(c, "argon2", InputEncoding::Wasm, |filter| {
        filter.compile.argon2
    })
}

pub fn bench_erc20(c: &mut Criterion) {
    compile_benchmark(c, "erc20", InputEncoding::Wasm, |filter| {
        filter.compile.coremark_minimal
    })
}
