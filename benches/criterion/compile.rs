use crate::utils::{wat2wasm, TestFilter};
use crate::vms;
use crate::vms::{BenchRuntime, BenchVm};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use std::time::Duration;

fn run_bz2(c: &mut Criterion, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.bz2 {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/bz2.wasm");
    let name = vm.name();
    let id = format!("compile/bz2/{name}");
    c.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_bz2(c: &mut Criterion) {
    for vm in vms() {
        run_bz2(c, &*vm);
    }
}

fn run_pulldown_cmark(c: &mut Criterion, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.pulldown_cmark {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/pulldown-cmark.wasm");
    let name = vm.name();
    let id = format!("compile/pulldown-cmark/{name}");
    c.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_pulldown_cmark(c: &mut Criterion) {
    for vm in vms() {
        run_pulldown_cmark(c, &*vm);
    }
}

fn run_spidermonkey(c: &mut Criterion, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.pulldown_cmark {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/spidermonkey.wasm");
    let name = vm.name();
    let id = format!("compile/spidermonkey/{name}");
    c.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_spidermonkey(c: &mut Criterion) {
    for vm in vms() {
        run_spidermonkey(c, &*vm);
    }
}

fn run_ffmpeg(c: &mut Criterion, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.ffmpeg {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/ffmpeg.wasm");
    let name = vm.name();
    let id = format!("compile/ffmpeg/{name}");
    c.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_ffmpeg(c: &mut Criterion) {
    for vm in vms() {
        run_ffmpeg(c, &*vm);
    }
}

fn run_coremark_minimal(c: &mut Criterion, vm: &dyn BenchVm) {
    if !vm.test_filter().compile.coremark_minimal {
        return;
    }
    static WASM: &[u8] = include_bytes!("../res/wasm/coremark-minimal.wasm");
    let name = vm.name();
    let id = format!("compile/coremark-minimal/{name}");
    c.bench_function(&id, |b| {
        b.iter(|| {
            vm.compile(&WASM[..]);
        });
    });
}

pub fn bench_coremark_minimal(c: &mut Criterion) {
    for vm in vms() {
        run_coremark_minimal(c, &*vm);
    }
}
