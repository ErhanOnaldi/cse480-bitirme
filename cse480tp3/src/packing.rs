use crate::instances::Instance;

#[derive(Clone, Debug)]
pub struct Packing {
    pub capacity: u32,
    pub bins: Vec<Vec<usize>>,
    pub bin_loads: Vec<u32>,
}

impl Packing {
    pub fn n_bins(&self) -> usize {
        self.bins.len()
    }
}

pub fn lower_bound_bins(instance: &Instance) -> usize {
    let sum: u32 = instance.sizes.iter().copied().sum();
    ((sum + instance.capacity - 1) / instance.capacity) as usize
}

pub fn best_fit_pack(instance: &Instance, order: &[usize]) -> Packing {
    let mut bins: Vec<Vec<usize>> = Vec::new();
    let mut loads: Vec<u32> = Vec::new();

    for &item_id in order {
        let size = instance.sizes[item_id];
        let mut best_bin_idx: Option<usize> = None;
        let mut best_remaining: Option<u32> = None;

        for (idx, &load) in loads.iter().enumerate() {
            let remaining = instance.capacity - load;
            if size <= remaining {
                let after = remaining - size;
                if best_remaining.is_none() || after < best_remaining.unwrap() {
                    best_remaining = Some(after);
                    best_bin_idx = Some(idx);
                }
            }
        }

        match best_bin_idx {
            None => {
                bins.push(vec![item_id]);
                loads.push(size);
            }
            Some(idx) => {
                bins[idx].push(item_id);
                loads[idx] += size;
            }
        }
    }

    Packing {
        capacity: instance.capacity,
        bins,
        bin_loads: loads,
    }
}

pub fn packing_objective(packing: &Packing) -> (usize, u32) {
    let unused: u32 = packing
        .bin_loads
        .iter()
        .map(|&load| packing.capacity - load)
        .sum();
    (packing.n_bins(), unused)
}

pub fn validate_packing(instance: &Instance, packing: &Packing) -> Result<(), String> {
    let n = instance.sizes.len();
    let mut seen = vec![false; n];

    if packing.capacity != instance.capacity {
        return Err("Capacity mismatch".to_string());
    }

    if packing.bins.len() != packing.bin_loads.len() {
        return Err("bins/bin_loads length mismatch".to_string());
    }

    for (bin_items, &load) in packing.bins.iter().zip(packing.bin_loads.iter()) {
        let computed: u32 = bin_items.iter().map(|&i| instance.sizes[i]).sum();
        if computed != load {
            return Err(format!("Bin load mismatch: expected {computed}, got {load}"));
        }
        if load > instance.capacity {
            return Err(format!(
                "Infeasible bin: load {load} > capacity {}",
                instance.capacity
            ));
        }
        for &i in bin_items {
            if i >= n {
                return Err(format!("Invalid item id: {i}"));
            }
            if seen[i] {
                return Err(format!("Item appears more than once: {i}"));
            }
            seen[i] = true;
        }
    }

    if let Some((idx, _)) = seen.iter().enumerate().find(|(_, ok)| !**ok) {
        return Err(format!("Missing item in packing: {idx}"));
    }
    Ok(())
}

pub fn try_reduce_bins(instance: &Instance, packing: &Packing) -> Packing {
    let mut bins = packing.bins.clone();
    let mut loads = packing.bin_loads.clone();

    let mut changed = true;
    while changed {
        changed = false;

        let mut indices: Vec<usize> = (0..bins.len()).collect();
        indices.sort_by_key(|&i| loads[i]);

        'outer: for source_idx in indices {
            if bins[source_idx].is_empty() {
                continue;
            }
            let mut placements: Vec<(usize, usize)> = Vec::new();

            let mut items_to_move = bins[source_idx].clone();
            items_to_move.sort_by_key(|&i| std::cmp::Reverse(instance.sizes[i]));

            let mut feasible = true;
            for item_id in items_to_move.iter().copied() {
                let size = instance.sizes[item_id];
                let mut target_idx: Option<usize> = None;
                let mut best_after: Option<u32> = None;

                for idx in 0..bins.len() {
                    if idx == source_idx {
                        continue;
                    }
                    let remaining = instance.capacity - loads[idx];
                    if size <= remaining {
                        let after = remaining - size;
                        if best_after.is_none() || after < best_after.unwrap() {
                            best_after = Some(after);
                            target_idx = Some(idx);
                        }
                    }
                }

                if let Some(t) = target_idx {
                    placements.push((item_id, t));
                    loads[t] += size;
                } else {
                    feasible = false;
                    break;
                }
            }

            if !feasible {
                for (item_id, t) in placements {
                    loads[t] -= instance.sizes[item_id];
                }
                continue;
            }

            for (item_id, t) in placements {
                bins[t].push(item_id);
            }
            bins[source_idx].clear();
            loads[source_idx] = 0;

            let mut new_bins = Vec::with_capacity(bins.len());
            let mut new_loads = Vec::with_capacity(loads.len());
            for (b, l) in bins.into_iter().zip(loads.into_iter()) {
                if !b.is_empty() {
                    new_bins.push(b);
                    new_loads.push(l);
                }
            }
            bins = new_bins;
            loads = new_loads;

            changed = true;
            break 'outer;
        }
    }

    Packing {
        capacity: instance.capacity,
        bins,
        bin_loads: loads,
    }
}

pub fn exact_min_bins(instance: &Instance) -> Result<usize, String> {
    // Branch-and-bound: intended only for small instances (like the TP2 example).
    if instance.sizes.iter().any(|&s| s > instance.capacity) {
        return Err("Instance contains an item larger than bin capacity.".to_string());
    }

    let mut sizes = instance.sizes.clone();
    sizes.sort_by_key(|&s| std::cmp::Reverse(s));

    let mut best = sizes.len();
    let mut loads: Vec<u32> = Vec::new();

    fn dfs(k: usize, sizes: &[u32], capacity: u32, loads: &mut Vec<u32>, best: &mut usize) {
        if k == sizes.len() {
            *best = (*best).min(loads.len());
            return;
        }
        if loads.len() >= *best {
            return;
        }

        let size = sizes[k];
        let mut tried: Vec<u32> = Vec::new();
        for i in 0..loads.len() {
            if tried.contains(&loads[i]) {
                continue;
            }
            if loads[i] + size <= capacity {
                tried.push(loads[i]);
                loads[i] += size;
                dfs(k + 1, sizes, capacity, loads, best);
                loads[i] -= size;
            }
        }

        loads.push(size);
        dfs(k + 1, sizes, capacity, loads, best);
        loads.pop();
    }

    dfs(0, &sizes, instance.capacity, &mut loads, &mut best);
    Ok(best)
}

pub fn exact_bins_if_small(instance: &Instance, max_items: usize) -> Option<usize> {
    if instance.sizes.len() > max_items {
        return None;
    }
    exact_min_bins(instance).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instances::example_instance_tp2;

    #[test]
    fn tp2_example_optimum_is_4() {
        let inst = example_instance_tp2();
        let opt = exact_min_bins(&inst).unwrap();
        assert_eq!(opt, 4);
    }
}
