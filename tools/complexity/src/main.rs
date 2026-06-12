use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use syn::visit::{self, Visit};
use syn::{
    BinOp, ExprBinary, ExprClosure, ExprForLoop, ExprIf, ExprLoop, ExprMatch, ExprTry, ExprWhile,
    File, ImplItemFn, ItemFn, TraitItemFn,
};
use walkdir::WalkDir;

#[derive(Debug)]
struct Args {
    paths: Vec<PathBuf>,
    top: usize,
    max: Option<usize>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct FunctionMetric {
    path: PathBuf,
    line: usize,
    name: String,
    complexity: usize,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args(env::args().skip(1))?;
    let files = collect_rust_files(&args.paths)?;
    let mut metrics = Vec::new();

    for file in &files {
        let source = fs::read_to_string(file)?;
        let syntax = syn::parse_file(&source).map_err(|err| {
            format!(
                "failed to parse {} at {}: {err}",
                file.display(),
                err.span().start().line
            )
        })?;
        metrics.extend(measure_file(file, &syntax));
    }

    metrics.sort_by(|a, b| {
        b.complexity
            .cmp(&a.complexity)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.name.cmp(&b.name))
    });

    print_report(&files, &metrics, args.top, args.max);

    if let Some(max) = args.max {
        if metrics.iter().any(|metric| metric.complexity > max) {
            process::exit(1);
        }
    }

    Ok(())
}

fn parse_args(raw_args: impl IntoIterator<Item = String>) -> Result<Args, Box<dyn Error>> {
    let mut paths = Vec::new();
    let mut top = 20;
    let mut max = None;
    let mut args = raw_args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            "--top" => {
                let value = args.next().ok_or("--top requires a value")?;
                top = value.parse()?;
            }
            "--max" => {
                let value = args.next().ok_or("--max requires a value")?;
                max = Some(value.parse()?);
            }
            value if value.starts_with("--top=") => {
                top = value["--top=".len()..].parse()?;
            }
            value if value.starts_with("--max=") => {
                max = Some(value["--max=".len()..].parse()?);
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}").into());
            }
            value => paths.push(PathBuf::from(value)),
        }
    }

    if paths.is_empty() {
        paths.push(PathBuf::from("src"));
        paths.push(PathBuf::from("tests"));
    }

    Ok(Args { paths, top, max })
}

fn print_help() {
    println!(
        "\
Measure Rust cyclomatic complexity.

Usage:
  cargo run --manifest-path tools/complexity/Cargo.toml -- [OPTIONS] [PATH ...]

Options:
      --top N    Number of highest-complexity functions to print (default: 20)
      --max N    Exit with status 1 if any function has complexity greater than N
  -h, --help     Print help

If no paths are provided, src and tests are scanned.

Metric:
  McCabe-style cyclomatic complexity: each function starts at 1. The tool adds
  one for each if, loop, while, for, try expression (?), boolean &&/||, and each
  additional match arm.
"
    );
}

fn collect_rust_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_rust_file(path) {
                files.push(path.clone());
            }
            continue;
        }

        if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                name != "target" && name != ".git"
            }) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && is_rust_file(path) {
                    files.push(path.to_path_buf());
                }
            }
            continue;
        }

        return Err(format!("path does not exist: {}", path.display()).into());
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
}

fn measure_file(path: &Path, syntax: &File) -> Vec<FunctionMetric> {
    let mut visitor = FileMetricVisitor {
        path,
        metrics: Vec::new(),
    };
    visitor.visit_file(syntax);
    visitor.metrics
}

struct FileMetricVisitor<'a> {
    path: &'a Path,
    metrics: Vec<FunctionMetric>,
}

impl FileMetricVisitor<'_> {
    fn record_function(&mut self, name: String, line: usize, body: &syn::Block) {
        let mut visitor = ComplexityVisitor { complexity: 1 };
        visitor.visit_block(body);
        self.metrics.push(FunctionMetric {
            path: self.path.to_path_buf(),
            line,
            name,
            complexity: visitor.complexity,
        });
    }
}

impl<'ast> Visit<'ast> for FileMetricVisitor<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.record_function(
            node.sig.ident.to_string(),
            node.sig.ident.span().start().line,
            &node.block,
        );
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        self.record_function(
            node.sig.ident.to_string(),
            node.sig.ident.span().start().line,
            &node.block,
        );
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        if let Some(block) = &node.default {
            self.record_function(
                node.sig.ident.to_string(),
                node.sig.ident.span().start().line,
                block,
            );
        }
    }
}

struct ComplexityVisitor {
    complexity: usize,
}

impl<'ast> Visit<'ast> for ComplexityVisitor {
    fn visit_expr_if(&mut self, node: &'ast ExprIf) {
        self.complexity += 1;
        visit::visit_expr_if(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast ExprLoop) {
        self.complexity += 1;
        visit::visit_expr_loop(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast ExprWhile) {
        self.complexity += 1;
        visit::visit_expr_while(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast ExprForLoop) {
        self.complexity += 1;
        visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        self.complexity += node.arms.len().saturating_sub(1);
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        if matches!(node.op, BinOp::And(_) | BinOp::Or(_)) {
            self.complexity += 1;
        }
        visit::visit_expr_binary(self, node);
    }

    fn visit_expr_try(&mut self, node: &'ast ExprTry) {
        self.complexity += 1;
        visit::visit_expr_try(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast ExprClosure) {
        visit::visit_expr_closure(self, node);
    }
}

fn print_report(
    files: &[PathBuf],
    metrics: &[FunctionMetric],
    top: usize,
    max_threshold: Option<usize>,
) {
    let max_complexity = metrics
        .iter()
        .map(|metric| metric.complexity)
        .max()
        .unwrap_or(0);
    let total_complexity: usize = metrics.iter().map(|metric| metric.complexity).sum();
    let average = if metrics.is_empty() {
        0.0
    } else {
        total_complexity as f64 / metrics.len() as f64
    };

    println!("Rust cyclomatic complexity");
    println!("==========================");
    println!("Files scanned: {}", files.len());
    println!("Functions:     {}", metrics.len());
    println!("Max:           {max_complexity}");
    println!("Average:       {average:.2}");

    if let Some(max) = max_threshold {
        let violations = metrics
            .iter()
            .filter(|metric| metric.complexity > max)
            .count();
        println!("Threshold:     {max}");
        println!("Violations:    {violations}");
    }

    println!();
    println!("Top {} functions:", top.min(metrics.len()));
    for metric in metrics.iter().take(top) {
        println!(
            "{:>4}  {}:{}  {}",
            metric.complexity,
            metric.path.display(),
            metric.line,
            metric.name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure_source(source: &str) -> Vec<FunctionMetric> {
        let syntax = syn::parse_file(source).expect("source should parse");
        measure_file(Path::new("sample.rs"), &syntax)
    }

    #[test]
    fn straight_line_function_has_complexity_one() {
        let metrics = measure_source("fn simple() { let value = 1; println!(\"{}\", value); }");

        assert_eq!(metrics[0].complexity, 1);
    }

    #[test]
    fn counts_common_branching_constructs() {
        let metrics = measure_source(
            r#"
            fn complex(input: Result<i32, ()>, values: &[i32]) {
                let value = input?;
                if value > 0 && value < 10 {
                    for item in values {
                        if *item == value || *item == 0 {
                            break;
                        }
                    }
                }
                match value {
                    0 => {}
                    1 => {}
                    _ => {}
                }
            }
            "#,
        );

        assert_eq!(metrics[0].complexity, 9);
    }

    #[test]
    fn records_impl_and_trait_default_methods() {
        let metrics = measure_source(
            r#"
            struct Thing;

            impl Thing {
                fn method(&self) {
                    while false {}
                }
            }

            trait Example {
                fn defaulted(&self) {
                    loop { break; }
                }

                fn required(&self);
            }
            "#,
        );

        let names: Vec<_> = metrics.iter().map(|metric| metric.name.as_str()).collect();
        assert_eq!(names, vec!["method", "defaulted"]);
        assert_eq!(metrics[0].complexity, 2);
        assert_eq!(metrics[1].complexity, 2);
    }
}
