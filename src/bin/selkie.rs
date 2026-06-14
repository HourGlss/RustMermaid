//! selkie - A fast mermaid diagram renderer
//!
//! CLI interface compatible with mermaid-cli (mmdc) for easy migration.
//!
//! Usage:
//!   selkie input.mmd -o output.svg             # render is the default
//!   selkie -i input.mmd -o output.svg          # -i flag also works
//!   selkie render input.mmd -o output.svg      # explicit render subcommand
//!   selkie eval                                # evaluate with gallery samples
//!   selkie eval -o ./reports                   # custom output directory

use std::io::{self, Read, Write};
#[cfg(feature = "eval")]
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::{env, fs};

use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
#[cfg(feature = "eval")]
use uuid::Uuid;

#[cfg(feature = "eval")]
use selkie::eval::{self, runner::DiagramInput, runner::SvgPair, samples};
#[cfg(feature = "eval")]
use selkie::render::ascii as ascii_render;
use selkie::render::{RenderConfig, Theme};
use selkie::{parse, render_with_config};

/// Configuration file format (compatible with mermaid-cli)
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigFile {
    /// Theme name
    #[serde(default)]
    theme: Option<String>,
    /// Custom theme variables
    #[serde(default)]
    theme_variables: Option<ThemeVariables>,
    /// Background color
    #[serde(default)]
    background: Option<String>,
}

/// Theme variable overrides
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemeVariables {
    primary_color: Option<String>,
    primary_text_color: Option<String>,
    primary_border_color: Option<String>,
    secondary_color: Option<String>,
    tertiary_color: Option<String>,
    line_color: Option<String>,
    background: Option<String>,
    font_family: Option<String>,
}

/// A fast mermaid diagram renderer
#[derive(Parser, Debug)]
#[command(name = "selkie")]
#[command(version, about = "A fast mermaid diagram renderer")]
struct Args {
    /// Emit structured tracing span timings as JSON lines on stderr.
    ///
    /// Can also be enabled with SELKIE_TRACE=1. Traces are intentionally
    /// written to stderr so SVG/PNG/PDF/stdout output remains pipeline-safe.
    #[arg(long, global = true)]
    trace: bool,

    /// Override the tracing filter used by --trace or SELKIE_TRACE=1.
    ///
    /// Defaults to "selkie=trace". SELKIE_TRACE_FILTER can also be used.
    #[arg(long, global = true, value_name = "FILTER")]
    trace_filter: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    // Flattened render args for backwards compatibility
    // When no subcommand is given but -i is provided, run render
    #[command(flatten)]
    render: RenderArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Render a mermaid diagram to SVG/PNG/PDF
    Render(RenderArgs),
    /// Evaluate selkie against mermaid.js reference
    #[cfg(feature = "eval")]
    Eval(EvalArgs),
}

/// Arguments for the render command
#[derive(Parser, Debug, Default)]
struct RenderArgs {
    /// Input file (.mmd, .md) or - for stdin
    #[arg(value_name = "INPUT")]
    input_positional: Option<String>,

    /// Input file (.mmd, .md) or - for stdin (alternative to positional)
    #[arg(short, long, value_name = "FILE")]
    input: Option<String>,

    /// Output file (.svg) or - for stdout
    #[arg(short, long)]
    output: Option<String>,

    /// Theme for diagram colors
    #[arg(short, long, value_enum, default_value = "default")]
    theme: ThemeArg,

    /// Background color (e.g., "white", "#f0f0f0", "transparent")
    #[arg(short, long)]
    background: Option<String>,

    /// Output format (defaults to extension or svg)
    #[arg(short = 'e', long)]
    output_format: Option<OutputFormat>,

    /// Diagram width in pixels (not yet implemented)
    #[arg(short, long)]
    width: Option<u32>,

    /// Diagram height in pixels (not yet implemented)
    #[arg(short = 'H', long)]
    height: Option<u32>,

    /// Configuration file (JSON)
    #[arg(short = 'c', long)]
    config_file: Option<PathBuf>,

    /// Suppress console output
    #[arg(short, long)]
    quiet: bool,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Display diagram directly in terminal (requires kitty/ghostty)
    #[cfg(feature = "kitty")]
    #[arg(short = 'd', long)]
    display: bool,

    /// Force terminal display even if kitty support is not detected
    #[cfg(feature = "kitty")]
    #[arg(long)]
    force_display: bool,
}

/// Arguments for the eval command
#[cfg(feature = "eval")]
#[derive(Parser, Debug)]
#[command(after_help = "\
Examples:
  selkie eval                     Run with gallery samples (AI-agent friendly output)
  selkie eval -o ./reports        Output to custom directory
  selkie eval --type flowchart    Evaluate only flowchart samples
  selkie eval ./diagrams/         Evaluate .mmd files from directory
  selkie eval --brief             Compact summary output
  selkie eval --verbose           Show detailed per-diagram diffs
  selkie eval --use-repo-svgs --skip-comparison-pngs
")]
struct EvalArgs {
    /// Input to evaluate: JSON file, directory, .mmd file, or omit for gallery samples
    #[arg(value_name = "TARGET")]
    target: Option<String>,

    /// Filter by diagram type (flowchart, sequence, pie, etc.)
    #[arg(short = 't', long = "type")]
    diagram_type: Option<String>,

    /// Output directory for report (default: ./eval-report). Creates selkie-eval-XXXX subdirectory.
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,

    /// Show detailed diff per diagram (legacy format)
    #[arg(short, long)]
    verbose: bool,

    /// Compact summary output (disables default AI-agent friendly format)
    #[arg(short, long)]
    brief: bool,

    /// Clear cache and re-render all reference SVGs
    #[arg(long)]
    force_refresh: bool,

    /// Show cache location and statistics, then exit
    #[arg(long)]
    cache_info: bool,

    /// Open HTML report in default browser after evaluation
    #[arg(long)]
    open_report: bool,

    /// Use pre-committed SVGs from docs/images/reference/ instead of rendering with mmdc.
    /// Useful in CI where Playwright/Chromium may not be available.
    #[arg(long)]
    use_repo_svgs: bool,

    /// Skip generated PNG comparison artifacts.
    /// Useful for structural-only eval runs and environments without Mermaid CLI.
    #[arg(long)]
    skip_comparison_pngs: bool,

    /// Evaluate ASCII output instead of SVG (only flowchart diagrams)
    #[arg(long)]
    ascii: bool,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
enum ThemeArg {
    #[default]
    Default,
    Dark,
    Forest,
    Neutral,
    /// Auto-detect based on terminal background color
    #[cfg(feature = "kitty")]
    Auto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Svg,
    #[cfg(feature = "png")]
    Png,
    #[cfg(feature = "pdf")]
    Pdf,
    /// Character-art ASCII output
    Ascii,
}

impl OutputFormat {
    /// Detect output format from file extension
    fn from_extension(path: &str) -> Option<Self> {
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".svg") {
            Some(OutputFormat::Svg)
        } else if path_lower.ends_with(".png") {
            #[cfg(feature = "png")]
            return Some(OutputFormat::Png);
            #[cfg(not(feature = "png"))]
            return None;
        } else if path_lower.ends_with(".pdf") {
            #[cfg(feature = "pdf")]
            return Some(OutputFormat::Pdf);
            #[cfg(not(feature = "pdf"))]
            return None;
        } else {
            None
        }
    }
}

fn main() {
    let args = Args::parse();

    if let Err(e) = init_tracing(&args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn init_tracing(args: &Args) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !trace_enabled(args) {
        return Ok(());
    }

    let filter = args
        .trace_filter
        .clone()
        .or_else(|| env::var("SELKIE_TRACE_FILTER").ok())
        .unwrap_or_else(|| "selkie=trace".to_string());

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_new(filter)?)
        .with_span_events(FmtSpan::CLOSE)
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(io::stderr)
        .try_init()?;
    Ok(())
}

fn trace_enabled(args: &Args) -> bool {
    args.trace || env_truthy("SELKIE_TRACE")
}

fn env_truthy(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    match args.command {
        Some(Commands::Render(render_args)) => run_render(render_args),
        #[cfg(feature = "eval")]
        Some(Commands::Eval(eval_args)) => run_eval(eval_args),
        // Default to render when no subcommand is specified
        None => run_render(args.render),
    }
}

fn run_render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let input_path = render_input_path(&args)?;
    let input = read_input(input_path)?;
    if args.verbose {
        eprintln!("Read {} bytes from input", input.len());
    }

    let config_file = load_config_file(&args)?;
    let theme = build_render_theme(&args, config_file.as_ref());
    let config = RenderConfig {
        theme,
        ..RenderConfig::default()
    };

    // Parse the diagram
    let diagram = parse(&input).map_err(|e| format!("Parse error: {}", e))?;

    if args.verbose {
        eprintln!("Parsed diagram successfully");
    }

    if render_format(&args) == OutputFormat::Ascii {
        render_ascii_output(&diagram, &args)?;
        return Ok(());
    }

    let svg = render_with_config(&diagram, &config).map_err(|e| format!("Render error: {}", e))?;
    if args.verbose {
        eprintln!("Rendered {} bytes of SVG", svg.len());
    }

    #[cfg(feature = "kitty")]
    if render_to_terminal_if_requested(&args, &svg)? {
        return Ok(());
    }

    write_rendered_output(&args, &svg)?;
    report_created(&args);
    Ok(())
}

fn render_input_path(args: &RenderArgs) -> Result<&str, Box<dyn std::error::Error>> {
    args.input_positional
        .as_deref()
        .or(args.input.as_deref())
        .ok_or_else(|| "Input file is required. Usage: selkie <INPUT> [-o OUTPUT]".into())
}

fn load_config_file(args: &RenderArgs) -> Result<Option<ConfigFile>, Box<dyn std::error::Error>> {
    let Some(ref path) = args.config_file else {
        return Ok(None);
    };
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
    let cfg: ConfigFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;
    if args.verbose {
        eprintln!("Loaded config from {}", path.display());
    }
    Ok(Some(cfg))
}

fn build_render_theme(args: &RenderArgs, config_file: Option<&ConfigFile>) -> Theme {
    let mut theme = base_theme(args, config_file);
    apply_config_theme(&mut theme, config_file);
    if let Some(ref bg) = args.background {
        apply_background(&mut theme, bg);
    }
    theme
}

fn base_theme(args: &RenderArgs, config_file: Option<&ConfigFile>) -> Theme {
    match args.theme {
        ThemeArg::Default => config_file_theme(config_file),
        ThemeArg::Dark => Theme::dark(),
        ThemeArg::Forest => Theme::forest(),
        ThemeArg::Neutral => Theme::neutral(),
        #[cfg(feature = "kitty")]
        ThemeArg::Auto => auto_terminal_theme(args.verbose),
    }
}

fn config_file_theme(config_file: Option<&ConfigFile>) -> Theme {
    match config_file.and_then(|cfg| cfg.theme.as_deref()) {
        Some("dark") => Theme::dark(),
        Some("forest") => Theme::forest(),
        Some("neutral") => Theme::neutral(),
        _ => Theme::default(),
    }
}

#[cfg(feature = "kitty")]
fn auto_terminal_theme(verbose: bool) -> Theme {
    if selkie::kitty::is_terminal_dark() {
        if verbose {
            eprintln!("Auto-detected dark terminal, using dark theme");
        }
        Theme::dark()
    } else {
        if verbose {
            eprintln!("Auto-detected light terminal, using default theme");
        }
        Theme::default()
    }
}

fn apply_config_theme(theme: &mut Theme, config_file: Option<&ConfigFile>) {
    let Some(cfg) = config_file else {
        return;
    };
    if let Some(ref vars) = cfg.theme_variables {
        if let Some(ref c) = vars.primary_color {
            theme.primary_color = c.clone();
        }
        if let Some(ref c) = vars.primary_text_color {
            theme.primary_text_color = c.clone();
        }
        if let Some(ref c) = vars.primary_border_color {
            theme.primary_border_color = c.clone();
        }
        if let Some(ref c) = vars.secondary_color {
            theme.secondary_color = c.clone();
        }
        if let Some(ref c) = vars.tertiary_color {
            theme.tertiary_color = c.clone();
        }
        if let Some(ref c) = vars.line_color {
            theme.line_color = c.clone();
        }
        if let Some(ref c) = vars.background {
            theme.background = c.clone();
        }
        if let Some(ref f) = vars.font_family {
            theme.font_family = f.clone();
        }
    }
    if let Some(ref bg) = cfg.background {
        apply_background(theme, bg);
    }
}

fn apply_background(theme: &mut Theme, background: &str) {
    theme.background = if background == "transparent" {
        "none".to_string()
    } else {
        background.to_string()
    };
}

fn render_format(args: &RenderArgs) -> OutputFormat {
    args.output_format.unwrap_or_else(|| {
        args.output
            .as_deref()
            .and_then(|p| {
                if p == "-" {
                    None
                } else {
                    OutputFormat::from_extension(p)
                }
            })
            .unwrap_or(OutputFormat::Svg)
    })
}

fn render_ascii_output(
    diagram: &selkie::diagrams::Diagram,
    args: &RenderArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_str = selkie::render_ascii(diagram)?;
    if args.verbose {
        eprintln!("Rendered {} bytes of ASCII output", output_str.len());
    }
    write_output(&args.output, output_str.as_bytes())?;
    report_created(args);
    Ok(())
}

#[cfg(feature = "kitty")]
fn render_to_terminal_if_requested(
    args: &RenderArgs,
    svg: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !args.display && !args.force_display {
        return Ok(false);
    }
    if !args.force_display && !selkie::kitty::is_supported() {
        return Err(
            "Terminal does not support kitty graphics protocol. Use --force-display to override."
                .into(),
        );
    }
    if args.verbose {
        eprintln!("Displaying diagram in terminal using kitty graphics protocol");
    }

    let png_data = svg_to_png(svg, args.width, args.height)?;
    selkie::kitty::display_png(&png_data).map_err(|e| format!("Failed to display image: {}", e))?;
    write_terminal_output_file(args, svg, &png_data)?;
    Ok(true)
}

#[cfg(feature = "kitty")]
fn write_terminal_output_file(
    args: &RenderArgs,
    svg: &str,
    png_data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(ref output) = args.output else {
        return Ok(());
    };
    if output == "-" {
        return Ok(());
    }
    let format = args
        .output_format
        .unwrap_or_else(|| OutputFormat::from_extension(output).unwrap_or(OutputFormat::Svg));
    match format {
        OutputFormat::Svg => write_output(&Some(output.clone()), svg.as_bytes())?,
        #[cfg(feature = "png")]
        OutputFormat::Png => write_binary_output(&Some(output.clone()), png_data)?,
        #[cfg(feature = "pdf")]
        OutputFormat::Pdf => {
            let pdf_data = svg_to_pdf(svg)?;
            write_binary_output(&Some(output.clone()), &pdf_data)?;
        }
        OutputFormat::Ascii => unreachable!("ASCII format handled above"),
    }
    if !args.quiet {
        eprintln!("Created {}", output);
    }
    Ok(())
}

#[cfg(not(feature = "kitty"))]
#[allow(dead_code)]
fn render_to_terminal_if_requested(
    _args: &RenderArgs,
    _svg: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(false)
}

fn write_rendered_output(args: &RenderArgs, svg: &str) -> Result<(), Box<dyn std::error::Error>> {
    match render_format(args) {
        OutputFormat::Svg => write_output(&args.output, svg.as_bytes())?,
        #[cfg(feature = "png")]
        OutputFormat::Png => {
            let png_data = svg_to_png(svg, args.width, args.height)?;
            write_binary_output(&args.output, &png_data)?;
        }
        #[cfg(feature = "pdf")]
        OutputFormat::Pdf => {
            let pdf_data = svg_to_pdf(svg)?;
            write_binary_output(&args.output, &pdf_data)?;
        }
        OutputFormat::Ascii => unreachable!("ASCII format handled above"),
    }
    Ok(())
}

fn report_created(args: &RenderArgs) {
    if !args.quiet && args.output.as_deref() != Some("-") {
        if let Some(ref output) = args.output {
            eprintln!("Created {}", output);
        }
    }
}

#[cfg(feature = "eval")]
fn run_eval(args: EvalArgs) -> Result<(), Box<dyn std::error::Error>> {
    let cache = eval::cache::ReferenceCache::with_defaults();

    clear_eval_cache_if_requested(&args, &cache)?;
    if args.cache_info {
        print_eval_cache_info(&cache);
        return Ok(());
    }
    if args.ascii {
        return run_eval_ascii(args);
    }

    let eval_config = eval_config(&args);
    let runner = eval::runner::EvalRunner::new(eval_config, cache);
    let inputs = load_eval_inputs(args.target.as_deref(), true)?;

    if inputs.is_empty() {
        return Err("No diagrams to evaluate".into());
    }

    eprintln!("Evaluating {} diagrams...", inputs.len());

    // Run evaluation
    let result = runner.evaluate(&inputs);

    let output_dir = create_eval_output_dir(args.output.clone())?;
    write_eval_report_files(&result, &output_dir)?;
    let svg_pairs = runner.take_svg_pairs();
    if should_write_comparison_pngs(&args, svg_pairs.len()) {
        write_comparison_pngs_if_enabled(&output_dir, &svg_pairs, runner.cache());
    }
    eval::report::write_json_by_type(&result, &output_dir)?;
    print_eval_summary(&args, &result, &output_dir);

    // Print the output directory path
    let report_path = output_dir.join("index.html");
    eprintln!("Evaluation report written to: {}", report_path.display());

    if args.open_report {
        open_eval_report(&report_path);
    }

    // Exit with error code if there are failures
    if result.issue_counts.errors > 0 {
        process::exit(1);
    }

    Ok(())
}

#[cfg(feature = "eval")]
fn should_write_comparison_pngs(args: &EvalArgs, svg_pair_count: usize) -> bool {
    svg_pair_count > 0 && !args.use_repo_svgs && !args.skip_comparison_pngs
}

#[cfg(feature = "eval")]
fn clear_eval_cache_if_requested(
    args: &EvalArgs,
    cache: &eval::cache::ReferenceCache,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.force_refresh && cache.cache_dir().exists() {
        let stats = cache.stats();
        cache.clear()?;
        eprintln!(
            "Cleared {} cached files ({:.2} KB)",
            stats.count,
            stats.total_size as f64 / 1024.0,
        );
    }
    Ok(())
}

#[cfg(feature = "eval")]
fn print_eval_cache_info(cache: &eval::cache::ReferenceCache) {
    let stats = cache.stats();
    println!("Reference SVG Cache");
    println!("===================");
    println!("Location: {}", cache.cache_dir().display());
    println!("Files:    {}", stats.count);
    println!("Size:     {:.2} KB", stats.total_size as f64 / 1024.0);
    if stats.count == 0 {
        println!();
        println!("Cache is empty. Run 'selkie eval' to populate.");
    }
}

#[cfg(feature = "eval")]
fn eval_config(args: &EvalArgs) -> eval::runner::EvalConfig {
    #[cfg(feature = "png")]
    let skip_visual = false;
    #[cfg(not(feature = "png"))]
    let skip_visual = true;

    eval::runner::EvalConfig {
        diagram_type_filter: args.diagram_type.clone(),
        skip_visual: skip_visual || args.use_repo_svgs,
        use_repo_svgs: args.use_repo_svgs,
        ..Default::default()
    }
}

#[cfg(feature = "eval")]
fn load_eval_inputs(
    target: Option<&str>,
    allow_json: bool,
) -> Result<Vec<DiagramInput>, Box<dyn std::error::Error>> {
    match target {
        None => {
            eprintln!("Using gallery samples (docs/sources/ + embedded)...");
            Ok(samples::all_samples_owned()
                .into_iter()
                .map(DiagramInput::from)
                .collect())
        }
        Some(target) => load_eval_target(target, allow_json),
    }
}

#[cfg(feature = "eval")]
fn load_eval_target(
    target: &str,
    allow_json: bool,
) -> Result<Vec<DiagramInput>, Box<dyn std::error::Error>> {
    let path = PathBuf::from(target);
    if path.is_dir() {
        return load_directory(&path);
    }
    if allow_json && target.ends_with(".json") {
        return load_json_diagrams(&path);
    }
    load_single_diagram(&path, target)
}

#[cfg(feature = "eval")]
fn load_single_diagram(
    path: &Path,
    target: &str,
) -> Result<Vec<DiagramInput>, Box<dyn std::error::Error>> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", target, e))?;
    Ok(vec![DiagramInput {
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "diagram".to_string()),
        source: Some(target.to_string()),
        diagram_type: None,
        text: content,
    }])
}

#[cfg(feature = "eval")]
fn create_eval_output_dir(
    base_dir: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let base_dir = base_dir.unwrap_or_else(|| PathBuf::from("./eval-report"));
    let random_id = &Uuid::new_v4().to_string()[..8];
    let output_dir = base_dir.join(format!("selkie-eval-{}", random_id));
    fs::create_dir_all(&output_dir)?;
    Ok(output_dir)
}

#[cfg(feature = "eval")]
fn write_eval_report_files(
    result: &eval::EvalResult,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    eprint!("Writing HTML report...");
    eval::report::write_html(result, &output_dir.join("index.html"))?;
    eprintln!(" done");

    eprint!("Writing SVG files...");
    write_eval_svgs(result, output_dir)?;
    eprintln!(" done");
    Ok(())
}

#[cfg(feature = "eval")]
fn write_eval_svgs(
    result: &eval::EvalResult,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let docs_images = Path::new("docs/images");
    let docs_images_ref = Path::new("docs/images/reference");
    let write_to_docs = docs_images.exists() && docs_images_ref.exists();

    for diagram in &result.diagrams {
        let type_dir = output_dir.join(&diagram.diagram_type);
        let safe_name = diagram.name.replace(['/', ' '], "_");
        if diagram.selkie_svg.is_some() || diagram.reference_svg.is_some() {
            fs::create_dir_all(&type_dir)?;
        }
        write_eval_svg_pair(diagram, &type_dir, &safe_name, write_to_docs)?;
    }
    Ok(())
}

#[cfg(feature = "eval")]
fn write_eval_svg_pair(
    diagram: &eval::DiagramResult,
    type_dir: &Path,
    safe_name: &str,
    write_to_docs: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref svg) = diagram.selkie_svg {
        fs::write(type_dir.join(format!("{}_selkie.svg", safe_name)), svg)?;
        if write_to_docs {
            fs::write(
                Path::new("docs/images").join(format!("{}.svg", safe_name)),
                svg,
            )?;
        }
    }
    if let Some(ref svg) = diagram.reference_svg {
        fs::write(type_dir.join(format!("{}_reference.svg", safe_name)), svg)?;
        if write_to_docs {
            fs::write(
                Path::new("docs/images/reference").join(format!("{}.svg", safe_name)),
                svg,
            )?;
        }
    }
    Ok(())
}

#[cfg(feature = "eval")]
fn write_comparison_pngs_if_enabled(
    output_dir: &Path,
    svg_pairs: &[SvgPair],
    cache: &eval::cache::ReferenceCache,
) {
    #[cfg(feature = "png")]
    if !svg_pairs.is_empty() {
        eprint!(
            "Generating comparison PNGs ({} diagrams)...",
            svg_pairs.len()
        );
        match eval::png::write_comparison_pngs(output_dir, svg_pairs, cache) {
            Ok(_) => eprintln!(" done"),
            Err(e) => {
                eprintln!(" failed");
                eprintln!("Warning: Failed to generate comparison PNGs: {}", e);
            }
        }
    }
    #[cfg(not(feature = "png"))]
    let _ = (output_dir, svg_pairs, cache);
}

#[cfg(feature = "eval")]
fn print_eval_summary(args: &EvalArgs, result: &eval::EvalResult, output_dir: &Path) {
    if args.brief {
        eprintln!("{}", eval::report::text_summary(result, Some(output_dir)));
    } else if args.verbose {
        eprintln!("{}", eval::report::text_detailed(result, Some(output_dir)));
    } else {
        eprintln!(
            "{}",
            eval::report::text_agent_friendly(result, Some(output_dir))
        );
    }
}

#[cfg(feature = "eval")]
fn open_eval_report(report_path: &Path) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(report_path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(report_path)
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &report_path.to_string_lossy()])
            .spawn();
    }
}

/// Run ASCII-specific evaluation: parse → layout → render ASCII → parse ASCII → check
#[cfg(feature = "eval")]
fn run_eval_ascii(args: EvalArgs) -> Result<(), Box<dyn std::error::Error>> {
    use selkie::layout::CharacterSizeEstimator;

    let inputs = load_eval_inputs(args.target.as_deref(), false)?;
    let ascii_diagrams: Vec<_> = inputs
        .iter()
        .filter(|input| is_ascii_eval_candidate(input, args.diagram_type.as_deref()))
        .collect();

    if ascii_diagrams.is_empty() {
        return Err("No ASCII-supported diagrams to evaluate".into());
    }

    eprintln!(
        "Evaluating {} diagrams in ASCII mode...",
        ascii_diagrams.len()
    );

    let estimator = CharacterSizeEstimator::default();
    let mut totals = AsciiEvalTotals::default();

    for (i, input) in ascii_diagrams.iter().enumerate() {
        eprint!(
            "\rEvaluating {}/{}: {}...",
            i + 1,
            ascii_diagrams.len(),
            input.name
        );

        totals.merge(evaluate_ascii_input(input, &estimator, args.verbose)?);
    }
    eprintln!();
    print_ascii_eval_summary(&totals);
    if totals.errors > 0 {
        process::exit(1);
    }

    Ok(())
}

#[cfg(feature = "eval")]
const ASCII_SUPPORTED_TYPES: &[&str] = &[
    "flowchart",
    "sequence",
    "state",
    "class",
    "er",
    "architecture",
    "requirement",
    "mindmap",
    "pie",
    "gantt",
    "journey",
    "timeline",
    "kanban",
    "packet",
    "xychart",
    "quadrant",
    "radar",
    "git",
    "sankey",
    "block",
    "c4",
    "treemap",
];

#[cfg(feature = "eval")]
#[derive(Default)]
struct AsciiEvalTotals {
    issues: usize,
    errors: usize,
    diagrams: usize,
    similarity: f64,
}

#[cfg(feature = "eval")]
impl AsciiEvalTotals {
    fn merge(&mut self, other: Self) {
        self.issues += other.issues;
        self.errors += other.errors;
        self.diagrams += other.diagrams;
        self.similarity += other.similarity;
    }

    fn average_similarity(&self) -> f64 {
        if self.diagrams > 0 {
            self.similarity / self.diagrams as f64
        } else {
            0.0
        }
    }
}

#[cfg(feature = "eval")]
fn is_ascii_eval_candidate(input: &DiagramInput, filter: Option<&str>) -> bool {
    if let Some(filter) = filter {
        input.diagram_type.as_deref() == Some(filter)
            || detect_diagram_type(&input.text) == Some(filter)
    } else if let Some(ref diagram_type) = input.diagram_type {
        ASCII_SUPPORTED_TYPES.contains(&diagram_type.as_str())
    } else {
        detect_diagram_type(&input.text).is_some_and(|t| ASCII_SUPPORTED_TYPES.contains(&t))
    }
}

#[cfg(feature = "eval")]
fn evaluate_ascii_input(
    input: &DiagramInput,
    estimator: &selkie::layout::CharacterSizeEstimator,
    verbose: bool,
) -> Result<AsciiEvalTotals, Box<dyn std::error::Error>> {
    let parsed = match selkie::parse(&input.text) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!(" PARSE ERROR: {}", e);
            return Ok(ascii_error_total());
        }
    };

    match evaluate_special_ascii_diagram(input, &parsed, verbose) {
        Ok(Some(result)) => return Ok(result),
        Ok(None) => {}
        Err(e) => {
            eprintln!(" RENDER ERROR: {}", e);
            return Ok(ascii_error_total());
        }
    }
    match evaluate_simple_ascii_diagram(input, &parsed, verbose) {
        Ok(Some(result)) => return Ok(result),
        Ok(None) => {}
        Err(e) => {
            eprintln!(" RENDER ERROR: {}", e);
            return Ok(ascii_error_total());
        }
    }
    evaluate_graph_ascii_diagram(input, &parsed, estimator, verbose)
}

#[cfg(feature = "eval")]
fn ascii_error_total() -> AsciiEvalTotals {
    AsciiEvalTotals {
        diagrams: 1,
        errors: 1,
        ..Default::default()
    }
}

#[cfg(feature = "eval")]
fn evaluate_special_ascii_diagram(
    input: &DiagramInput,
    parsed: &selkie::diagrams::Diagram,
    verbose: bool,
) -> Result<Option<AsciiEvalTotals>, Box<dyn std::error::Error>> {
    use selkie::eval::ascii_checks;

    match parsed {
        selkie::diagrams::Diagram::Pie(db) => {
            let ascii_output = ascii_render::pie::render_pie_ascii(db)?;
            let issues = ascii_checks::check_ascii_pie_structure(&ascii_output, db);
            let similarity = ascii_checks::calculate_ascii_pie_similarity(&ascii_output, db);
            Ok(Some(ascii_issue_total(input, issues, similarity, verbose)))
        }
        selkie::diagrams::Diagram::Sequence(db) => {
            let ascii_output = ascii_render::render_sequence_ascii(db)?;
            let ascii_struct = ascii_checks::parse_ascii_sequence(&ascii_output);
            let issues = ascii_checks::check_ascii_sequence_structure(&ascii_struct, db);
            let similarity = ascii_checks::calculate_ascii_sequence_similarity(&ascii_struct, db);
            Ok(Some(ascii_issue_total(input, issues, similarity, verbose)))
        }
        selkie::diagrams::Diagram::Gantt(db) => {
            let mut db_clone = db.clone();
            let ascii_output = ascii_render::gantt::render_gantt_ascii(&mut db_clone)?;
            let issues = ascii_checks::check_ascii_gantt_structure(&ascii_output, &mut db_clone);
            let similarity =
                ascii_checks::calculate_ascii_gantt_similarity(&ascii_output, &mut db_clone);
            Ok(Some(ascii_issue_total(input, issues, similarity, verbose)))
        }
        selkie::diagrams::Diagram::Mindmap(db) => {
            let ascii_output = ascii_render::mindmap::render_mindmap_ascii(db)?;
            let issues = ascii_checks::check_ascii_mindmap_structure(&ascii_output, db);
            let similarity = ascii_checks::calculate_ascii_mindmap_similarity(&ascii_output, db);
            Ok(Some(ascii_issue_total(input, issues, similarity, verbose)))
        }
        _ => Ok(None),
    }
}

#[cfg(feature = "eval")]
fn evaluate_simple_ascii_diagram(
    input: &DiagramInput,
    parsed: &selkie::diagrams::Diagram,
    verbose: bool,
) -> Result<Option<AsciiEvalTotals>, Box<dyn std::error::Error>> {
    use selkie::eval::ascii_checks;

    let rendered = simple_ascii_output(parsed)?;
    let Some((diagram_type, output)) = rendered else {
        return Ok(None);
    };
    let issues = ascii_checks::check_ascii_text_output(&output, diagram_type);
    let similarity = ascii_checks::calculate_ascii_text_similarity(&output);
    Ok(Some(ascii_issue_total(input, issues, similarity, verbose)))
}

#[cfg(feature = "eval")]
fn simple_ascii_output(
    parsed: &selkie::diagrams::Diagram,
) -> Result<Option<(&'static str, String)>, Box<dyn std::error::Error>> {
    if let Some(result) = simple_ascii_output_primary(parsed)? {
        return Ok(Some(result));
    }
    simple_ascii_output_secondary(parsed)
}

#[cfg(feature = "eval")]
fn simple_ascii_output_primary(
    parsed: &selkie::diagrams::Diagram,
) -> Result<Option<(&'static str, String)>, Box<dyn std::error::Error>> {
    Ok(match parsed {
        selkie::diagrams::Diagram::Journey(db) => {
            Some(("journey", ascii_render::journey::render_journey_ascii(db)?))
        }
        selkie::diagrams::Diagram::Timeline(db) => Some((
            "timeline",
            ascii_render::timeline::render_timeline_ascii(db)?,
        )),
        selkie::diagrams::Diagram::Kanban(db) => {
            Some(("kanban", ascii_render::kanban::render_kanban_ascii(db)?))
        }
        selkie::diagrams::Diagram::Packet(db) => {
            Some(("packet", ascii_render::packet::render_packet_ascii(db)?))
        }
        selkie::diagrams::Diagram::XyChart(db) => {
            Some(("xychart", ascii_render::xychart::render_xychart_ascii(db)?))
        }
        selkie::diagrams::Diagram::Quadrant(db) => Some((
            "quadrant",
            ascii_render::quadrant::render_quadrant_ascii(db)?,
        )),
        _ => None,
    })
}

#[cfg(feature = "eval")]
fn simple_ascii_output_secondary(
    parsed: &selkie::diagrams::Diagram,
) -> Result<Option<(&'static str, String)>, Box<dyn std::error::Error>> {
    Ok(match parsed {
        selkie::diagrams::Diagram::Radar(db) => {
            Some(("radar", ascii_render::radar::render_radar_ascii(db)?))
        }
        selkie::diagrams::Diagram::Git(db) => {
            Some(("git", ascii_render::gitgraph::render_gitgraph_ascii(db)?))
        }
        selkie::diagrams::Diagram::Sankey(db) => {
            Some(("sankey", ascii_render::sankey::render_sankey_ascii(db)?))
        }
        selkie::diagrams::Diagram::Block(db) => {
            Some(("block", ascii_render::block::render_block_ascii(db)?))
        }
        selkie::diagrams::Diagram::C4(db) => Some(("c4", ascii_render::c4::render_c4_ascii(db)?)),
        selkie::diagrams::Diagram::Treemap(db) => {
            Some(("treemap", ascii_render::treemap::render_treemap_ascii(db)?))
        }
        _ => None,
    })
}

#[cfg(feature = "eval")]
fn evaluate_graph_ascii_diagram(
    input: &DiagramInput,
    parsed: &selkie::diagrams::Diagram,
    estimator: &selkie::layout::CharacterSizeEstimator,
    verbose: bool,
) -> Result<AsciiEvalTotals, Box<dyn std::error::Error>> {
    use selkie::eval::ascii_checks;

    let graph = match layout_diagram(parsed, estimator).and_then(selkie::layout::layout) {
        Ok(graph) => graph,
        Err(e) => {
            eprintln!(" LAYOUT ERROR: {}", e);
            return Ok(ascii_error_total());
        }
    };
    let ascii_output = match selkie::render_ascii(parsed) {
        Ok(output) => output,
        Err(e) => {
            eprintln!(" RENDER ERROR: {}", e);
            return Ok(ascii_error_total());
        }
    };

    let ascii_struct = ascii_checks::parse_ascii(&ascii_output);
    let mut issues = ascii_checks::check_ascii_structure(&ascii_struct, &graph);
    if let selkie::diagrams::Diagram::Er(db) = parsed {
        issues.extend(ascii_checks::check_er_ascii_structure(&ascii_struct, db));
    }
    if matches!(parsed, selkie::diagrams::Diagram::State(_)) {
        issues.extend(ascii_checks::check_ascii_state_structure(
            &ascii_struct,
            &graph,
        ));
    }
    let similarity = ascii_checks::calculate_ascii_similarity(&ascii_struct, &graph);
    Ok(ascii_issue_total(input, issues, similarity, verbose))
}

#[cfg(feature = "eval")]
fn ascii_issue_total(
    input: &DiagramInput,
    issues: Vec<eval::Issue>,
    similarity: f64,
    verbose: bool,
) -> AsciiEvalTotals {
    let error_count = issues
        .iter()
        .filter(|issue| issue.level == eval::Level::Error)
        .count();
    let warning_count = issues
        .iter()
        .filter(|issue| issue.level == eval::Level::Warning)
        .count();
    if verbose && !issues.is_empty() {
        print_ascii_issues(input, &issues, error_count, warning_count, similarity);
    }
    AsciiEvalTotals {
        issues: issues.len(),
        errors: error_count,
        diagrams: 1,
        similarity,
    }
}

#[cfg(feature = "eval")]
fn print_ascii_issues(
    input: &DiagramInput,
    issues: &[eval::Issue],
    error_count: usize,
    warning_count: usize,
    similarity: f64,
) {
    eprintln!();
    eprintln!(
        "  {} ({} errors, {} warnings, similarity: {:.1}%):",
        input.name,
        error_count,
        warning_count,
        similarity * 100.0
    );
    for issue in issues {
        eprintln!(
            "    [{}] {}: {}",
            issue_level_name(issue.level),
            issue.check,
            issue.message
        );
    }
}

#[cfg(feature = "eval")]
fn issue_level_name(level: eval::Level) -> &'static str {
    match level {
        eval::Level::Error => "ERROR",
        eval::Level::Warning => "WARN",
        eval::Level::Info => "INFO",
    }
}

#[cfg(feature = "eval")]
fn print_ascii_eval_summary(totals: &AsciiEvalTotals) {
    eprintln!("ASCII Evaluation Summary");
    eprintln!("======================");
    eprintln!("Diagrams:   {}", totals.diagrams);
    eprintln!("Issues:     {} ({} errors)", totals.issues, totals.errors);
    eprintln!(
        "Similarity: {:.1}% avg",
        totals.average_similarity() * 100.0
    );
}

/// Load diagrams from a directory of .mmd files
#[cfg(feature = "eval")]
fn load_directory(dir: &Path) -> Result<Vec<DiagramInput>, Box<dyn std::error::Error>> {
    let pattern = dir.join("**/*.mmd").to_string_lossy().to_string();
    let mut inputs = Vec::new();

    for entry in glob::glob(&pattern)? {
        let path = entry?;
        let content = fs::read_to_string(&path)?;
        inputs.push(DiagramInput {
            name: path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "diagram".to_string()),
            source: Some(path.to_string_lossy().to_string()),
            diagram_type: None,
            text: content,
        });
    }

    Ok(inputs)
}

/// Load diagrams from JSON file (extract_diagrams output format)
#[cfg(feature = "eval")]
fn load_json_diagrams(path: &PathBuf) -> Result<Vec<DiagramInput>, Box<dyn std::error::Error>> {
    #[derive(Deserialize)]
    struct JsonDiagram {
        name: Option<String>,
        #[serde(alias = "type")]
        diagram_type: Option<String>,
        source: String,
    }

    let content = fs::read_to_string(path)?;
    let diagrams: Vec<JsonDiagram> = serde_json::from_str(&content)?;

    Ok(diagrams
        .into_iter()
        .enumerate()
        .map(|(i, d)| DiagramInput {
            name: d.name.unwrap_or_else(|| format!("diagram_{}", i)),
            source: Some(path.to_string_lossy().to_string()),
            diagram_type: d.diagram_type,
            text: d.source,
        })
        .collect())
}

fn read_input(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    if input == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        Ok(fs::read_to_string(input)?)
    }
}

fn write_output(output: &Option<String>, content: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    match output.as_deref() {
        Some("-") | None => {
            io::stdout().write_all(content)?;
        }
        Some(path) => {
            fs::write(path, content)?;
        }
    }
    Ok(())
}

#[cfg(any(feature = "png", feature = "pdf"))]
fn write_binary_output(
    output: &Option<String>,
    content: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    match output.as_deref() {
        Some("-") | None => {
            io::stdout().write_all(content)?;
        }
        Some(path) => {
            fs::write(path, content)?;
        }
    }
    Ok(())
}

/// Convert SVG string to PNG bytes using resvg
#[cfg(feature = "png")]
fn svg_to_png(
    svg: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use resvg::tiny_skia;
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    let fontdb = opt.fontdb_mut();
    fontdb.load_system_fonts();

    // Set default font families to use when specified fonts aren't found
    // This ensures text renders even if "trebuchet ms" isn't available
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_serif_family("Times New Roman");
    fontdb.set_monospace_family("Courier New");

    // Parse SVG
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Calculate dimensions
    let svg_size = tree.size();
    let (target_width, target_height) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let scale = w as f32 / svg_size.width();
            (w, (svg_size.height() * scale) as u32)
        }
        (None, Some(h)) => {
            let scale = h as f32 / svg_size.height();
            ((svg_size.width() * scale) as u32, h)
        }
        (None, None) => (svg_size.width() as u32, svg_size.height() as u32),
    };

    // Create pixmap
    let mut pixmap =
        tiny_skia::Pixmap::new(target_width, target_height).ok_or("Failed to create pixmap")?;

    // Calculate transform to fit
    let scale_x = target_width as f32 / svg_size.width();
    let scale_y = target_height as f32 / svg_size.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

    // Render
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Encode to PNG
    let png_data = pixmap
        .encode_png()
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(png_data)
}

/// Convert SVG string to PDF bytes using svg2pdf
#[cfg(feature = "pdf")]
fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use resvg::usvg;

    // Set up options with font database
    let mut opt = usvg::Options::default();
    let fontdb = opt.fontdb_mut();
    fontdb.load_system_fonts();

    // Set default font families to use when specified fonts aren't found
    // This ensures text renders even if "trebuchet ms" isn't available
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_serif_family("Times New Roman");
    fontdb.set_monospace_family("Courier New");

    // Parse SVG
    let tree =
        usvg::Tree::from_str(svg, &opt).map_err(|e| format!("Failed to parse SVG: {}", e))?;

    // Convert to PDF
    let pdf_data = svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| format!("Failed to convert to PDF: {}", e))?;

    Ok(pdf_data)
}

/// Detect diagram type from raw mermaid text.
#[cfg(feature = "eval")]
fn detect_diagram_type(text: &str) -> Option<&str> {
    let lower = text.trim().to_lowercase();
    if lower.starts_with("flowchart") || lower.starts_with("graph ") {
        Some("flowchart")
    } else if lower.starts_with("statediagram") {
        Some("state")
    } else if lower.starts_with("classdiagram") || lower.starts_with("class") {
        Some("class")
    } else if lower.starts_with("erdiagram") {
        Some("er")
    } else if lower.starts_with("architecture") {
        Some("architecture")
    } else if lower.starts_with("requirement") {
        Some("requirement")
    } else if lower.starts_with("sequencediagram") || lower.starts_with("sequence") {
        Some("sequence")
    } else if lower.starts_with("gantt") {
        Some("gantt")
    } else if lower.starts_with("mindmap") {
        Some("mindmap")
    } else if lower.starts_with("pie") {
        Some("pie")
    } else {
        None
    }
}

/// Get a LayoutGraph from any diagram type that implements ToLayoutGraph.
#[cfg(feature = "eval")]
fn layout_diagram(
    diagram: &selkie::diagrams::Diagram,
    estimator: &selkie::layout::CharacterSizeEstimator,
) -> selkie::error::Result<selkie::layout::LayoutGraph> {
    use selkie::layout::ToLayoutGraph;

    match diagram {
        selkie::diagrams::Diagram::Flowchart(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::State(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Class(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Er(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Architecture(db) => db.to_layout_graph(estimator),
        selkie::diagrams::Diagram::Requirement(db) => db.to_layout_graph(estimator),
        _ => Err(selkie::error::MermaidError::RenderError(
            "Diagram type does not support layout graph".to_string(),
        )),
    }
}

#[cfg(all(test, feature = "eval"))]
mod eval_cli_tests {
    use super::*;

    fn eval_args() -> EvalArgs {
        EvalArgs {
            target: None,
            diagram_type: Some("flowchart".to_string()),
            output: None,
            verbose: false,
            brief: true,
            force_refresh: false,
            cache_info: false,
            open_report: false,
            use_repo_svgs: false,
            skip_comparison_pngs: false,
            ascii: false,
        }
    }

    #[test]
    fn use_repo_svgs_skips_comparison_pngs() {
        let mut args = eval_args();
        args.use_repo_svgs = true;

        assert!(!should_write_comparison_pngs(&args, 1));
    }

    #[test]
    fn explicit_flag_skips_comparison_pngs() {
        let mut args = eval_args();
        args.skip_comparison_pngs = true;

        assert!(!should_write_comparison_pngs(&args, 1));
    }

    #[test]
    fn png_comparisons_run_when_available_and_not_skipped() {
        let args = eval_args();

        assert!(should_write_comparison_pngs(&args, 1));
        assert!(!should_write_comparison_pngs(&args, 0));
    }
}
