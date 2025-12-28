use std::collections::{HashSet, VecDeque};
use std::hash::Hash;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::instances::Instance;
use crate::packing::{best_fit_pack, lower_bound_bins, packing_objective, try_reduce_bins, Packing};
use crate::rng::XorShift64;

#[derive(Clone, Copy, Debug)]
pub struct TabuParams {
    pub max_iters: u32,
    pub neighborhood_samples: u32,
    pub tabu_tenure: usize,
    pub stagnation_limit: u32,
    pub time_limit: Option<Duration>,
}

impl Default for TabuParams {
    fn default() -> Self {
        Self {
            max_iters: 5_000,
            neighborhood_samples: 200,
            tabu_tenure: 25,
            stagnation_limit: 600,
            time_limit: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TabuResult {
    pub best_order: Vec<usize>,
    pub best_packing: Packing,
    pub best_bins: usize,
    pub best_unused: u32,
    pub elapsed: Duration,
    pub iters: u32,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum MoveKey {
    Swap { a: usize, b: usize },
    Insert { item: usize, pos: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct TraceConfig {
    pub show_candidates: bool,
    pub show_packings: bool,
}

fn format_order(instance: &Instance, order: &[usize]) -> String {
    order
        .iter()
        .map(|&i| format!("{}:{}", i + 1, instance.sizes[i]))
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_packing<W: Write>(w: &mut W, instance: &Instance, packing: &Packing) -> std::io::Result<()> {
    for (b_idx, b) in packing.bins.iter().enumerate() {
        let load: u32 = b.iter().map(|&i| instance.sizes[i]).sum();
        let items = b
            .iter()
            .map(|&i| format!("{}:{}", i + 1, instance.sizes[i]))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(w, "    bin#{:02} load={:3} [{}]", b_idx + 1, load, items)?;
    }
    Ok(())
}

fn tabu_push<K: Copy + Eq + Hash>(queue: &mut VecDeque<K>, set: &mut HashSet<K>, key: K, max_len: usize) {
    if max_len == 0 {
        return;
    }
    while queue.len() >= max_len {
        if let Some(old) = queue.pop_front() {
            set.remove(&old);
        }
    }
    queue.push_back(key);
    set.insert(key);
}

fn apply_swap(order: &[usize], i: usize, j: usize) -> Vec<usize> {
    let mut out = order.to_vec();
    out.swap(i, j);
    out
}

fn apply_insert(order: &[usize], i: usize, j: usize) -> Vec<usize> {
    if i == j {
        return order.to_vec();
    }
    let mut out = order.to_vec();
    let item = out.remove(i);
    out.insert(j, item);
    out
}

pub fn tabu_search(instance: &Instance, seed: u64, params: TabuParams) -> TabuResult {
    let n = instance.sizes.len();
    let start = Instant::now();
    let mut rng = XorShift64::new(seed);

    // Strong baseline: decreasing sizes with deterministic tie-breaking.
    let mut items: Vec<usize> = (0..n).collect();
    let tiebreak: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
    items.sort_by_key(|&i| (std::cmp::Reverse(instance.sizes[i]), tiebreak[i]));
    let mut current = items;

    let mut current_pack = try_reduce_bins(instance, &best_fit_pack(instance, &current));
    let mut current_obj = packing_objective(&current_pack);

    let mut best_order = current.clone();
    let mut best_pack = current_pack.clone();
    let mut best_obj = current_obj;
    let mut best_iter: u32 = 0;

    let mut tabu_q: VecDeque<MoveKey> = VecDeque::new();
    let mut tabu_set: HashSet<MoveKey> = HashSet::new();

    let lb = lower_bound_bins(instance);
    let mut last_it = 0;

    for it in 1..=params.max_iters {
        last_it = it;
        if let Some(limit) = params.time_limit {
            if start.elapsed() >= limit {
                break;
            }
        }

        if it.saturating_sub(best_iter) >= params.stagnation_limit {
            current = best_order.clone();
            rng.shuffle(&mut current);
            tabu_q.clear();
            tabu_set.clear();
        }

        let mut best_candidate: Option<Vec<usize>> = None;
        let mut best_candidate_obj: Option<(usize, u32)> = None;
        let mut best_candidate_move: Option<MoveKey> = None;
        let mut best_candidate_pack: Option<Packing> = None;

        for _ in 0..params.neighborhood_samples {
            let move_is_swap = rng.gen_f64() < 0.6;
            let i = rng.gen_range_usize(n);
            let j = rng.gen_range_usize(n);
            if i == j {
                continue;
            }

            let (candidate, mv) = if move_is_swap {
                let a = current[i];
                let b = current[j];
                let (a, b) = if a < b { (a, b) } else { (b, a) };
                (apply_swap(&current, i, j), MoveKey::Swap { a, b })
            } else {
                let item = current[i];
                (apply_insert(&current, i, j), MoveKey::Insert { item, pos: j })
            };

            let pack = try_reduce_bins(instance, &best_fit_pack(instance, &candidate));
            let obj = packing_objective(&pack);

            let is_tabu = tabu_set.contains(&mv);
            let aspiration = obj < best_obj;
            if is_tabu && !aspiration {
                continue;
            }

            if best_candidate_obj.is_none() || obj < best_candidate_obj.unwrap() {
                best_candidate = Some(candidate);
                best_candidate_obj = Some(obj);
                best_candidate_move = Some(mv);
                best_candidate_pack = Some(pack);
            }
        }

        let Some(candidate) = best_candidate else { continue };
        current = candidate;
        current_pack = best_candidate_pack.unwrap();
        current_obj = best_candidate_obj.unwrap();
        tabu_push(&mut tabu_q, &mut tabu_set, best_candidate_move.unwrap(), params.tabu_tenure);

        if current_obj < best_obj {
            best_obj = current_obj;
            best_order = current.clone();
            best_pack = current_pack.clone();
            best_iter = it;
            if best_obj.0 == lb {
                break;
            }
        }
    }

    TabuResult {
        best_order,
        best_packing: best_pack,
        best_bins: best_obj.0,
        best_unused: best_obj.1,
        elapsed: start.elapsed(),
        iters: last_it,
    }
}

pub fn tabu_search_trace<W: Write>(
    instance: &Instance,
    seed: u64,
    params: TabuParams,
    cfg: TraceConfig,
    out: &mut W,
) -> std::io::Result<TabuResult> {
    let n = instance.sizes.len();
    let start = Instant::now();
    let mut rng = XorShift64::new(seed);

    writeln!(out, "TRACE: Tabu Search (TP2 instance trace)")?;
    writeln!(
        out,
        "instance={} capacity={} n={} seed={}",
        instance.name, instance.capacity, n, seed
    )?;
    writeln!(
        out,
        "params: max_iters={} neighborhood_samples={} tabu_tenure={} stagnation_limit={} time_limit={:?}",
        params.max_iters, params.neighborhood_samples, params.tabu_tenure, params.stagnation_limit, params.time_limit
    )?;

    let lb = lower_bound_bins(instance);
    writeln!(out, "lower_bound_bins={}", lb)?;

    let mut items: Vec<usize> = (0..n).collect();
    let tiebreak: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
    items.sort_by_key(|&i| (std::cmp::Reverse(instance.sizes[i]), tiebreak[i]));
    let mut current = items;

    let mut current_pack = try_reduce_bins(instance, &best_fit_pack(instance, &current));
    let mut current_obj = packing_objective(&current_pack);

    writeln!(out, "\ninit permutation (item:size): {}", format_order(instance, &current))?;
    writeln!(out, "init objective: bins={} unused={}", current_obj.0, current_obj.1)?;
    if cfg.show_packings {
        writeln!(out, "  init packing:")?;
        write_packing(out, instance, &current_pack)?;
    }

    let mut best_order = current.clone();
    let mut best_pack = current_pack.clone();
    let mut best_obj = current_obj;
    let mut best_iter: u32 = 0;

    let mut tabu_q: VecDeque<MoveKey> = VecDeque::new();
    let mut tabu_set: HashSet<MoveKey> = HashSet::new();

    let mut last_it = 0;

    for it in 1..=params.max_iters {
        last_it = it;
        if let Some(limit) = params.time_limit {
            if start.elapsed() >= limit {
                writeln!(out, "\nstop: time_limit reached at it={}", it)?;
                break;
            }
        }

        if it.saturating_sub(best_iter) >= params.stagnation_limit {
            writeln!(out, "\nit={}: stagnation reached, diversify: shuffle(best_order) + clear tabu", it)?;
            current = best_order.clone();
            rng.shuffle(&mut current);
            tabu_q.clear();
            tabu_set.clear();
        }

        writeln!(
            out,
            "\n-- it={} -- current bins={} unused={} best bins={} unused={} tabu_size={}",
            it,
            current_obj.0,
            current_obj.1,
            best_obj.0,
            best_obj.1,
            tabu_set.len()
        )?;

        let mut best_candidate: Option<Vec<usize>> = None;
        let mut best_candidate_obj: Option<(usize, u32)> = None;
        let mut best_candidate_move: Option<MoveKey> = None;
        let mut best_candidate_pack: Option<Packing> = None;

        for s in 0..params.neighborhood_samples {
            let move_is_swap = rng.gen_f64() < 0.6;
            let i = rng.gen_range_usize(n);
            let j = rng.gen_range_usize(n);
            if i == j {
                continue;
            }

            let (candidate, mv, mv_desc) = if move_is_swap {
                let item_i = current[i];
                let item_j = current[j];
                let (a, b) = if item_i < item_j { (item_i, item_j) } else { (item_j, item_i) };
                (
                    apply_swap(&current, i, j),
                    MoveKey::Swap { a, b },
                    format!(
                        "swap pos {}<->{}  items {}:{} <-> {}:{}",
                        i,
                        j,
                        item_i + 1,
                        instance.sizes[item_i],
                        item_j + 1,
                        instance.sizes[item_j]
                    ),
                )
            } else {
                let item = current[i];
                (
                    apply_insert(&current, i, j),
                    MoveKey::Insert { item, pos: j },
                    format!(
                        "insert from pos {} to {}  item {}:{}",
                        i,
                        j,
                        item + 1,
                        instance.sizes[item]
                    ),
                )
            };

            let pack = try_reduce_bins(instance, &best_fit_pack(instance, &candidate));
            let obj = packing_objective(&pack);

            let is_tabu = tabu_set.contains(&mv);
            let aspiration = obj < best_obj;
            let allowed = !is_tabu || aspiration;

            if cfg.show_candidates {
                writeln!(
                    out,
                    "  sample#{:03}: {:45} -> bins={} unused={} tabu={} aspiration={} allowed={}",
                    s + 1,
                    mv_desc,
                    obj.0,
                    obj.1,
                    is_tabu,
                    aspiration,
                    allowed
                )?;
            }

            if !allowed {
                continue;
            }

            if best_candidate_obj.is_none() || obj < best_candidate_obj.unwrap() {
                best_candidate = Some(candidate);
                best_candidate_obj = Some(obj);
                best_candidate_move = Some(mv);
                best_candidate_pack = Some(pack);
            }
        }

        let Some(candidate) = best_candidate else {
            writeln!(out, "  no admissible candidate found")?;
            continue;
        };

        current = candidate;
        current_pack = best_candidate_pack.unwrap();
        current_obj = best_candidate_obj.unwrap();

        let chosen_move = best_candidate_move.unwrap();
        let chosen_desc = match chosen_move {
            MoveKey::Swap { a, b } => format!("chosen move: swap items {}:{} and {}:{}", a + 1, instance.sizes[a], b + 1, instance.sizes[b]),
            MoveKey::Insert { item, pos } => format!("chosen move: insert item {}:{} to position {}", item + 1, instance.sizes[item], pos),
        };
        writeln!(out, "  {}", chosen_desc)?;
        writeln!(out, "  new current: bins={} unused={}", current_obj.0, current_obj.1)?;

        tabu_push(&mut tabu_q, &mut tabu_set, chosen_move, params.tabu_tenure);

        if cfg.show_packings {
            writeln!(out, "  packing after move:")?;
            write_packing(out, instance, &current_pack)?;
        }

        if current_obj < best_obj {
            best_obj = current_obj;
            best_order = current.clone();
            best_pack = current_pack.clone();
            best_iter = it;
            writeln!(out, "  NEW BEST at it={}: bins={} unused={}", it, best_obj.0, best_obj.1)?;
            if best_obj.0 == lb {
                writeln!(out, "stop: reached lower bound on bins")?;
                break;
            }
        }
    }

    writeln!(out, "\nDONE: elapsed={:.4}s iters={}", start.elapsed().as_secs_f64(), last_it)?;
    writeln!(out, "best: bins={} unused={}", best_obj.0, best_obj.1)?;
    writeln!(out, "best permutation (item:size): {}", format_order(instance, &best_order))?;
    if cfg.show_packings {
        writeln!(out, "best packing:")?;
        write_packing(out, instance, &best_pack)?;
    }

    Ok(TabuResult {
        best_order,
        best_packing: best_pack,
        best_bins: best_obj.0,
        best_unused: best_obj.1,
        elapsed: start.elapsed(),
        iters: last_it,
    })
}
