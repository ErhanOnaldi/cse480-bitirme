use std::time::Duration;
use std::io::Write;

use crate::instances::Instance;
use crate::exact_compare::{exact_reference, gap_percent};
use crate::tabu::{tabu_search, TabuParams};

#[derive(Clone, Debug)]
pub struct RunSummary {
    pub instance_name: String,
    pub mean_obj: f64,
    pub best_obj: usize,
    pub std_obj: f64,
    pub mean_time_s: f64,
    pub best_time_s: f64,
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / (values.len() as f64)
}

fn pstdev(values: &[f64]) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }
    let m = mean(values);
    let var = values.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (values.len() as f64);
    var.sqrt()
}

pub fn run_instance(
    instance: &Instance,
    runs: u32,
    seed0: u64,
    params: TabuParams,
) -> (RunSummary, Vec<usize>, Vec<Duration>) {
    let mut objs: Vec<usize> = Vec::with_capacity(runs as usize);
    let mut times: Vec<Duration> = Vec::with_capacity(runs as usize);

    for r in 0..runs {
        let res = tabu_search(instance, seed0 + (r as u64), params);
        objs.push(res.best_bins);
        times.push(res.elapsed);
    }

    let objs_f: Vec<f64> = objs.iter().map(|&v| v as f64).collect();
    let times_f: Vec<f64> = times.iter().map(|t| t.as_secs_f64()).collect();

    let summary = RunSummary {
        instance_name: instance.name.clone(),
        mean_obj: mean(&objs_f),
        best_obj: *objs.iter().min().unwrap(),
        std_obj: pstdev(&objs_f),
        mean_time_s: mean(&times_f),
        best_time_s: times_f
            .iter()
            .copied()
            .reduce(f64::min)
            .unwrap_or(0.0),
    };

    (summary, objs, times)
}

pub fn run_instance_verbose<W: Write>(
    instance: &Instance,
    runs: u32,
    seed0: u64,
    params: TabuParams,
    out: &mut W,
) -> (RunSummary, Vec<usize>, Vec<Duration>) {
    let mut objs: Vec<usize> = Vec::with_capacity(runs as usize);
    let mut times: Vec<Duration> = Vec::with_capacity(runs as usize);

    writeln!(out, "instance={} capacity={} n={}", instance.name, instance.capacity, instance.sizes.len()).ok();
    for r in 0..runs {
        let seed = seed0 + (r as u64);
        writeln!(out, "  run {}/{} seed={}", r + 1, runs, seed).ok();
        out.flush().ok();

        let res = tabu_search(instance, seed, params);
        objs.push(res.best_bins);
        times.push(res.elapsed);

        writeln!(
            out,
            "    result: bins={} unused={} time={:.4}s iters={}",
            res.best_bins,
            res.best_unused,
            res.elapsed.as_secs_f64(),
            res.iters
        )
        .ok();
        out.flush().ok();
    }

    let objs_f: Vec<f64> = objs.iter().map(|&v| v as f64).collect();
    let times_f: Vec<f64> = times.iter().map(|t| t.as_secs_f64()).collect();

    let summary = RunSummary {
        instance_name: instance.name.clone(),
        mean_obj: mean(&objs_f),
        best_obj: *objs.iter().min().unwrap(),
        std_obj: pstdev(&objs_f),
        mean_time_s: mean(&times_f),
        best_time_s: times_f
            .iter()
            .copied()
            .reduce(f64::min)
            .unwrap_or(0.0),
    };

    (summary, objs, times)
}

pub fn format_table(rows: &[RunSummary]) -> String {
    let header = format!(
        "{:<18}{:>10}{:>10}{:>10}{:>14}{:>14}",
        "Instance", "Mean Obj", "Best Obj", "Std Dev", "Mean Time(s)", "Best Time(s)"
    );
    let mut out = String::new();
    out.push_str(&header);
    out.push('\n');
    out.push_str(&"-".repeat(header.len()));
    out.push('\n');

    for r in rows {
        out.push_str(&format!(
            "{:<18}{:>10.2}{:>10}{:>10.2}{:>14.4}{:>14.4}\n",
            r.instance_name, r.mean_obj, r.best_obj, r.std_obj, r.mean_time_s, r.best_time_s
        ));
    }
    out
}

#[derive(Clone, Debug)]
pub struct ExactGapSummary {
    pub instance_name: String,
    pub exact_bins: Option<usize>,
    pub mean_obj: f64,
    pub best_obj: usize,
    pub std_obj: f64,
    pub mean_time_s: f64,
    pub best_time_s: f64,
    pub gap_per_run: Vec<f64>,
}

pub fn format_exact_gap_table(rows: &[ExactGapSummary]) -> String {
    let header = format!(
        "{:<18}{:>7}{:>10}{:>10}{:>10}{:>14}{:>14}  {}",
        "Instance",
        "Exact",
        "Mean",
        "Best",
        "StdDev",
        "Mean Time(s)",
        "Best Time(s)",
        "Gap% (runs)"
    );
    let mut out = String::new();
    out.push_str(&header);
    out.push('\n');
    out.push_str(&"-".repeat(header.len()));
    out.push('\n');

    for r in rows {
        let exact = r.exact_bins.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
        let gaps = if r.gap_per_run.is_empty() {
            "-".to_string()
        } else {
            r.gap_per_run
                .iter()
                .map(|g| format!("{g:.2}"))
                .collect::<Vec<_>>()
                .join(",")
        };

        out.push_str(&format!(
            "{:<18}{:>7}{:>10.2}{:>10}{:>10.2}{:>14.4}{:>14.4}  {}\n",
            r.instance_name,
            exact,
            r.mean_obj,
            r.best_obj,
            r.std_obj,
            r.mean_time_s,
            r.best_time_s,
            gaps
        ));
    }
    out
}

pub fn run_instance_with_exact(
    instance: &Instance,
    runs: u32,
    seed0: u64,
    params: TabuParams,
) -> ExactGapSummary {
    let mut objs: Vec<usize> = Vec::with_capacity(runs as usize);
    let mut times: Vec<Duration> = Vec::with_capacity(runs as usize);
    for r in 0..runs {
        let res = tabu_search(instance, seed0 + (r as u64), params);
        objs.push(res.best_bins);
        times.push(res.elapsed);
    }

    let objs_f: Vec<f64> = objs.iter().map(|&v| v as f64).collect();
    let times_f: Vec<f64> = times.iter().map(|t| t.as_secs_f64()).collect();

    let exact = exact_reference(instance);
    let gap_per_run = match exact.as_ref() {
        Some(ex) => objs.iter().map(|&b| gap_percent(b, ex.bins)).collect(),
        None => Vec::new(),
    };

    ExactGapSummary {
        instance_name: instance.name.clone(),
        exact_bins: exact.map(|e| e.bins),
        mean_obj: mean(&objs_f),
        best_obj: *objs.iter().min().unwrap(),
        std_obj: pstdev(&objs_f),
        mean_time_s: mean(&times_f),
        best_time_s: times_f.iter().copied().reduce(f64::min).unwrap_or(0.0),
        gap_per_run,
    }
}

pub fn run_instance_with_exact_verbose<W: Write>(
    instance: &Instance,
    runs: u32,
    seed0: u64,
    params: TabuParams,
    out: &mut W,
) -> ExactGapSummary {
    let exact = exact_reference(instance).map(|e| e.bins);
    let mut objs: Vec<usize> = Vec::with_capacity(runs as usize);
    let mut times: Vec<Duration> = Vec::with_capacity(runs as usize);

    writeln!(
        out,
        "instance={} capacity={} n={} exact_bins={}",
        instance.name,
        instance.capacity,
        instance.sizes.len(),
        exact.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string())
    )
    .ok();
    out.flush().ok();

    for r in 0..runs {
        let seed = seed0 + (r as u64);
        writeln!(out, "  run {}/{} seed={}", r + 1, runs, seed).ok();
        out.flush().ok();

        let res = tabu_search(instance, seed, params);
        objs.push(res.best_bins);
        times.push(res.elapsed);

        let gap = exact.map(|ex| gap_percent(res.best_bins, ex));
        writeln!(
            out,
            "    found_bins={} gap_percent={} time={:.4}s iters={}",
            res.best_bins,
            gap.map(|g| format!("{g:.2}")).unwrap_or_else(|| "N/A".to_string()),
            res.elapsed.as_secs_f64(),
            res.iters
        )
        .ok();
        out.flush().ok();
    }

    let objs_f: Vec<f64> = objs.iter().map(|&v| v as f64).collect();
    let times_f: Vec<f64> = times.iter().map(|t| t.as_secs_f64()).collect();
    let gap_per_run = match exact {
        Some(ex) => objs.iter().map(|&b| gap_percent(b, ex)).collect(),
        None => Vec::new(),
    };

    ExactGapSummary {
        instance_name: instance.name.clone(),
        exact_bins: exact,
        mean_obj: mean(&objs_f),
        best_obj: *objs.iter().min().unwrap(),
        std_obj: pstdev(&objs_f),
        mean_time_s: mean(&times_f),
        best_time_s: times_f.iter().copied().reduce(f64::min).unwrap_or(0.0),
        gap_per_run,
    }
}
