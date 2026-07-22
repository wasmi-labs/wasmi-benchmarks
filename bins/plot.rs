use clap::Parser;
use plotters::coord::Shift;
use plotters::coord::ranged1d::{Ranged, SegmentedCoord, ValueFormatter};
use plotters::coord::types::RangedCoordusize;
use plotters::prelude::*;
use plotters::style::colors::full_palette as color;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display};
use std::str::FromStr;

/// Scaling of the relative-time axis in the rendered plots.
#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
enum Scale {
    /// Logarithmic scaling.
    Log,
    /// Linear scaling.
    Linear,
}

/// How measured times are expressed in the rendered plots.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, clap::ValueEnum)]
enum Time {
    /// Values relative to the fastest runtime in the group (e.g. `x2.35`).
    #[default]
    Relative,
    /// Absolute measured durations (e.g. `5.23 ms`).
    Absolute,
}

/// Excludes a kind of Wasm runtime from the rendered plots.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, clap::ValueEnum)]
enum Filter {
    /// Include all Wasm runtimes.
    #[default]
    None,
    /// Exclude all JIT-compiling Wasm runtimes.
    Jit,
    /// Exclude all interpreter-based Wasm runtimes.
    Interpreter,
}

impl Filter {
    /// Returns `true` if the given `vm` passes this filter.
    fn keeps(self, vm: VmAndConfig) -> bool {
        match self {
            Filter::None => true,
            Filter::Jit => vm.kind() != RuntimeKind::Jit,
            Filter::Interpreter => vm.kind() != RuntimeKind::Interpreter,
        }
    }
}

/// The execution kind of a Wasm runtime.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum RuntimeKind {
    /// A JIT-compiling Wasm runtime.
    Jit,
    /// An interpreter-based Wasm runtime.
    Interpreter,
}

/// Renders Criterion benchmark results (read as JSON from stdin) into SVG plots.
#[derive(Debug, Parser)]
struct Args {
    /// Optional external title appended to each plot's title.
    title: Option<String>,
    /// Scaling of the relative-time axis.
    #[arg(long, value_enum, default_value_t = Scale::Log)]
    scale: Scale,
    /// Whether to plot relative or absolute times.
    #[arg(long, value_enum, default_value_t = Time::Relative)]
    time: Time,
    /// Excludes a kind of Wasm runtime from the plots.
    #[arg(long, value_enum, default_value_t = Filter::None)]
    filter: Filter,
}

/// VM under test and its configuration.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmAndConfig {
    Wasmi031,
    Wasmi032,
    WasmiV1(WasmiConfig),
    WasmiV2(WasmiConfig),
    Wasmtime(WasmtimeConfig),
    DlrWasmInterpreter,
    Fizzy,
    SpaceWasm,
    Stitch,
    Tinywasm,
    Toywasm,
    V8,
    Wamr,
    Wasm3(Wasm3Config),
    WasmEdge,
    Wasmer(WasmerConfig),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmiConfig {
    Checked,
    Unchecked,
    LazyTranslation,
    Lazy,
    LazyUnchecked,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Wasm3Config {
    Lazy,
    Eager,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmtimeConfig {
    Cranelift,
    Winch,
    Pulley,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmerConfig {
    Cranelift,
    Singlepass,
}

impl VmAndConfig {
    /// Returns the label of the Wasm runtime kind.
    fn label(&self) -> &str {
        match self {
            Self::DlrWasmInterpreter => "DLR-wasm-interpreter",
            Self::Fizzy => "Fizzy",
            Self::SpaceWasm => "SpaceWasm",
            Self::Stitch => "Stitch (lazy)",
            Self::Tinywasm => "Tinywasm",
            Self::Toywasm => "Toywasm",
            Self::V8 => "V8",
            Self::Wamr => "WAMR fast-interpreter",
            Self::Wasm3(Wasm3Config::Eager) => "Wasm3 (eager)",
            Self::Wasm3(Wasm3Config::Lazy) => "Wasm3 (lazy)",
            Self::WasmEdge => "WasmEdge (interpreter)",
            Self::Wasmer(WasmerConfig::Cranelift) => "Wasmer (Cranelift)",
            Self::Wasmer(WasmerConfig::Singlepass) => "Wasmer (Singlepass)",
            Self::Wasmi031 => "Wasmi v0.31",
            Self::Wasmi032 => "Wasmi v0.32",
            Self::WasmiV1(WasmiConfig::Checked) => "Wasmi v1 (eager)",
            Self::WasmiV1(WasmiConfig::Unchecked) => "Wasmi v1 (eager, unchecked)",
            Self::WasmiV1(WasmiConfig::Lazy) => "Wasmi v1 (lazy)",
            Self::WasmiV1(WasmiConfig::LazyUnchecked) => "Wasmi v1 (lazy, unchecked)",
            Self::WasmiV1(WasmiConfig::LazyTranslation) => "Wasmi v1 (lazy-translation)",
            Self::WasmiV2(WasmiConfig::Checked) => "Wasmi v2 (eager)",
            Self::WasmiV2(WasmiConfig::Unchecked) => "Wasmi v2 (eager, unchecked)",
            Self::WasmiV2(WasmiConfig::Lazy) => "Wasmi v2 (lazy)",
            Self::WasmiV2(WasmiConfig::LazyUnchecked) => "Wasmi v2 (lazy, unchecked)",
            Self::WasmiV2(WasmiConfig::LazyTranslation) => "Wasmi v2 (lazy-translation)",
            Self::Wasmtime(WasmtimeConfig::Cranelift) => "Wasmtime (Cranelift)",
            Self::Wasmtime(WasmtimeConfig::Winch) => "Wasmtime (Winch)",
            Self::Wasmtime(WasmtimeConfig::Pulley) => "Wasmtime (Pulley)",
        }
    }

    /// The color of JIT-compiling Wasm runtimes.
    const BLUE: RGBColor = RGBColor(52, 119, 186);
    /// The color of most interpreter-based Wasm runtimes.
    const TEAL: RGBColor = RGBColor(76, 161, 143);
    /// The color of the Wasmi v2 interpreter.
    const ORANGE: RGBColor = RGBColor(227, 146, 63);

    /// Returns the color associated to the Wasm runtime kind.
    fn color(&self) -> RGBColor {
        match self {
            VmAndConfig::WasmiV2(_) => Self::ORANGE,
            VmAndConfig::Wasmtime(WasmtimeConfig::Pulley) => Self::TEAL,
            VmAndConfig::V8 | VmAndConfig::Wasmer(_) | VmAndConfig::Wasmtime(_) => Self::BLUE,
            _ => Self::TEAL,
        }
    }

    /// Returns the execution kind of the Wasm runtime.
    ///
    /// Derived from [`Self::color`] so it stays consistent with the plotted
    /// colors: JITs are blue, interpreters are teal or orange.
    fn kind(&self) -> RuntimeKind {
        if self.color() == Self::BLUE {
            RuntimeKind::Jit
        } else {
            RuntimeKind::Interpreter
        }
    }
}

impl FromStr for VmAndConfig {
    type Err = FromStrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let vm_and_config = match input {
            "dlr-wasm-interpreter" => Self::DlrWasmInterpreter,
            "fizzy" => Self::Fizzy,
            "spacewasm" => Self::SpaceWasm,
            "stitch" => Self::Stitch,
            "tinywasm" => Self::Tinywasm,
            "toywasm" => Self::Toywasm,
            "v8" => Self::V8,
            "wamr" => Self::Wamr,
            "wasm3.eager" => Self::Wasm3(Wasm3Config::Eager),
            "wasm3.lazy" => Self::Wasm3(Wasm3Config::Lazy),
            "wasmedge" => Self::WasmEdge,
            "wasmer.cranelift" => Self::Wasmer(WasmerConfig::Cranelift),
            "wasmer.singlepass" => Self::Wasmer(WasmerConfig::Singlepass),
            "wasmi-v0.31" => Self::Wasmi031,
            "wasmi-v0.32" => Self::Wasmi032,
            "wasmi-v1.eager.checked" => Self::WasmiV1(WasmiConfig::Checked),
            "wasmi-v1.eager.unchecked" => Self::WasmiV1(WasmiConfig::Unchecked),
            "wasmi-v1.lazy.checked" => Self::WasmiV1(WasmiConfig::Lazy),
            "wasmi-v1.lazy.unchecked" => Self::WasmiV1(WasmiConfig::LazyUnchecked),
            "wasmi-v1.lazy-translation.checked" => Self::WasmiV1(WasmiConfig::LazyTranslation),
            "wasmi-v2.eager.checked" => Self::WasmiV2(WasmiConfig::Checked),
            "wasmi-v2.eager.unchecked" => Self::WasmiV2(WasmiConfig::Unchecked),
            "wasmi-v2.lazy.checked" => Self::WasmiV2(WasmiConfig::Lazy),
            "wasmi-v2.lazy.unchecked" => Self::WasmiV2(WasmiConfig::LazyUnchecked),
            "wasmi-v2.lazy-translation.checked" => Self::WasmiV2(WasmiConfig::LazyTranslation),
            "wasmtime.cranelift" => Self::Wasmtime(WasmtimeConfig::Cranelift),
            "wasmtime.winch" => Self::Wasmtime(WasmtimeConfig::Winch),
            "wasmtime.pulley" => Self::Wasmtime(WasmtimeConfig::Pulley),
            _ => return Err(FromStrError::from(format!("invalid VmAndConfig: {input}"))),
        };
        Ok(vm_and_config)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BenchEntry {
    pub vm: VmAndConfig,
    /// The measured time, normalized to nanoseconds.
    pub time: f64,
}

impl BenchEntry {
    /// Returns the value plotted for this entry given the fastest time `min` (nanoseconds).
    ///
    /// In [`Time::Relative`] mode this is the ratio to the fastest runtime, in
    /// [`Time::Absolute`] mode it is the raw time in nanoseconds.
    fn value(&self, min: f64, time: Time) -> f64 {
        match time {
            Time::Relative => self.time / min,
            Time::Absolute => self.time,
        }
    }

    /// Returns the label drawn at the end of this entry's bar.
    fn label(&self, min: f64, time: Time) -> String {
        match time {
            Time::Relative => format!("x{:.02}", self.value(min, time)),
            Time::Absolute => format_duration_ns(self.time),
        }
    }
}

/// Converts a `estimate` given in `unit` to nanoseconds.
///
/// Criterion reports times in one of `ns`, `us`/`µs`, `ms` or `s`; anything
/// else is unexpected and treated as an error.
fn estimate_to_ns(estimate: f64, unit: &str) -> Result<f64, Box<dyn Error>> {
    let factor = match unit {
        "ns" => 1.0,
        "us" | "µs" => 1_000.0,
        "ms" => 1_000_000.0,
        "s" => 1_000_000_000.0,
        _ => return Err(FromStrError::from(format!("unexpected time unit: {unit}")).into()),
    };
    Ok(estimate * factor)
}

/// Formats a nanosecond duration adaptively as `ns`, `µs`, `ms` or `s`.
fn format_duration_ns(ns: f64) -> String {
    let (value, unit) = if ns < 1_000.0 {
        (ns, "ns")
    } else if ns < 1_000_000.0 {
        (ns / 1_000.0, "µs")
    } else if ns < 1_000_000_000.0 {
        (ns / 1_000_000.0, "ms")
    } else {
        (ns / 1_000_000_000.0, "s")
    };
    format!("{value:.02} {unit}")
}

fn plot_for_data(
    ext_title: Option<&str>,
    scale: Scale,
    time: Time,
    filter: Filter,
    bench_group: &BenchGroup,
) -> Result<(), Box<dyn Error>> {
    let mut data: Vec<_> = bench_group
        .results
        .iter()
        .filter(|&(&vm, _)| filter.keeps(vm))
        .map(|(&vm, BenchResult { estimate, unit })| {
            Ok(BenchEntry {
                vm,
                time: estimate_to_ns(*estimate, unit)?,
            })
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    if data.is_empty() {
        // No runtime of the selected kind ran in this group: nothing to plot.
        return Ok(());
    }
    let min = data
        .iter()
        .map(|entry| entry.time)
        .min_by(f64::total_cmp)
        .unwrap_or(1.0);
    let max = data
        .iter()
        .map(|entry| entry.time)
        .max_by(f64::total_cmp)
        .unwrap_or(1.0);
    // The longest bar reaches the slowest runtime's plotted value: its relative
    // time (`max / min`) in relative mode or its absolute time in absolute mode.
    let max_value = match time {
        Time::Relative => max / min,
        Time::Absolute => max,
    };
    data.sort_by_key(|lhs| lhs.time as u64);
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
    let height = 50 + 75 + 25 + 5 + data.len() as u32 * 50;
    let root = SVGBackend::new(&path, (1280, height)).into_drawing_area();
    root.fill(&color::WHITE)?;
    let root = root.margin(5, 5, 5, 5).titled(
        &test_title,
        TextStyle::from(("monospace", 45)).pos(Pos::new(HPos::Center, VPos::Center)),
    )?;
    let mut builder = ChartBuilder::on(&root);
    builder
        .x_label_area_size(75)
        .y_label_area_size(400)
        .margin_right(200)
        .margin_top(25);
    let y_axis = (0usize..data.len() - 1).into_segmented();

    // In log scaling the bars start at a lower bound below the fastest value so
    // the fastest bar stays visible: `0.5` (below a relative min of `1.0`) in
    // relative mode, or `min * 0.5` (below the absolute min) in absolute mode.
    let kind = match time {
        Time::Relative => "Relative Time",
        Time::Absolute => "Time",
    };
    let log_baseline = match time {
        Time::Relative => 0.5,
        Time::Absolute => min * 0.5,
    };

    // Log and linear scaling produce different chart coordinate types, so the
    // shared drawing logic lives in the generic `draw_chart` helper. The two
    // scales also differ in how the axis maximum is derived: the log scale is
    // floored to a full decade (`10.0`) in relative mode so it always shows a
    // complete range of gridlines, whereas the linear scale is fit tightly to
    // the data (plus a small headroom) so the bars are not squeezed into a
    // fraction of the plot. Linear scaling also starts the axis (and the bar
    // baseline) at `0.0` instead of the log baseline.
    match scale {
        Scale::Log => {
            let axis_max = match time {
                Time::Relative => core::cmp::max_by(10.0, max_value, f64::total_cmp),
                Time::Absolute => max_value,
            };
            let mut chart =
                builder.build_cartesian_2d((log_baseline..axis_max * 1.05).log_scale(), y_axis)?;
            draw_chart(
                &root,
                &mut chart,
                &data,
                min,
                time,
                log_baseline,
                &format!("{kind} (lower is better, logarithmic scale)"),
            )?;
        }
        Scale::Linear => {
            let mut chart = builder.build_cartesian_2d(0.0_f64..max_value * 1.05, y_axis)?;
            draw_chart(
                &root,
                &mut chart,
                &data,
                min,
                time,
                0.0,
                &format!("{kind} (lower is better, linear scale)"),
            )?;
        }
    }
    Ok(())
}

/// Draws the mesh, the bars and their value labels onto `chart`, then presents `root`.
///
/// This is generic over the X coordinate type so it can render both the
/// logarithmic and the linear chart produced in [`plot_for_data`].
fn draw_chart<DB, X>(
    root: &DrawingArea<DB, Shift>,
    chart: &mut ChartContext<'_, DB, Cartesian2d<X, SegmentedCoord<RangedCoordusize>>>,
    data: &[BenchEntry],
    min: f64,
    time: Time,
    baseline: f64,
    x_desc: &str,
) -> Result<(), Box<dyn Error>>
where
    DB: DrawingBackend,
    DB::ErrorType: 'static,
    X: Ranged<ValueType = f64> + ValueFormatter<f64>,
{
    // We want to draw the Wasm runtime names instead of the numbers.
    let y_label_formatter = |coord: &SegmentValue<usize>| match coord {
        SegmentValue::CenterOf(n) => data[*n].vm.label().to_string(),
        SegmentValue::Exact(_n) => unreachable!(),
        SegmentValue::Last => unreachable!(),
    };
    // In absolute mode the axis values are nanoseconds, so format the ticks
    // adaptively as ns/µs/ms/s; relative mode keeps plotters' default numbers.
    let x_label_formatter = |value: &f64| format_duration_ns(*value);

    let mut mesh = chart.configure_mesh();
    mesh.disable_y_mesh()
        .x_max_light_lines(1)
        .bold_line_style(BLACK.mix(0.15))
        .y_desc("") // WebAssembly Runtime
        .x_desc(x_desc)
        .y_label_formatter(&y_label_formatter)
        .x_label_style(("sans-serif", 20))
        .y_label_style(("sans-serif", 30))
        .axis_desc_style(("sans-serif", 35))
        .x_labels(3)
        .y_labels(data.len());
    if let Time::Absolute = time {
        mesh.x_label_formatter(&x_label_formatter);
    }
    mesh.draw()?;

    chart.draw_series(
        Histogram::horizontal(chart)
            .style_func(|x, _bar_height| match x {
                SegmentValue::Exact(n) => data[*n].vm.color().filled(),
                SegmentValue::CenterOf(_n) => unreachable!(),
                SegmentValue::Last => unreachable!(),
            })
            .margin(15)
            .baseline(baseline)
            .data(
                data.iter()
                    .enumerate()
                    .map(|(index, entry)| (index, entry.value(min, time))),
            ),
    )?;

    chart.draw_series(data.iter().enumerate().map(|(index, &entry)| {
        let value = entry.value(min, time);
        // Anchor the label at the bar's end and offset it by a fixed pixel
        // amount so the gap between bar and label is identical for every bar,
        // regardless of the runtime's value, the axis range or the scaling.
        //
        // The font size is kept below the bar thickness (bars render ~20px tall:
        // a 50px row minus the histogram's 15px margin on each side) so the label
        // sits within the bar instead of overhanging it.
        //
        // `VPos::Center` centers on the font's x-height, but the labels are
        // digits (cap-height, no descenders) whose optical center is a couple of
        // pixels higher, so nudge the label down slightly to sit on the bar's
        // vertical center.
        EmptyElement::at((value, SegmentValue::CenterOf(index)))
            + Text::new(
                entry.label(min, time),
                (10, 2),
                TextStyle::from(("monospace", 22)).pos(Pos::new(HPos::Left, VPos::Center)),
            )
    }))?;

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present()?;
    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BenchCategory {
    Execute,
    Startup,
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
            "startup" => Ok(Self::Startup),
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
            BenchCategory::Startup => "startup".fmt(f),
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

fn decode_stdin(
    ext_title: Option<&str>,
    scale: Scale,
    time: Time,
    filter: Filter,
) -> Result<(), Box<dyn Error>> {
    use serde_json as json;
    use std::io::{self, BufRead};

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
                    plot_for_data(ext_title, scale, time, filter, &bench_group)?;
                }
            }
            _ => panic!("malformed JSON input: {json:?}"),
        };
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    decode_stdin(args.title.as_deref(), args.scale, args.time, args.filter)
}
