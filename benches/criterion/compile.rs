use benchmark_utils::CompileTestId;
use benchmark_utils::{InputEncoding, read_benchmark_file};
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

fn compile_benchmark(c: &mut Criterion, name: &str, encoding: InputEncoding, id: CompileTestId) {
    let wasm = read_benchmark_file(encoding, name);
    let mut g = c.benchmark_group(format!("compile/{name}"));
    for vm in vms_under_test() {
        if !vm.can_run(id.into()) {
            continue;
        }
        let id = format!("{}", vm.name());
        g.bench_function(&id, |b| {
            b.iter(|| {
                vm.compile(&wasm[..]);
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
        CompileTestId::CoreMarkMinimal,
    )
}

pub fn bench_argon2(c: &mut Criterion) {
    compile_benchmark(c, "argon2", InputEncoding::Wasm, CompileTestId::Argon2)
}

pub fn bench_erc20(c: &mut Criterion) {
    compile_benchmark(c, "erc20", InputEncoding::Wasm, CompileTestId::Erc20)
}
