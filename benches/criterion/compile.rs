use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion};
use wasmi_benchmarks::{vms_under_test, BenchVm};

/// Parses the `wasm` bytes and returns a Wasmi [`Module`].
///
/// The returned [`Module`] can then be used to query import information.
/// This import information is then fed into the benchmarked VMs for their disposal.
///
/// [`Module`]: wasmi_new::Module
fn parse_module(wasm: &[u8]) -> wasmi_new::Module {
    let mut config = wasmi_new::Config::default();
    config.wasm_tail_call(true);
    config.compilation_mode(wasmi_new::CompilationMode::Lazy);
    let engine = wasmi_new::Engine::new(&config);
    wasmi_new::Module::new(&engine, wasm).unwrap()
}

fn run_bz2(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.bz2 {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/bz2.wasm");
    let name = vm.name();
    let id = format!("{name}");
    let module = parse_module(WASM);
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..], module.imports());
        });
    });
}

pub fn bench_bz2(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/bz2");
    for vm in vms_under_test() {
        run_bz2(&mut g, &*vm);
    }
}

fn run_pulldown_cmark(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.pulldown_cmark {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/pulldown-cmark.wasm");
    let name = vm.name();
    let id = format!("{name}");
    let module = parse_module(WASM);
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..], module.imports());
        });
    });
}

pub fn bench_pulldown_cmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/pulldown-cmark");
    for vm in vms_under_test() {
        run_pulldown_cmark(&mut g, &*vm);
    }
}

fn run_spidermonkey(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.pulldown_cmark {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/spidermonkey.wasm");
    let name = vm.name();
    let id = format!("{name}");
    let module = parse_module(WASM);
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..], module.imports());
        });
    });
}

pub fn bench_spidermonkey(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/spidermonkey");
    for vm in vms_under_test() {
        run_spidermonkey(&mut g, &*vm);
    }
}

fn run_ffmpeg(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.ffmpeg {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/ffmpeg.wasm");
    let name = vm.name();
    let id = format!("{name}");
    let module = parse_module(WASM);
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..], module.imports());
        });
    });
}

pub fn bench_ffmpeg(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/ffmpeg");
    for vm in vms_under_test() {
        run_ffmpeg(&mut g, &*vm);
    }
}

fn run_coremark_minimal(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.coremark_minimal {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/coremark-minimal.wasm");
    let name = vm.name();
    let id = format!("{name}");
    let module = parse_module(WASM);
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..], module.imports());
        });
    });
}

pub fn bench_coremark_minimal(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/coremark-minimal");
    for vm in vms_under_test() {
        run_coremark_minimal(&mut g, &*vm);
    }
}

fn run_argon2(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/argon2.wasm");
    let name = vm.name();
    let id = format!("{name}");
    let module = parse_module(WASM);
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..], module.imports());
        });
    });
}

pub fn bench_argon2(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/argon2");
    for vm in vms_under_test() {
        run_argon2(&mut g, &*vm);
    }
}

fn run_erc20(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.coremark_minimal {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/erc20.wasm");
    let name = vm.name();
    let id = format!("{name}");
    let module = parse_module(WASM);
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..], module.imports());
        });
    });
}

pub fn bench_erc20(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/erc20");
    for vm in vms_under_test() {
        run_erc20(&mut g, &*vm);
    }
}
