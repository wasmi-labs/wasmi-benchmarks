use clap::Parser;
use plotters::coord::Shift;
use plotters::coord::ranged1d::{Ranged, SegmentedCoord, ValueFormatter};
use plotters::coord::types::RangedCoordusize;
use plotters::prelude::*;
use plotters::style::colors::full_palette as color;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use std::collections::{BTreeMap, BTreeSet};
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
    /// Exclude all Wasm runtimes that a newer supported version supersedes.
    Outdated,
}

impl Filter {
    /// Returns `true` if the given `vm` passes this filter.
    fn keeps(self, vm: VmAndConfig) -> bool {
        match self {
            Filter::None => true,
            Filter::Jit => vm.kind() != RuntimeKind::Jit,
            Filter::Interpreter => vm.kind() != RuntimeKind::Interpreter,
            Filter::Outdated => !vm.is_outdated(),
        }
    }
}

/// The [`Filter`]s to apply, combined conjunctively.
#[derive(Debug, Default, Clone)]
struct Filters {
    filters: Vec<Filter>,
}

impl Filters {
    /// Returns `true` if the given `vm` passes all of the filters.
    fn keeps(&self, vm: VmAndConfig) -> bool {
        self.filters.iter().all(|filter| filter.keeps(vm))
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

/// The Wasm runtime whose results are highlighted in the rendered plots.
///
/// Highlighting applies to an entire Wasm runtime, not to a single one of its
/// configurations: `wasmi-v2` highlights all of `wasmi-v2.eager.checked`,
/// `wasmi-v2.lazy.checked` and so on.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, clap::ValueEnum)]
enum Highlight {
    /// Highlight the Wasmi v2 interpreter.
    #[default]
    #[value(name = "wasmi-v2")]
    WasmiV2,
    /// Do not highlight any Wasm runtime.
    #[value(name = "none")]
    None,
    #[value(name = "fizzy")]
    Fizzy,
    #[value(name = "spacewasm")]
    SpaceWasm,
    #[value(name = "stitch")]
    Stitch,
    #[value(name = "submilli-wasm")]
    SubmilliWasm,
    #[value(name = "tinywasm")]
    Tinywasm,
    #[value(name = "toywasm")]
    Toywasm,
    #[value(name = "v8")]
    V8,
    #[value(name = "wamr")]
    Wamr,
    #[value(name = "wasm3")]
    Wasm3,
    #[value(name = "wasmedge")]
    WasmEdge,
    #[value(name = "wasmer")]
    Wasmer,
    #[value(name = "wasmi-v0.31")]
    Wasmi031,
    #[value(name = "wasmi-v0.32")]
    Wasmi032,
    #[value(name = "wasmi-v1")]
    WasmiV1,
    #[value(name = "wasmtime")]
    Wasmtime,
    #[value(name = "dlr-wasm-interpreter")]
    DlrWasmInterpreter,
    #[value(name = "silverfir-nano")]
    SilverfirNano,
    #[value(name = "wasmz")]
    Wasmz,
}

impl Highlight {
    /// Returns `true` if the given `vm` is highlighted.
    fn matches(self, vm: VmAndConfig) -> bool {
        match self {
            Highlight::None => false,
            Highlight::Fizzy => matches!(vm, VmAndConfig::Fizzy),
            Highlight::SpaceWasm => matches!(vm, VmAndConfig::SpaceWasm),
            Highlight::Stitch => matches!(vm, VmAndConfig::Stitch),
            Highlight::SubmilliWasm => matches!(vm, VmAndConfig::SubmilliWasm),
            Highlight::Tinywasm => matches!(vm, VmAndConfig::Tinywasm),
            Highlight::Toywasm => matches!(vm, VmAndConfig::Toywasm),
            Highlight::V8 => matches!(vm, VmAndConfig::V8),
            Highlight::Wamr => matches!(vm, VmAndConfig::Wamr),
            Highlight::Wasm3 => matches!(vm, VmAndConfig::Wasm3(_)),
            Highlight::WasmEdge => matches!(vm, VmAndConfig::WasmEdge),
            Highlight::Wasmer => matches!(vm, VmAndConfig::Wasmer(_)),
            Highlight::Wasmi031 => matches!(vm, VmAndConfig::Wasmi031),
            Highlight::Wasmi032 => matches!(vm, VmAndConfig::Wasmi032),
            Highlight::WasmiV1 => matches!(vm, VmAndConfig::WasmiV1(_)),
            Highlight::WasmiV2 => matches!(vm, VmAndConfig::WasmiV2(_)),
            Highlight::Wasmtime => matches!(vm, VmAndConfig::Wasmtime(_)),
            Highlight::DlrWasmInterpreter => matches!(vm, VmAndConfig::DlrWasmInterpreter),
            Highlight::SilverfirNano => matches!(vm, VmAndConfig::SilverfirNano(_)),
            Highlight::Wasmz => matches!(vm, VmAndConfig::Wasmz),
        }
    }
}

/// The rendering options shared by all plots.
#[derive(Debug, Copy, Clone)]
struct Style {
    /// Scaling of the relative-time axis.
    scale: Scale,
    /// Whether to plot relative or absolute times.
    time: Time,
    /// The Wasm runtime to highlight.
    highlight: Highlight,
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
    /// Excludes kinds of Wasm runtimes from the plots.
    ///
    /// May be given repeatedly or as a comma separated list.
    #[arg(long = "filter", value_enum, value_delimiter = ',')]
    filters: Vec<Filter>,
    /// Highlights the results of the given Wasm runtime.
    ///
    /// Use `none` to disable highlighting.
    #[arg(long, value_enum, default_value_t = Highlight::WasmiV2)]
    highlight: Highlight,
}

/// VM under test and its configuration.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmAndConfig {
    Wasmi031,
    Wasmi032,
    WasmiV1(WasmiConfig),
    WasmiV2(WasmiConfig),
    Wasmtime(WasmtimeConfig),
    Fizzy,
    SpaceWasm,
    Stitch,
    SubmilliWasm,
    Tinywasm,
    Toywasm,
    V8,
    Wamr,
    Wasm3(Wasm3Config),
    WasmEdge,
    Wasmer(WasmerConfig),
    DlrWasmInterpreter,
    SilverfirNano(SilverfirNanoConfig),
    Wasmz,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SilverfirNanoConfig {
    Jit,
    Interpreter,
}

impl VmAndConfig {
    /// Returns the label of the Wasm runtime kind.
    fn label(&self) -> &str {
        match self {
            Self::Fizzy => "Fizzy",
            Self::SpaceWasm => "SpaceWasm",
            Self::Stitch => "Stitch (lazy)",
            Self::SubmilliWasm => "Submilli-wasm",
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
            Self::DlrWasmInterpreter => "DLR-wasm-interpreter",
            Self::SilverfirNano(SilverfirNanoConfig::Jit) => "Silverfir-nano (JIT)",
            Self::SilverfirNano(SilverfirNanoConfig::Interpreter) => "Silverfir-nano (interpreter)",
            Self::Wasmz => "Wasmz",
        }
    }

    /// The color of JIT-compiling Wasm runtimes.
    const BLUE: RGBColor = RGBColor(52, 119, 186);
    /// The color of most interpreter-based Wasm runtimes.
    const TEAL: RGBColor = RGBColor(76, 161, 143);
    /// The color of the highlighted Wasm runtime.
    const ORANGE: RGBColor = RGBColor(227, 146, 63);

    /// Returns the color associated to the Wasm runtime.
    ///
    /// The runtime chosen by `highlight` is orange, all others are colored by
    /// their [`RuntimeKind`]: JITs are blue and interpreters are teal.
    fn color(&self, highlight: Highlight) -> RGBColor {
        if highlight.matches(*self) {
            return Self::ORANGE;
        }
        match self.kind() {
            RuntimeKind::Jit => Self::BLUE,
            RuntimeKind::Interpreter => Self::TEAL,
        }
    }

    /// Returns the execution kind of the Wasm runtime.
    fn kind(&self) -> RuntimeKind {
        match self {
            // Pulley is Wasmtime's interpreter, not one of its JITs.
            VmAndConfig::Wasmtime(WasmtimeConfig::Pulley) => RuntimeKind::Interpreter,
            VmAndConfig::V8
            | VmAndConfig::Wasmer(_)
            | VmAndConfig::Wasmtime(_)
            | VmAndConfig::SilverfirNano(SilverfirNanoConfig::Jit) => RuntimeKind::Jit,
            _ => RuntimeKind::Interpreter,
        }
    }

    /// Returns `true` if a newer version of this Wasm runtime is supported.
    ///
    /// Configurations of the same version are never considered newer than one
    /// another, only entire runtime versions are.
    fn is_outdated(&self) -> bool {
        matches!(
            self,
            // All superseded by `Self::WasmiV2`.
            Self::Wasmi031 | Self::Wasmi032 | Self::WasmiV1(_)
        )
    }
}

impl FromStr for VmAndConfig {
    type Err = FromStrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let vm_and_config = match input {
            "fizzy" => Self::Fizzy,
            "spacewasm" => Self::SpaceWasm,
            "stitch" => Self::Stitch,
            "submilli-wasm" => Self::SubmilliWasm,
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
            "dlr-wasm-interpreter" => Self::DlrWasmInterpreter,
            "silverfir-nano.jit" => Self::SilverfirNano(SilverfirNanoConfig::Jit),
            "silverfir-nano.interpreter" => Self::SilverfirNano(SilverfirNanoConfig::Interpreter),
            "wasmz" => Self::Wasmz,
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
    style: Style,
    filters: &Filters,
    bench_group: &BenchGroup,
) -> Result<(), Box<dyn Error>> {
    let data = bench_group.entries(filters)?;
    if data.is_empty() {
        // No runtime of the selected kind ran in this group: nothing to plot.
        return Ok(());
    }
    // Bars are plotted relative to the fastest runtime of this group.
    let min = data
        .iter()
        .map(|entry| entry.time)
        .min_by(f64::total_cmp)
        .unwrap_or(1.0);
    let kind = match style.time {
        Time::Relative => "Relative Time",
        Time::Absolute => "Time",
    };
    let category = bench_group.category;
    let name = &bench_group.name;
    render_plot(
        &plot_title(ext_title, &format!("{category}/{name}")),
        &format!("target/wasmi-benchmarks/{category}/{name}.svg"),
        style,
        kind,
        min,
        data,
    )
}

/// Appends the optional external title to the plot's `test_id`.
fn plot_title(ext_title: Option<&str>, test_id: &str) -> String {
    match ext_title {
        Some(ext_title) => format!("{test_id} - {ext_title}"),
        None => test_id.to_string(),
    }
}

/// Renders `data` as a horizontal bar chart into the SVG file at `path`.
///
/// In [`Time::Relative`] mode every bar is plotted as `entry.time / min`, so
/// `min` is the baseline the plot is relative to: the fastest runtime of a
/// benchmark group for the per-test-case plots, or `1.0` for the geomean plots
/// whose entries already are relative values.
fn render_plot(
    title: &str,
    path: &str,
    style: Style,
    kind: &str,
    min: f64,
    mut data: Vec<BenchEntry>,
) -> Result<(), Box<dyn Error>> {
    let max = data
        .iter()
        .map(|entry| entry.time)
        .max_by(f64::total_cmp)
        .unwrap_or(1.0);
    // The longest bar reaches the slowest runtime's plotted value: its relative
    // time (`max / min`) in relative mode or its absolute time in absolute mode.
    let max_value = match style.time {
        Time::Relative => max / min,
        Time::Absolute => max,
    };
    // Slowest runtime first so the bars form a descending staircase.
    data.sort_by(|lhs, rhs| rhs.time.total_cmp(&lhs.time));
    let data = &data[..];

    let _ = std::fs::create_dir_all(path);
    let _ = std::fs::remove_dir(path);
    let height = 50 + 75 + 25 + 5 + data.len() as u32 * 50;
    let root = SVGBackend::new(path, (1280, height)).into_drawing_area();
    root.fill(&color::WHITE)?;
    let root = root.margin(5, 5, 5, 5).titled(
        title,
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
    let log_baseline = match style.time {
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
    match style.scale {
        Scale::Log => {
            let axis_max = match style.time {
                Time::Relative => core::cmp::max_by(10.0, max_value, f64::total_cmp),
                Time::Absolute => max_value,
            };
            let mut chart =
                builder.build_cartesian_2d((log_baseline..axis_max * 1.05).log_scale(), y_axis)?;
            draw_chart(
                &root,
                &mut chart,
                data,
                min,
                style,
                log_baseline,
                &format!("{kind} (lower is better, logarithmic scale)"),
            )?;
        }
        Scale::Linear => {
            let mut chart = builder.build_cartesian_2d(0.0_f64..max_value * 1.05, y_axis)?;
            draw_chart(
                &root,
                &mut chart,
                data,
                min,
                style,
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
    style: Style,
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
    if let Time::Absolute = style.time {
        mesh.x_label_formatter(&x_label_formatter);
    }
    mesh.draw()?;

    chart.draw_series(
        Histogram::horizontal(chart)
            .style_func(|x, _bar_height| match x {
                SegmentValue::Exact(n) => data[*n].vm.color(style.highlight).filled(),
                SegmentValue::CenterOf(_n) => unreachable!(),
                SegmentValue::Last => unreachable!(),
            })
            .margin(15)
            .baseline(baseline)
            .data(
                data.iter()
                    .enumerate()
                    .map(|(index, entry)| (index, entry.value(min, style.time))),
            ),
    )?;

    chart.draw_series(data.iter().enumerate().map(|(index, &entry)| {
        let value = entry.value(min, style.time);
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
                entry.label(min, style.time),
                (10, 2),
                TextStyle::from(("monospace", 22)).pos(Pos::new(HPos::Left, VPos::Center)),
            )
    }))?;

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present()?;
    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl BenchGroup {
    /// Returns the measured times of all runtimes of this group that pass `filters`.
    fn entries(&self, filters: &Filters) -> Result<Vec<BenchEntry>, Box<dyn Error>> {
        self.results
            .iter()
            .filter(|&(&vm, _)| filters.keeps(vm))
            .map(|(&vm, BenchResult { estimate, unit })| {
                Ok(BenchEntry {
                    vm,
                    time: estimate_to_ns(*estimate, unit)?,
                })
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct BenchResult {
    pub estimate: f64,
    pub unit: String,
}

/// The measured times of all benchmark groups of a single [`BenchCategory`].
///
/// Collected while decoding so the geomean plot of the category can be rendered
/// once all its groups have been seen.
#[derive(Debug, Default)]
struct GeomeanData {
    /// One entry per test case: its name and the times of the runtimes that
    /// passed the [`Filters`], in nanoseconds.
    cases: Vec<(String, BTreeMap<VmAndConfig, f64>)>,
}

impl GeomeanData {
    /// Records the filtered results of `bench_group` as another test case.
    fn push_group(
        &mut self,
        filters: &Filters,
        bench_group: &BenchGroup,
    ) -> Result<(), Box<dyn Error>> {
        let times = bench_group
            .entries(filters)?
            .into_iter()
            .map(|entry| (entry.vm, entry.time))
            .collect();
        self.cases.push((bench_group.name.clone(), times));
        Ok(())
    }

    /// Returns the runtimes that appear in at least one test case.
    fn runtimes(&self) -> BTreeSet<VmAndConfig> {
        self.cases
            .iter()
            .flat_map(|(_name, times)| times.keys().copied())
            .collect()
    }
}

/// Renders the geomean plot of `category` into
/// `target/wasmi-benchmarks/geomean-{category}.svg`.
///
/// The geomean summarizes an entire category instead of being one of its test
/// cases, so it is put next to the `{category}` folders instead of into them.
///
/// Every runtime is plotted relative to a theoretical optimal runtime that
/// picks the fastest measured runtime for each test case individually, as the
/// geometric mean of its per-test-case ratios `time / optimal`. The geometric
/// mean is the correct average for such normalized ratios: it is symmetric
/// under being twice as fast or twice as slow and independent of the runtime
/// the ratios are normalized to.
///
/// Only the test cases that _every_ plotted runtime ran are averaged, so all
/// bars cover the same set of test cases and stay comparable. Note that this is
/// evaluated after `filter` has been applied: a test case that is missing only
/// runtimes that `filter` excludes anyway still contributes to the geomean.
///
/// The geomean is always plotted as a relative time, since averaging absolute
/// times across differently sized test cases is meaningless.
fn plot_geomean(
    ext_title: Option<&str>,
    style: Style,
    category: BenchCategory,
    geomean_data: &GeomeanData,
) -> Result<(), Box<dyn Error>> {
    let runtimes = geomean_data.runtimes();
    if runtimes.is_empty() {
        // No runtime of the selected kind ran at all: nothing to plot.
        return Ok(());
    }
    let mut common = Vec::new();
    for (name, times) in &geomean_data.cases {
        let missing: Vec<_> = runtimes
            .iter()
            .filter(|vm| !times.contains_key(vm))
            .map(|vm| vm.label())
            .collect();
        match missing.is_empty() {
            true => common.push(times),
            false => eprintln!(
                "{category}/geomean: excluding test case {name:?}: not run by {}",
                missing.join(", ")
            ),
        }
    }
    if common.is_empty() {
        eprintln!(
            "{category}/geomean: no test case was run by all runtimes: skipping geomean plot"
        );
        return Ok(());
    }
    eprintln!(
        "{category}/geomean: averaging {} runtimes over {} of {} test cases",
        runtimes.len(),
        common.len(),
        geomean_data.cases.len(),
    );
    // Sum the logarithms of the ratios per runtime, so that exponentiating
    // their mean yields the geometric mean of the ratios themselves.
    let mut sum_of_logs: BTreeMap<VmAndConfig, f64> =
        runtimes.iter().map(|&vm| (vm, 0.0)).collect();
    for times in common.iter() {
        let optimal = times
            .values()
            .copied()
            .min_by(f64::total_cmp)
            .expect("test case ran by all runtimes has at least one time");
        for (vm, sum) in sum_of_logs.iter_mut() {
            *sum += (times[vm] / optimal).ln();
        }
    }
    let count = common.len() as f64;
    let data: Vec<_> = sum_of_logs
        .into_iter()
        .map(|(vm, sum)| BenchEntry {
            vm,
            time: (sum / count).exp(),
        })
        .collect();
    render_plot(
        &plot_title(ext_title, &format!("{category}/geomean")),
        &format!("target/wasmi-benchmarks/geomean-{category}.svg"),
        // The geomean is always plotted as a relative time.
        Style {
            time: Time::Relative,
            ..style
        },
        "Relative Time vs. optimal runtime",
        1.0,
        data,
    )
}

fn decode_stdin(
    ext_title: Option<&str>,
    style: Style,
    filters: &Filters,
) -> Result<(), Box<dyn Error>> {
    use serde_json as json;
    use std::io::{self, BufRead};

    // Create a buffer to read input
    let stdin = io::stdin();
    let handle = stdin.lock();

    let mut bench_group: Option<BenchGroup> = None;
    // The results of all groups seen so far, needed to plot the per-category
    // geomeans once the entire input has been decoded.
    let mut geomean_data: BTreeMap<BenchCategory, GeomeanData> = BTreeMap::new();

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
                    plot_for_data(ext_title, style, filters, &bench_group)?;
                    geomean_data
                        .entry(bench_group.category)
                        .or_default()
                        .push_group(filters, &bench_group)?;
                }
            }
            _ => panic!("malformed JSON input: {json:?}"),
        };
    }
    for (category, geomean_data) in &geomean_data {
        plot_geomean(ext_title, style, *category, geomean_data)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let filters = Filters {
        filters: args.filters,
    };
    let style = Style {
        scale: args.scale,
        time: args.time,
        highlight: args.highlight,
    };
    decode_stdin(args.title.as_deref(), style, &filters)
}
