use std::io::Write;

use crate::instances::Instance;
use crate::packing::exact_bins_if_small;
use crate::tabu::{tabu_search, TabuParams};

#[derive(Clone, Debug)]
pub struct ExactRef {
    pub bins: usize,
    pub source: &'static str,
}

fn lower_bound_bins(instance: &Instance) -> usize {
    if instance.capacity == 0 {
        return 0;
    }
    let total: u64 = instance.sizes.iter().map(|&v| v as u64).sum();
    let cap: u64 = instance.capacity as u64;
    ((total + cap - 1) / cap) as usize
}

pub fn exact_reference(instance: &Instance) -> Option<ExactRef> {
    if let Some(b) = instance.opt_bins {
        return Some(ExactRef {
            bins: b,
            source: "dataset-opt",
        });
    }
    // Brute force / exact is only practical for very small instances.
    if let Some(b) = exact_bins_if_small(instance, 30) {
        return Some(ExactRef {
            bins: b,
            source: "bruteforce",
        });
    }
    // Fallback reference (not exact): capacity lower bound.
    Some(ExactRef {
        bins: lower_bound_bins(instance),
        source: "lower-bound",
    })
}

pub fn gap_percent(found: usize, exact: usize) -> f64 {
    if exact == 0 {
        return 0.0;
    }
    ((found as f64) - (exact as f64)) / (exact as f64) * 100.0
}

pub fn compare_against_exact<W: Write>(
    instance: &Instance,
    runs: u32,
    seed0: u64,
    params: TabuParams,
    out: &mut W,
) -> std::io::Result<()> {
    let Some(exact) = exact_reference(instance) else {
        writeln!(
            out,
            "instance={} exact=N/A (n={}, no dataset-opt and too large for brute force)",
            instance.name,
            instance.sizes.len()
        )?;
        return Ok(());
    };

    writeln!(
        out,
        "instance={} exact_bins={} exact_source={}",
        instance.name, exact.bins, exact.source
    )?;
    for r in 0..runs {
        let seed = seed0 + (r as u64);
        let res = tabu_search(instance, seed, params);
        let gap = gap_percent(res.best_bins, exact.bins);
        writeln!(
            out,
            "  run={} seed={} found_bins={} gap_percent={:.2}",
            r + 1,
            seed,
            res.best_bins,
            gap
        )?;
    }
    Ok(())
}

pub fn gaps_against_exact(
    instance: &Instance,
    runs: u32,
    seed0: u64,
    params: TabuParams,
) -> Option<(ExactRef, Vec<usize>, Vec<f64>)> {
    let exact = exact_reference(instance)?;
    let mut found: Vec<usize> = Vec::with_capacity(runs as usize);
    let mut gaps: Vec<f64> = Vec::with_capacity(runs as usize);
    for r in 0..runs {
        let seed = seed0 + (r as u64);
        let res = tabu_search(instance, seed, params);
        found.push(res.best_bins);
        gaps.push(gap_percent(res.best_bins, exact.bins));
    }
    Some((exact, found, gaps))
}
