use crate::utils::{wat2wasm, TestFilter};
use crate::vms;
use crate::vms::{BenchRuntime, BenchVm};
use criterion::measurement::WallTime;
use criterion::{criterion_group, criterion_main, Bencher, BenchmarkGroup, Criterion};
use std::time::Duration;

fn run_bz2(g: &mut BenchmarkGroup<WallTime>, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.bz2 {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/bz2.wasm");
    let name = vm.name();
    let id = format!("{name}");
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_bz2(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/bz2");
    for vm in vms() {
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
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_pulldown_cmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/pulldown-cmark");
    for vm in vms() {
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
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_spidermonkey(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/spidermonkey");
    for vm in vms() {
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
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_ffmpeg(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/ffmpeg");
    for vm in vms() {
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
    g.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_coremark_minimal(c: &mut Criterion) {
    let mut g = c.benchmark_group("compile/coremark-minimal");
    for vm in vms() {
        run_coremark_minimal(&mut g, &*vm);
    }
}
