use std::io::Write;
use wasmi_benchmarks::{read_benchmark_file, vms_under_test, InputEncoding};

fn main() {
    let coremark_wasm = read_benchmark_file(InputEncoding::Wasm, "coremark-minimal");
    for vm in vms_under_test() {
        let name = vm.name();
        print!("Running Coremark 1.0\n\tusing {name} ");
        std::io::stdout().flush().unwrap();
        let result = vm.coremark(&coremark_wasm[..]);
        println!("\n\tresult = {result}\n");
    }
}
