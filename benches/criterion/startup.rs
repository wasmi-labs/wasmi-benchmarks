use benchmark_utils::{FuncType, InputEncoding, StartupTestId, Val, ValType, read_benchmark_file};
use criterion::Criterion;
use wasmi_benchmarks::vms_under_test;

/// Inert host function stub used for all linked imports.
///
/// The instantiation benchmarks stop right after instantiation and never call into the module, and
/// none of the benchmarked modules have a `start` section that invokes an imported function, so a
/// stub that does nothing is sufficient to satisfy linking.
fn stub(_params: &[Val], _results: &mut [Val]) {}

/// Convenience constructor for a runtime-neutral [`FuncType`] from slices of [`ValType`].
fn func_ty(params: &[ValType], results: &[ValType]) -> FuncType {
    FuncType::new(params.iter().copied(), results.iter().copied())
}

/// Returns the imported functions (with their signatures) required to instantiate `id`.
///
/// Every module imports functions only (no memory/table/global imports), so linking these host
/// function stubs via [`RuntimeInstance::link_func`](benchmark_utils::RuntimeInstance::link_func) is
/// enough to instantiate.
fn required_imports(id: StartupTestId) -> Vec<(&'static str, &'static str, FuncType)> {
    use ValType::{I32, I64};
    // Signature shorthands shared across the WASI-heavy modules.
    let unit = || func_ty(&[], &[]);
    let i = || func_ty(&[I32], &[]);
    let i_i = || func_ty(&[I32], &[I32]);
    let ii = || func_ty(&[I32, I32], &[]);
    let ii_i = || func_ty(&[I32, I32], &[I32]);
    let iii = || func_ty(&[I32, I32, I32], &[]);
    let iii_i = || func_ty(&[I32, I32, I32], &[I32]);
    let iiii = || func_ty(&[I32, I32, I32, I32], &[]);
    let iiii_i = || func_ty(&[I32, I32, I32, I32], &[I32]);
    let iiiii_i = || func_ty(&[I32, I32, I32, I32, I32], &[I32]);
    let iiiiii_i = || func_ty(&[I32, I32, I32, I32, I32, I32], &[I32]);
    let ili_i = || func_ty(&[I32, I64, I32], &[I32]);
    let ilii_i = || func_ty(&[I32, I64, I32, I32], &[I32]);
    let iiili_i = || func_ty(&[I32, I32, I32, I64, I32], &[I32]);
    let path_open = || func_ty(&[I32, I32, I32, I32, I32, I64, I64, I32, I32], &[I32]);
    let wasi = "wasi_snapshot_preview1";
    match id {
        StartupTestId::Argon2 => vec![],
        StartupTestId::CoreMark => vec![("env", "clock_ms", func_ty(&[], &[I32]))],
        StartupTestId::Erc20 => vec![
            ("__unstable__", "seal_get_storage", iiii_i()),
            ("__unstable__", "seal_set_storage", iiii_i()),
            ("seal0", "seal_value_transferred", ii()),
            ("seal0", "seal_input", ii()),
            ("seal0", "seal_caller", ii()),
            ("seal0", "seal_deposit_event", iiii()),
            ("seal0", "seal_return", iii()),
            ("seal0", "seal_hash_blake2_256", iii()),
        ],
        StartupTestId::Bz2 => vec![
            ("bench", "start", unit()),
            ("bench", "end", unit()),
            (wasi, "proc_exit", i()),
            (wasi, "fd_close", i_i()),
            (wasi, "args_get", ii_i()),
            (wasi, "args_sizes_get", ii_i()),
            (wasi, "fd_prestat_get", ii_i()),
            (wasi, "fd_fdstat_get", ii_i()),
            (wasi, "fd_prestat_dir_name", iii_i()),
            (wasi, "fd_read", iiii_i()),
            (wasi, "fd_write", iiii_i()),
            (wasi, "path_filestat_get", iiiii_i()),
            (wasi, "fd_seek", ilii_i()),
            (wasi, "path_open", path_open()),
        ],
        StartupTestId::PulldownCmark => vec![
            ("bench", "start", unit()),
            ("bench", "end", unit()),
            (wasi, "proc_exit", i()),
            (wasi, "fd_close", i_i()),
            (wasi, "fd_filestat_get", ii_i()),
            (wasi, "fd_prestat_get", ii_i()),
            (wasi, "random_get", ii_i()),
            (wasi, "environ_sizes_get", ii_i()),
            (wasi, "environ_get", ii_i()),
            (wasi, "fd_prestat_dir_name", iii_i()),
            (wasi, "fd_read", iiii_i()),
            (wasi, "fd_write", iiii_i()),
            (wasi, "path_open", path_open()),
        ],
        StartupTestId::Spidermonkey => vec![
            ("bench", "start", unit()),
            ("bench", "end", unit()),
            (wasi, "proc_exit", i()),
            (wasi, "fd_close", i_i()),
            (wasi, "args_get", ii_i()),
            (wasi, "args_sizes_get", ii_i()),
            (wasi, "environ_get", ii_i()),
            (wasi, "environ_sizes_get", ii_i()),
            (wasi, "clock_res_get", ii_i()),
            (wasi, "fd_fdstat_get", ii_i()),
            (wasi, "fd_fdstat_set_flags", ii_i()),
            (wasi, "fd_prestat_get", ii_i()),
            (wasi, "fd_prestat_dir_name", iii_i()),
            (wasi, "path_remove_directory", iii_i()),
            (wasi, "path_unlink_file", iii_i()),
            (wasi, "fd_read", iiii_i()),
            (wasi, "fd_write", iiii_i()),
            (wasi, "clock_time_get", ili_i()),
            (wasi, "fd_seek", ilii_i()),
            (wasi, "path_open", path_open()),
        ],
        StartupTestId::Ffmpeg => vec![
            (wasi, "proc_exit", i()),
            (wasi, "fd_close", i_i()),
            (wasi, "args_get", ii_i()),
            (wasi, "args_sizes_get", ii_i()),
            (wasi, "environ_get", ii_i()),
            (wasi, "environ_sizes_get", ii_i()),
            (wasi, "fd_fdstat_get", ii_i()),
            (wasi, "fd_fdstat_set_flags", ii_i()),
            (wasi, "fd_filestat_get", ii_i()),
            (wasi, "fd_prestat_get", ii_i()),
            (wasi, "fd_prestat_dir_name", iii_i()),
            (wasi, "path_create_directory", iii_i()),
            (wasi, "path_remove_directory", iii_i()),
            (wasi, "path_unlink_file", iii_i()),
            (wasi, "fd_read", iiii_i()),
            (wasi, "fd_write", iiii_i()),
            (wasi, "poll_oneoff", iiii_i()),
            (wasi, "path_filestat_get", iiiii_i()),
            (wasi, "path_rename", iiiiii_i()),
            (wasi, "clock_time_get", ili_i()),
            (wasi, "fd_seek", ilii_i()),
            (wasi, "fd_readdir", iiili_i()),
            (wasi, "path_open", path_open()),
        ],
    }
}

fn instantiate_benchmark(c: &mut Criterion, encoding: InputEncoding, id: StartupTestId) {
    let wasm = read_benchmark_file(encoding, id.into());
    let imports = required_imports(id);
    let mut g = c.benchmark_group(format!("startup/{id}"));
    for vm in vms_under_test() {
        let Some(mut rt) = vm.setup(id.into()) else {
            continue;
        };
        // Link every imported function once as an inert stub. Only `instantiate` is timed below.
        for (module, field, ty) in &imports {
            rt.link_func(module, field, ty.clone(), stub);
        }
        let bench_id = vm.id().to_string();
        g.bench_function(&bench_id, |b| {
            b.iter(|| {
                rt.instantiate(&wasm[..]);
            });
        });
    }
}

pub fn bench_bz2(c: &mut Criterion) {
    instantiate_benchmark(c, InputEncoding::Wasm, StartupTestId::Bz2)
}

pub fn bench_pulldown_cmark(c: &mut Criterion) {
    instantiate_benchmark(c, InputEncoding::Wasm, StartupTestId::PulldownCmark)
}

pub fn bench_spidermonkey(c: &mut Criterion) {
    instantiate_benchmark(c, InputEncoding::Wasm, StartupTestId::Spidermonkey)
}

pub fn bench_ffmpeg(c: &mut Criterion) {
    instantiate_benchmark(c, InputEncoding::Wasm, StartupTestId::Ffmpeg)
}

pub fn bench_coremark_minimal(c: &mut Criterion) {
    instantiate_benchmark(c, InputEncoding::Wasm, StartupTestId::CoreMark)
}

pub fn bench_argon2(c: &mut Criterion) {
    instantiate_benchmark(c, InputEncoding::Wasm, StartupTestId::Argon2)
}

pub fn bench_erc20(c: &mut Criterion) {
    instantiate_benchmark(c, InputEncoding::Wasm, StartupTestId::Erc20)
}
