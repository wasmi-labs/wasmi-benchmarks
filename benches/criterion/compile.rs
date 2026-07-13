use benchmark_utils::CompileTestId;
use benchmark_utils::{InputEncoding, read_benchmark_file};
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

fn compile_benchmark(
    c: &mut Criterion,
    name: &str,
    encoding: InputEncoding,
    test_id: CompileTestId,
) {
    let wasm = read_benchmark_file(encoding, name);
    let mut g = c.benchmark_group(format!("compile/{name}"));
    for vm in vms_under_test() {
        let bench_id = vm.id().to_string();
        g.bench_function(&bench_id, |b| {
            if !vm.compile(test_id, &wasm[..]) {
                return;
            }
            b.iter(|| {
                vm.compile(test_id, &wasm[..]);
            });
        });
    }
}

pub fn bench_bz2(c: &mut Criterion) {
    compile_benchmark(c, "bz2", InputEncoding::Wasm, CompileTestId::Bz2)
}

pub fn bench_pulldown_cmark(c: &mut Criterion) {
    compile_benchmark(
        c,
        "pulldown-cmark",
        InputEncoding::Wasm,
        CompileTestId::PulldownCmark,
    )
}

pub fn bench_spidermonkey(c: &mut Criterion) {
    compile_benchmark(
        c,
        "spidermonkey",
        InputEncoding::Wasm,
        CompileTestId::Spidermonkey,
    )
}

pub fn bench_ffmpeg(c: &mut Criterion) {
    compile_benchmark(c, "ffmpeg", InputEncoding::Wasm, CompileTestId::Ffmpeg)
}

pub fn bench_coremark_minimal(c: &mut Criterion) {
    compile_benchmark(
        c,
        "coremark-minimal",
        InputEncoding::Wasm,
        CompileTestId::CoreMark,
    )
}

pub fn bench_argon2(c: &mut Criterion) {
    compile_benchmark(c, "argon2", InputEncoding::Wasm, CompileTestId::Argon2)
}

pub fn bench_erc20(c: &mut Criterion) {
    compile_benchmark(c, "erc20", InputEncoding::Wasm, CompileTestId::Erc20)
}
