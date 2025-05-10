use plotters::prelude::*;
use plotters::style::colors::full_palette as color;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display};
use std::str::FromStr;

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
    WasmtimePulley,
    WasmerCranelift,
    WasmerSinglepass,
    WasmerWamr,
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
            VmAndConfig::Wasm3 => "Wasm3 (eager)",
            VmAndConfig::Wasm3Lazy => "Wasm3 (lazy)",
            VmAndConfig::Stitch => "Stitch (lazy)",
            VmAndConfig::WasmtimeCranelift => "Wasmtime (Cranelift)",
            VmAndConfig::WasmtimeWinch => "Wasmtime (Winch)",
            VmAndConfig::WasmtimePulley => "Wasmtime (Pulley)",
            VmAndConfig::WasmerCranelift => "Wasmer (Cranelift)",
            VmAndConfig::WasmerSinglepass => "Wasmer (Singlepass)",
            VmAndConfig::WasmerWamr => "Wasmer (WAMR)",
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
            Self::Tinywasm => RGBColor(108, 140, 108),
            Self::Wasm3 | Self::Wasm3Lazy => RGBColor(90, 90, 90),
            Self::Stitch => RGBColor(220, 175, 180),
            Self::WasmtimeCranelift | Self::WasmtimeWinch | Self::WasmtimePulley => {
                RGBColor(140, 120, 160)
            }
            Self::WasmerCranelift | Self::WasmerSinglepass | Self::WasmerWamr => {
                RGBColor(95, 140, 175)
            }
        }
    }
}

impl FromStr for VmAndConfig {
    type Err = FromStrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "wasmi-old" => Ok(Self::WasmiOld),
            "wasmi-new.eager.checked" => Ok(Self::WasmiNew),
            "wasmi-new.eager.unchecked" => Ok(Self::WasmiNewUnchecked),
            "wasmi-new.lazy.checked" => Ok(Self::WasmiNewLazy),
            "wasmi-new.lazy.unchecked" => Ok(Self::WasmiNewLazyUnchecked),
            "wasmi-new.lazy-translation.checked" => Ok(Self::WasmiNewLazyTranslation),
            "tinywasm" => Ok(Self::Tinywasm),
            "wasm3.eager" => Ok(Self::Wasm3),
            "wasm3.lazy" => Ok(Self::Wasm3Lazy),
            "stitch" => Ok(Self::Stitch),
            "wasmtime.cranelift" => Ok(Self::WasmtimeCranelift),
            "wasmtime.winch" => Ok(Self::WasmtimeWinch),
            "wasmtime.pulley" => Ok(Self::WasmtimePulley),
            "wasmer.cranelift" => Ok(Self::WasmerCranelift),
            "wasmer.singlepass" => Ok(Self::WasmerSinglepass),
            "wasmer.wamr" => Ok(Self::WasmerWamr),
            _ => Err(FromStrError::from(format!("invalid VmAndConfig: {input}"))),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BenchEntry {
    pub vm: VmAndConfig,
    pub time: f64,
}

impl BenchEntry {
    pub fn result(&self, min: f64) -> f64 {
        self.time / min
    }
}

fn plot_for_data(ext_title: Option<&str>, bench_group: &BenchGroup) -> Result<(), Box<dyn Error>> {
    let min = bench_group
        .results
        .iter()
        .map(|(_id, &BenchResult { estimate, unit: _ })| estimate)
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(1.0);
    let max = bench_group
        .results
        .iter()
        .map(|(_id, &BenchResult { estimate, unit: _ })| estimate)
        .max_by(|a, b| a.total_cmp(b))
        .unwrap_or(1.0);
    let max_diff = core::cmp::max_by(10.0, max / min, f64::total_cmp);
    let mut data: Vec<_> = bench_group
        .results
        .iter()
        .map(|(&vm, &BenchResult { estimate, unit: _ })| BenchEntry { vm, time: estimate })
        .collect();
    data.sort_by(|lhs, rhs| lhs.vm.cmp(&rhs.vm));
    data.reverse();

    let category = bench_group.category;
    let name = &bench_group.name;
    let test_id = format!("{category}/{name}");
    let test_title = match ext_title {
        Some(ext_title) => format!("{test_id} - {ext_title}"),
        None => test_id,
    };
    let path = format!("target/wasmi-benchmarks/{category}/{name}.svg");
    let _ = std::fs::create_dir_all(&path);
    let _ = std::fs::remove_dir(&path);
    let height = 50 + 75 + 25 + 5 + bench_group.results.len() as u32 * 50;
    let root = SVGBackend::new(&path, (1280, height)).into_drawing_area();
    root.fill(&color::WHITE)?;
    let root = root.margin(5, 5, 5, 5).titled(
        &test_title,
        TextStyle::from(("monospace", 45)).pos(Pos::new(HPos::Center, VPos::Center)),
    )?;
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(75)
        .y_label_area_size(400)
        .margin_right(200)
        .margin_top(25)
        .build_cartesian_2d(
            (0.5_f64..max_diff * 1.05).log_scale(),
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
        .x_labels(3)
        .y_labels(data.len())
        .draw()?;

    chart.draw_series(
        Histogram::horizontal(&chart)
            .style_func(|x, _bar_height| match x {
                SegmentValue::Exact(n) => data[*n].vm.color().filled(),
                SegmentValue::CenterOf(_n) => unreachable!(),
                SegmentValue::Last => unreachable!(),
            })
            .margin(15)
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BenchCategory {
    Execute,
    Compile,
}

#[derive(Debug)]
pub struct FromStrError {
    message: String,
}

impl Error for FromStrError {}

impl<S> From<S> for FromStrError
where
    S: Into<String>,
{
    fn from(message: S) -> Self {
        FromStrError {
            message: message.into(),
        }
    }
}

impl Display for FromStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl FromStr for BenchCategory {
    type Err = FromStrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "execute" => Ok(Self::Execute),
            "compile" => Ok(Self::Compile),
            _ => Err(FromStrError::from(format!(
                "invalid BenchCategory: {input}"
            ))),
        }
    }
}

impl fmt::Display for BenchCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BenchCategory::Execute => "execute".fmt(f),
            BenchCategory::Compile => "compile".fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct BenchGroup {
    pub category: BenchCategory,
    pub name: String,
    pub results: BTreeMap<VmAndConfig, BenchResult>,
    pub input: Option<i64>,
}

#[derive(Debug)]
pub struct BenchResult {
    pub estimate: f64,
    pub unit: String,
}

fn decode_stdin() -> Result<(), Box<dyn Error>> {
    use serde_json as json;
    use std::io::{self, BufRead};

    let args: Vec<String> = std::env::args().collect();
    let ext_title = args.get(1).cloned();

    // Create a buffer to read input
    let stdin = io::stdin();
    let handle = stdin.lock();

    let mut bench_group: Option<BenchGroup> = None;

    // Iterate over lines from stdin and collect data:
    for line in handle.lines() {
        let line = line?;

        let json: json::Value = json::from_str(&line)?;
        let json::Value::Object(map) = &json else {
            panic!("malformed JSON input: {json:?}")
        };
        match map.get("reason").and_then(json::Value::as_str) {
            Some("benchmark-complete") => {
                // Important message properties:
                //
                // reason: benchmark-complete
                //     - id: {exec-or-compile} / {test-case} / {wasm-runtime} / {input}
                //     - typical: { "estimate": f32, "unit": ["ns", "us", "ms", "s"] }
                let Some(id) = map.get("id").and_then(json::Value::as_str) else {
                    panic!("malformed `id` value: {json:?}")
                };
                let mut parts = id.split('/');
                let category = BenchCategory::from_str(parts.next().unwrap())?;
                let name = String::from(parts.next().unwrap());
                let vm_and_config = VmAndConfig::from_str(parts.next().unwrap())?;
                let input = parts.next().map(|s| s.parse::<i64>()).transpose()?;
                let Some(typical) = map.get("typical").and_then(json::Value::as_object) else {
                    panic!("malformed `typical` value: {json:#?}")
                };
                let Some(estimate) = typical
                    .get("estimate")
                    .and_then(json::Value::as_number)
                    .and_then(json::Number::as_f64)
                else {
                    panic!("malformed `typical.estimate` value: {json:#?}")
                };
                let Some(unit) = typical
                    .get("unit")
                    .and_then(json::Value::as_str)
                    .map(String::from)
                else {
                    panic!("malformed `typical.unit` value: {json:#?}")
                };
                let result = BenchResult { estimate, unit };
                match &mut bench_group {
                    Some(bench_group) => {
                        assert_eq!(&bench_group.category, &category);
                        assert_eq!(&bench_group.name, &name);
                        assert_eq!(&bench_group.input, &input);
                        assert!(bench_group.results.insert(vm_and_config, result).is_none());
                    }
                    None => {
                        let g = bench_group.insert(BenchGroup {
                            category,
                            name,
                            input,
                            results: BTreeMap::new(),
                        });
                        g.results.insert(vm_and_config, result);
                    }
                };
            }
            Some("group-complete") => {
                // Important message properties:
                //
                // reason: group-complete
                //     - group_name: "{exec-or-compile} / {test-case}"
                if let Some(bench_group) = bench_group.take() {
                    plot_for_data(ext_title.as_deref(), &bench_group)?;
                }
            }
            _ => panic!("malformed JSON input: {json:?}"),
        };
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    decode_stdin()
}
