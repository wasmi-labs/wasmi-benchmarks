use plotters::prelude::*;
use plotters::style::colors::full_palette as color;
use plotters::style::text_anchor::{HPos, Pos, VPos};

/// VM under test and its configuration.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmAndConfig {
    WasmiOld,
    WasmiNew,
    WasmiNewUnchecked,
    WasmiNewLazyTranslation,
    WasmiNewLazy,
    WasmiNewLazyUnchecked,
    Tinywasm,
    Wasm3,
    Wasm3Lazy,
    Stitch,
    WasmtimeCranelift,
    WasmtimeWinch,
    WasmerCranelift,
    WasmerSinglepass,
}

impl VmAndConfig {
    /// Returns the label of the Wasm runtime kind.
    fn label(&self) -> &str {
        match self {
            VmAndConfig::WasmiOld => "Wasmi v0.31",
            VmAndConfig::WasmiNew => "Wasmi v0.32 (eager)",
            VmAndConfig::WasmiNewUnchecked => "Wasmi v0.32 (eager, unchecked)",
            VmAndConfig::WasmiNewLazy => "Wasmi v0.32 (lazy)",
            VmAndConfig::WasmiNewLazyUnchecked => "Wasmi v0.32 (lazy, unchecked)",
            VmAndConfig::WasmiNewLazyTranslation => "Wasmi v0.32 (lazy-translation)",
            VmAndConfig::Tinywasm => "Tinywasm",
            VmAndConfig::Wasm3 => "Wasm3",
            VmAndConfig::Wasm3Lazy => "Wasm3 (lazy)",
            VmAndConfig::WasmtimeCranelift => "Wasmtime (Cranelift)",
            VmAndConfig::WasmtimeWinch => "Wasmtime (Winch)",
            VmAndConfig::WasmerCranelift => "Wasmer (Cranelift)",
            VmAndConfig::WasmerSinglepass => "Wasmer (Singlepass)",
            VmAndConfig::Stitch => "Stitch",
        }
    }

    /// Returns the color associated to the Wasm runtime kind.
    fn color(&self) -> RGBColor {
        match self {
            Self::WasmiOld => RGBColor(140, 130, 50),
            Self::WasmiNew
            | Self::WasmiNewUnchecked
            | Self::WasmiNewLazy
            | Self::WasmiNewLazyUnchecked
            | Self::WasmiNewLazyTranslation => RGBColor(200, 200, 70),
            Self::Wasm3 | Self::Wasm3Lazy => RGBColor(90, 90, 90),
            Self::Tinywasm => RGBColor(108, 140, 108),
            Self::WasmtimeCranelift | Self::WasmtimeWinch => RGBColor(140, 120, 160),
            Self::WasmerCranelift | Self::WasmerSinglepass => RGBColor(95, 140, 175),
            Self::Stitch => RGBColor(220, 175, 180),
        }
    }
}

impl VmAndConfig {
    /// Decode a [`WasmRuntimeKind`] from a `&str`.
    fn decode_str(part: &str) -> Option<Self> {
        match part {
            "wasmi-old" => Some(Self::WasmiOld),
            "wasmi-new.eager" => Some(Self::WasmiNew),
            "wasmi-new.eager.unchecked" => Some(Self::WasmiNewUnchecked),
            "wasmi-new.lazy" => Some(Self::WasmiNewLazy),
            "wasmi-new.lazy.unchecked" => Some(Self::WasmiNewLazyUnchecked),
            "wasmi-new.lazy-translation" => Some(Self::WasmiNewLazyTranslation),
            "wasm3.eager" => Some(Self::Wasm3),
            "wasm3.lazy" => Some(Self::Wasm3Lazy),
            "tinywasm" => Some(Self::Tinywasm),
            "wasmtime.cranelift" => Some(Self::WasmtimeCranelift),
            "wasmtime.winch" => Some(Self::WasmtimeWinch),
            "wasmer.cranelift" => Some(Self::WasmerCranelift),
            "wasmer.singlepass" => Some(Self::WasmerSinglepass),
            "stitch" => Some(Self::Stitch),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BenchEntry {
    pub vm: VmAndConfig,
    pub time: f32,
}

impl BenchEntry {
    pub fn result(&self, min: f32) -> f32 {
        self.time / min
    }
}

fn plot_for_data(test_id: &str, data: &[(&str, f32)]) -> Result<(), Box<dyn std::error::Error>> {
    let min = data
        .iter()
        .map(|(_id, score)| score)
        .copied()
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(1.0);
    let max = data
        .iter()
        .map(|(_id, score)| score)
        .copied()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(1.0);
    let max_diff = core::cmp::max_by(10.0, max / min, f32::total_cmp);
    let mut data: Vec<_> = data
        .iter()
        .map(|&(label, time)| {
            let vm = VmAndConfig::decode_str(label).unwrap();
            BenchEntry { vm, time }
        })
        .collect();
    data.sort_by(|lhs, rhs| lhs.vm.cmp(&rhs.vm));
    data.reverse();

    let path = format!("target/wasmi-benchmarks/{test_id}.svg");
    let _ = std::fs::create_dir_all(&path);
    let _ = std::fs::remove_dir(&path);
    let root = SVGBackend::new(&path, (1280, 960)).into_drawing_area();
    let root = root.margin(5, 5, 5, 5).titled(
        test_id,
        TextStyle::from(("monospace", 50)).pos(Pos::new(HPos::Center, VPos::Center)),
    )?;
    root.fill(&color::WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(75)
        .y_label_area_size(400)
        .margin_right(200)
        .margin_top(25)
        .build_cartesian_2d(
            (0.5_f32..max_diff * 1.05).log_scale(),
            (0usize..data.len() - 1).into_segmented(),
        )?;
    chart
        .configure_mesh()
        .disable_y_mesh()
        .x_max_light_lines(1)
        .bold_line_style(BLACK.mix(0.15))
        .y_desc("") // WebAssembly Runtime
        .x_desc("Relative Time (lower is better, logarithmic scale)")
        .y_label_formatter(&|coord| {
            // We want to draw the Wasm runtime names instead of the numbers.
            match coord {
                SegmentValue::CenterOf(n) => data[*n].vm.label().to_string(),
                SegmentValue::Exact(_n) => unreachable!(),
                SegmentValue::Last => unreachable!(),
            }
        })
        .x_label_style(("sans-serif", 20))
        .y_label_style(("sans-serif", 30))
        .axis_desc_style(("sans-serif", 35))
        .y_labels(data.len())
        .draw()?;

    chart.draw_series(
        Histogram::horizontal(&chart)
            .style_func(|x, _bar_height| match x {
                SegmentValue::Exact(n) => data[*n].vm.color().filled(),
                SegmentValue::CenterOf(_n) => unreachable!(),
                SegmentValue::Last => unreachable!(),
            })
            .margin(20)
            .baseline(0.5)
            .data(
                data.iter()
                    .enumerate()
                    .map(|(index, entry)| (index, entry.result(min))),
            ),
    )?;

    chart.draw_series(data.iter().enumerate().map(|(index, &entry)| {
        let result = entry.result(min);
        Text::new(
            format!("x{result:.02}"),
            (result * 1.05, SegmentValue::CenterOf(index)),
            TextStyle::from(("monospace", 30)).pos(Pos::new(HPos::Left, VPos::Center)),
        )
    }))?;

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present()?;
    Ok(())
}

fn main() {
    let execute_data: Vec<(&str, f32)> = vec![
        ("wasmi-old", 12.0),
        ("wasmi-new.eager", 4.5),
        ("wasmi-new.lazy", 4.55),
        ("tinywasm", 17.25),
        ("wasm3.eager", 2.3),
        ("wasm3.lazy", 2.35),
        ("stitch", 1.9),
        ("wasmtime.cranelift", 0.25),
        ("wasmtime.winch", 0.35),
        ("wasmer.cranelift", 0.3),
        ("wasmer.singlepass", 0.75),
    ];
    plot_for_data("execute/counter", &execute_data).unwrap();

    let compile_data: Vec<(&str, f32)> = vec![
        ("wasmi-old", 250.0),
        ("wasmi-new.eager", 280.5),
        ("wasmi-new.eager.unchecked", 260.25),
        ("wasmi-new.lazy", 46.55),
        ("wasmi-new.lazy.unchecked", 35.32),
        ("wasmi-new.lazy-translation", 129.81),
        ("tinywasm", 244.15),
        ("wasm3.eager", 408.02),
        ("wasm3.lazy", 54.51),
        ("stitch", 147.44),
        ("wasmtime.cranelift", 32756.15),
        ("wasmtime.winch", 3140.78),
        ("wasmer.cranelift", 5102.30),
        ("wasmer.singlepass", 976.73),
    ];
    plot_for_data("compile/pulldown-cmark", &compile_data).unwrap();
}
