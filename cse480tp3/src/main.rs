use std::time::Duration;

use cse480tp3::experiments::{
    format_exact_gap_table, format_table, run_instance, run_instance_verbose, run_instance_with_exact,
    run_instance_with_exact_verbose,
};
use cse480tp3::exact_compare::compare_against_exact;
use cse480tp3::instances::{
    default_batch_instances, example_instance_tp2, load_bpp_instances_from_dir, load_bpp_instances_from_file,
};
use cse480tp3::packing::{exact_min_bins, validate_packing};
use cse480tp3::tabu::{tabu_search, tabu_search_trace, TabuParams, TraceConfig};

fn usage() -> ! {
    eprintln!(
        "Usage:\n  cargo run --release -- run-example\n  cargo run --release -- trace-tp2 [--iters N] [--samples K] [--tenure T] [--seed S] [--show-packings] [--no-candidates]\n  cargo run --release -- compare-exact-file <FILE> [--runs N] [--seed0 S] [--skip S] [--take K] [--time-limit-s T]\n  cargo run --release -- report-file <FILE> [--runs N] [--seed0 S] [--skip S] [--take K] [--time-limit-s T] [--progress]\n  cargo run --release -- run-batch [--runs N] [--seed0 S] [--time-limit-s T] [--progress]\n  cargo run --release -- run-file <FILE> [--runs N] [--seed0 S] [--skip S] [--take K] [--time-limit-s T] [--progress]\n  cargo run --release -- run-dir <DIR> [--runs N] [--seed0 S] [--skip S] [--take K] [--time-limit-s T] [--progress]\n"
    );
    std::process::exit(2);
}

fn parse_u32(flag: &str, v: Option<&String>) -> u32 {
    v.unwrap_or_else(|| usage()).parse::<u32>().unwrap_or_else(|_| {
        eprintln!("Invalid value for {flag}");
        usage()
    })
}

fn parse_u64(flag: &str, v: Option<&String>) -> u64 {
    v.unwrap_or_else(|| usage()).parse::<u64>().unwrap_or_else(|_| {
        eprintln!("Invalid value for {flag}");
        usage()
    })
}

fn parse_usize(flag: &str, v: Option<&String>) -> usize {
    v.unwrap_or_else(|| usage()).parse::<usize>().unwrap_or_else(|_| {
        eprintln!("Invalid value for {flag}");
        usage()
    })
}

fn parse_f64(flag: &str, v: Option<&String>) -> f64 {
    v.unwrap_or_else(|| usage()).parse::<f64>().unwrap_or_else(|_| {
        eprintln!("Invalid value for {flag}");
        usage()
    })
}

fn run_example() -> i32 {
    let inst = example_instance_tp2();
    let params = TabuParams {
        max_iters: 2_000,
        neighborhood_samples: 150,
        tabu_tenure: 20,
        stagnation_limit: 400,
        time_limit: None,
    };

    let exact = exact_min_bins(&inst).ok();
    let res = tabu_search(&inst, 0, params);

    println!(
        "Instance: {} (capacity={}, n={})",
        inst.name,
        inst.capacity,
        inst.sizes.len()
    );
    if let Some(opt) = exact {
        println!("Exact optimum (small-instance check): {opt} bins");
    }
    println!(
        "Best found: {} bins (unused={})  iters={}  time(s)={:.4}",
        res.best_bins,
        res.best_unused,
        res.iters,
        res.elapsed.as_secs_f64()
    );

    if let Err(e) = validate_packing(&inst, &res.best_packing) {
        eprintln!("ERROR: produced invalid packing: {e}");
        return 1;
    }

    println!("Bins (item_id:size):");
    for b in res.best_packing.bins.iter() {
        let load: u32 = b.iter().map(|&i| inst.sizes[i]).sum();
        let items = b
            .iter()
            .map(|&i| format!("{}:{}", i + 1, inst.sizes[i]))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  load={:3}  [{}]", load, items);
    }

    if res.best_bins != 4 {
        println!("NOTE: TP2 example expected optimum is 4 bins; try increasing iterations/samples if needed.");
    }

    0
}

fn compare_exact_file(args: &[String]) -> i32 {
    if args.is_empty() {
        usage();
    }
    let file = args[0].clone();
    let mut runs: u32 = 5;
    let mut seed0: u64 = 0;
    let mut skip: usize = 0;
    let mut take: Option<usize> = None;
    let mut time_limit_s: f64 = 2.0;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" => {
                runs = parse_u32("--runs", args.get(i + 1));
                i += 2;
            }
            "--seed0" => {
                seed0 = parse_u64("--seed0", args.get(i + 1));
                i += 2;
            }
            "--skip" => {
                skip = parse_usize("--skip", args.get(i + 1));
                i += 2;
            }
            "--take" => {
                take = Some(parse_usize("--take", args.get(i + 1)));
                i += 2;
            }
            "--time-limit-s" => {
                time_limit_s = parse_f64("--time-limit-s", args.get(i + 1));
                i += 2;
            }
            "--help" | "-h" => usage(),
            other => {
                eprintln!("Unknown arg: {other}");
                usage()
            }
        }
    }

    let instances = match load_bpp_instances_from_file(&file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    let time_limit = if time_limit_s <= 0.0 {
        None
    } else {
        Some(Duration::from_secs_f64(time_limit_s))
    };

    let params = TabuParams {
        max_iters: 5_000,
        neighborhood_samples: 200,
        tabu_tenure: 25,
        stagnation_limit: 600,
        time_limit,
    };

    let iter0 = instances.into_iter().skip(skip);
    let iter: Box<dyn Iterator<Item = _>> = match take {
        Some(k) => Box::new(iter0.take(k)),
        None => Box::new(iter0),
    };

    let mut stdout = std::io::stdout().lock();
    for inst in iter {
        if let Err(e) = compare_against_exact(&inst, runs, seed0, params, &mut stdout) {
            eprintln!("compare failed: {e}");
            return 1;
        }
    }
    0
}

fn report_file(args: &[String]) -> i32 {
    if args.is_empty() {
        usage();
    }
    let file = args[0].clone();
    let mut runs: u32 = 5;
    let mut seed0: u64 = 0;
    let mut skip: usize = 0;
    let mut take: Option<usize> = None;
    let mut time_limit_s: f64 = 2.0;
    let mut progress = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" => {
                runs = parse_u32("--runs", args.get(i + 1));
                i += 2;
            }
            "--seed0" => {
                seed0 = parse_u64("--seed0", args.get(i + 1));
                i += 2;
            }
            "--skip" => {
                skip = parse_usize("--skip", args.get(i + 1));
                i += 2;
            }
            "--take" => {
                take = Some(parse_usize("--take", args.get(i + 1)));
                i += 2;
            }
            "--time-limit-s" => {
                time_limit_s = parse_f64("--time-limit-s", args.get(i + 1));
                i += 2;
            }
            "--progress" => {
                progress = true;
                i += 1;
            }
            "--help" | "-h" => usage(),
            other => {
                eprintln!("Unknown arg: {other}");
                usage()
            }
        }
    }

    let instances = match load_bpp_instances_from_file(&file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    let time_limit = if time_limit_s <= 0.0 {
        None
    } else {
        Some(Duration::from_secs_f64(time_limit_s))
    };

    let params = TabuParams {
        max_iters: 5_000,
        neighborhood_samples: 200,
        tabu_tenure: 25,
        stagnation_limit: 600,
        time_limit,
    };

    let iter0 = instances.into_iter().skip(skip);
    let iter: Box<dyn Iterator<Item = _>> = match take {
        Some(k) => Box::new(iter0.take(k)),
        None => Box::new(iter0),
    };

    let mut rows = Vec::new();
    let mut stderr = std::io::stderr().lock();
    for inst in iter {
        let row = if progress {
            run_instance_with_exact_verbose(&inst, runs, seed0, params, &mut stderr)
        } else {
            run_instance_with_exact(&inst, runs, seed0, params)
        };
        rows.push(row);
    }
    print!("{}", format_exact_gap_table(&rows));
    0
}

fn trace_tp2(args: &[String]) -> i32 {
    let mut iters: u32 = 30;
    let mut samples: u32 = 25;
    let mut tenure: usize = 10;
    let mut seed: u64 = 0;
    let mut show_packings = false;
    let mut show_candidates = true;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--iters" => {
                iters = parse_u32("--iters", args.get(i + 1));
                i += 2;
            }
            "--samples" => {
                samples = parse_u32("--samples", args.get(i + 1));
                i += 2;
            }
            "--tenure" => {
                tenure = parse_usize("--tenure", args.get(i + 1));
                i += 2;
            }
            "--seed" => {
                seed = parse_u64("--seed", args.get(i + 1));
                i += 2;
            }
            "--show-packings" => {
                show_packings = true;
                i += 1;
            }
            "--no-candidates" => {
                show_candidates = false;
                i += 1;
            }
            "--help" | "-h" => usage(),
            other => {
                eprintln!("Unknown arg: {other}");
                usage()
            }
        }
    }

    let inst = example_instance_tp2();
    let params = TabuParams {
        max_iters: iters,
        neighborhood_samples: samples,
        tabu_tenure: tenure,
        stagnation_limit: 10_000,
        time_limit: None,
    };
    let cfg = TraceConfig {
        show_candidates,
        show_packings,
    };

    let mut stdout = std::io::stdout().lock();
    if let Err(e) = tabu_search_trace(&inst, seed, params, cfg, &mut stdout) {
        eprintln!("trace failed: {e}");
        return 1;
    }
    0
}

fn run_batch(args: &[String]) -> i32 {
    let mut runs: u32 = 5;
    let mut seed0: u64 = 0;
    let mut time_limit_s: f64 = 2.0;
    let mut progress = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" => {
                runs = parse_u32("--runs", args.get(i + 1));
                i += 2;
            }
            "--seed0" => {
                seed0 = parse_u64("--seed0", args.get(i + 1));
                i += 2;
            }
            "--time-limit-s" => {
                time_limit_s = parse_f64("--time-limit-s", args.get(i + 1));
                i += 2;
            }
            "--progress" => {
                progress = true;
                i += 1;
            }
            "--help" | "-h" => usage(),
            other => {
                eprintln!("Unknown arg: {other}");
                usage()
            }
        }
    }

    let time_limit = if time_limit_s <= 0.0 {
        None
    } else {
        Some(Duration::from_secs_f64(time_limit_s))
    };

    let params = TabuParams {
        max_iters: 5_000,
        neighborhood_samples: 200,
        tabu_tenure: 25,
        stagnation_limit: 600,
        time_limit,
    };

    let mut summaries = Vec::new();
    for inst in default_batch_instances() {
        let (s, _, _) = if progress {
            let mut stderr = std::io::stderr().lock();
            run_instance_verbose(&inst, runs, seed0, params, &mut stderr)
        } else {
            run_instance(&inst, runs, seed0, params)
        };
        summaries.push(s);
    }

    print!("{}", format_table(&summaries));
    0
}

fn run_dir(args: &[String]) -> i32 {
    if args.is_empty() {
        usage();
    }
    let dir = args[0].clone();
    let mut runs: u32 = 5;
    let mut seed0: u64 = 0;
    let mut skip: usize = 0;
    let mut take: Option<usize> = None;
    let mut time_limit_s: f64 = 2.0;
    let mut progress = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" => {
                runs = parse_u32("--runs", args.get(i + 1));
                i += 2;
            }
            "--seed0" => {
                seed0 = parse_u64("--seed0", args.get(i + 1));
                i += 2;
            }
            "--skip" => {
                skip = parse_usize("--skip", args.get(i + 1));
                i += 2;
            }
            "--take" => {
                take = Some(parse_usize("--take", args.get(i + 1)));
                i += 2;
            }
            "--time-limit-s" => {
                time_limit_s = parse_f64("--time-limit-s", args.get(i + 1));
                i += 2;
            }
            "--progress" => {
                progress = true;
                i += 1;
            }
            "--help" | "-h" => usage(),
            other => {
                eprintln!("Unknown arg: {other}");
                usage()
            }
        }
    }

    let instances = match load_bpp_instances_from_dir(&dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    let time_limit = if time_limit_s <= 0.0 {
        None
    } else {
        Some(Duration::from_secs_f64(time_limit_s))
    };

    let params = TabuParams {
        max_iters: 5_000,
        neighborhood_samples: 200,
        tabu_tenure: 25,
        stagnation_limit: 600,
        time_limit,
    };

    let mut summaries = Vec::new();
    let iter0 = instances.into_iter().skip(skip);
    let iter: Box<dyn Iterator<Item = _>> = match take {
        Some(k) => Box::new(iter0.take(k)),
        None => Box::new(iter0),
    };
    let mut stderr = std::io::stderr().lock();
    for inst in iter {
        let (s, _, _) = if progress {
            run_instance_verbose(&inst, runs, seed0, params, &mut stderr)
        } else {
            run_instance(&inst, runs, seed0, params)
        };
        summaries.push(s);
    }
    print!("{}", format_table(&summaries));
    0
}

fn run_file(args: &[String]) -> i32 {
    if args.is_empty() {
        usage();
    }
    let file = args[0].clone();
    let mut runs: u32 = 5;
    let mut seed0: u64 = 0;
    let mut skip: usize = 0;
    let mut take: Option<usize> = None;
    let mut time_limit_s: f64 = 2.0;
    let mut progress = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" => {
                runs = parse_u32("--runs", args.get(i + 1));
                i += 2;
            }
            "--seed0" => {
                seed0 = parse_u64("--seed0", args.get(i + 1));
                i += 2;
            }
            "--skip" => {
                skip = parse_usize("--skip", args.get(i + 1));
                i += 2;
            }
            "--take" => {
                take = Some(parse_usize("--take", args.get(i + 1)));
                i += 2;
            }
            "--time-limit-s" => {
                time_limit_s = parse_f64("--time-limit-s", args.get(i + 1));
                i += 2;
            }
            "--progress" => {
                progress = true;
                i += 1;
            }
            "--help" | "-h" => usage(),
            other => {
                eprintln!("Unknown arg: {other}");
                usage()
            }
        }
    }

    let instances = match load_bpp_instances_from_file(&file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    let time_limit = if time_limit_s <= 0.0 {
        None
    } else {
        Some(Duration::from_secs_f64(time_limit_s))
    };

    let params = TabuParams {
        max_iters: 5_000,
        neighborhood_samples: 200,
        tabu_tenure: 25,
        stagnation_limit: 600,
        time_limit,
    };

    let mut summaries = Vec::new();
    let iter0 = instances.into_iter().skip(skip);
    let iter: Box<dyn Iterator<Item = _>> = match take {
        Some(k) => Box::new(iter0.take(k)),
        None => Box::new(iter0),
    };
    let mut stderr = std::io::stderr().lock();
    for inst in iter {
        let (s, _, _) = if progress {
            run_instance_verbose(&inst, runs, seed0, params, &mut stderr)
        } else {
            run_instance(&inst, runs, seed0, params)
        };
        summaries.push(s);
    }
    print!("{}", format_table(&summaries));
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let cmd = args[1].as_str();
    let code = match cmd {
        "run-example" => run_example(),
        "trace-tp2" => trace_tp2(&args[2..]),
        "compare-exact-file" => compare_exact_file(&args[2..]),
        "report-file" => report_file(&args[2..]),
        "run-batch" => run_batch(&args[2..]),
        "run-file" => run_file(&args[2..]),
        "run-dir" => run_dir(&args[2..]),
        _ => usage(),
    };
    std::process::exit(code);
}
