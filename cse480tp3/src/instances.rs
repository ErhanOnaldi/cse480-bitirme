use crate::rng::XorShift64;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Instance {
    pub name: String,
    pub capacity: u32,
    pub sizes: Vec<u32>,
    pub opt_bins: Option<usize>,
}

fn decimal_places(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (signless, _) = s.strip_prefix('-').map(|x| (x, true)).unwrap_or((s, false));
    if signless.is_empty() {
        return None;
    }
    if let Some((a, b)) = signless.split_once('.') {
        if a.is_empty() || !a.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if b.is_empty() || !b.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(b.len())
    } else if signless.chars().all(|c| c.is_ascii_digit()) {
        Some(0)
    } else {
        None
    }
}

fn pow10_u32(exp: usize) -> Result<u32, String> {
    let mut v: u32 = 1;
    for _ in 0..exp {
        v = v.checked_mul(10).ok_or("scale overflow".to_string())?;
    }
    Ok(v)
}

fn parse_decimal_scaled_u32(token: &str, scale: usize) -> Result<u32, String> {
    let token = token.trim();
    if token.starts_with('-') {
        return Err(format!("negative value not allowed: {token}"));
    }
    let mult = pow10_u32(scale)?;
    if let Some((a, b)) = token.split_once('.') {
        let int_part: u32 = a.parse().map_err(|_| format!("invalid number: {token}"))?;
        let frac_digits = b.len();
        let mut frac_part: u32 = b.parse().map_err(|_| format!("invalid number: {token}"))?;
        if frac_digits > scale {
            return Err(format!("too many decimals in {token} for scale {scale}"));
        }
        let extra = scale - frac_digits;
        frac_part = frac_part
            .checked_mul(pow10_u32(extra)?)
            .ok_or("scaled value overflow".to_string())?;
        int_part
            .checked_mul(mult)
            .and_then(|v| v.checked_add(frac_part))
            .ok_or("scaled value overflow".to_string())
    } else {
        let int_part: u32 = token.parse().map_err(|_| format!("invalid number: {token}"))?;
        int_part
            .checked_mul(mult)
            .ok_or("scaled value overflow".to_string())
    }
}

fn parse_simple_single_instance(path: &Path, content: &str) -> Result<Instance, String> {
    // Accept whitespace-separated integers; allow full-line comments starting with '#'.
    let mut ints: Vec<u32> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for tok in line.split_whitespace() {
            let Ok(v) = tok.parse::<u32>() else {
                return Err(format!("Non-integer token '{tok}'"));
            };
            ints.push(v);
        }
    }

    if ints.len() < 3 {
        return Err("Too few integers".to_string());
    }

    let parse_as_n_cap = |ints: &[u32]| -> Option<(usize, u32, Vec<u32>)> {
        let n = ints[0] as usize;
        let cap = ints[1];
        if ints.len() != 2 + n {
            return None;
        }
        Some((n, cap, ints[2..].to_vec()))
    };

    let parse_as_cap_n = |ints: &[u32]| -> Option<(usize, u32, Vec<u32>)> {
        let cap = ints[0];
        let n = ints[1] as usize;
        if ints.len() != 2 + n {
            return None;
        }
        Some((n, cap, ints[2..].to_vec()))
    };

    let parsed = parse_as_n_cap(&ints).or_else(|| parse_as_cap_n(&ints));
    let Some((n, cap, sizes)) = parsed else {
        return Err("Invalid simple instance format".to_string());
    };

    if cap == 0 {
        return Err("Capacity must be > 0".to_string());
    }
    if sizes.len() != n {
        return Err("Size count mismatch".to_string());
    }
    if sizes.iter().any(|&s| s == 0) {
        return Err("Item sizes must be > 0".to_string());
    }
    if let Some(&mx) = sizes.iter().max() {
        if mx > cap {
            return Err(format!("max_size {mx} > capacity {cap}"));
        }
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dataset")
        .to_string();

    Ok(Instance {
        name,
        capacity: cap,
        sizes,
        opt_bins: None,
    })
}

fn parse_binpack_multi(path: &Path, content: &str) -> Result<Vec<Instance>, String> {
    // Common "BinPack" multi-instance layout:
    //   K
    //   <name>
    //   <capacity> <n> <opt?>
    //   <size> x n
    // repeated K times.
    let lines: Vec<&str> = content.lines().collect();
    let mut idx: usize = 0;

    let next_line = |idx: &mut usize| -> Option<String> {
        while *idx < lines.len() {
            let s = lines[*idx].trim().to_string();
            *idx += 1;
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            return Some(s);
        }
        None
    };

    let start_idx = idx;
    let Some(first) = next_line(&mut idx) else {
        return Err("empty file".to_string());
    };
    let Ok(k) = first.parse::<usize>() else {
        return Err("not a multi-instance file".to_string());
    };
    let Some(name_peek) = next_line(&mut idx) else {
        return Err("unexpected EOF after instance count".to_string());
    };
    // Name should not be a pure number in this format.
    if decimal_places(&name_peek).is_some() {
        return Err("multi-instance header mismatch".to_string());
    }
    let Some(header_peek) = next_line(&mut idx) else {
        return Err("unexpected EOF after instance name".to_string());
    };
    let header_toks: Vec<&str> = header_peek.split_whitespace().collect();
    if header_toks.len() < 2 || decimal_places(header_toks[0]).is_none() || header_toks[1].parse::<usize>().is_err() {
        return Err("multi-instance header mismatch".to_string());
    }

    // Reset and do full parse.
    idx = start_idx;
    let _ = next_line(&mut idx).unwrap(); // consume k

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("dataset");
    let mut instances: Vec<Instance> = Vec::with_capacity(k);

    for _ in 0..k {
        let name = next_line(&mut idx).ok_or("unexpected EOF while reading instance name")?;
        let header = next_line(&mut idx).ok_or("unexpected EOF while reading instance header")?;
        let header_toks: Vec<&str> = header.split_whitespace().collect();
        if header_toks.len() < 2 {
            return Err(format!("invalid header line: {header}"));
        }
        let cap_tok = header_toks[0].to_string();
        let n: usize = header_toks[1]
            .parse()
            .map_err(|_| format!("invalid n in header: {header}"))?;
        let opt_bins: Option<usize> = header_toks.get(2).and_then(|s| s.parse::<usize>().ok());

        let mut size_tokens: Vec<String> = Vec::with_capacity(n);
        while size_tokens.len() < n {
            let s = next_line(&mut idx).ok_or("unexpected EOF while reading item sizes")?;
            for tok in s.split_whitespace() {
                if decimal_places(tok).is_none() {
                    return Err(format!(
                        "non-numeric size token '{tok}' in {}",
                        path.display()
                    ));
                }
                size_tokens.push(tok.to_string());
                if size_tokens.len() == n {
                    break;
                }
            }
        }

        let mut scale = decimal_places(&cap_tok).ok_or("invalid capacity")?;
        for t in size_tokens.iter() {
            let d = decimal_places(t).ok_or("invalid item size")?;
            scale = scale.max(d);
        }
        // Avoid pathological scaling.
        if scale > 6 {
            return Err(format!("too many decimals (scale={scale}) in {}", path.display()));
        }

        let cap = parse_decimal_scaled_u32(&cap_tok, scale)?;
        if cap == 0 {
            return Err(format!("capacity must be > 0 in {}", path.display()));
        }

        let mut sizes: Vec<u32> = Vec::with_capacity(n);
        for t in size_tokens.iter() {
            let v = parse_decimal_scaled_u32(t, scale)?;
            if v == 0 {
                return Err(format!("item sizes must be > 0 in {}", path.display()));
            }
            if v > cap {
                return Err(format!(
                    "found item larger than capacity in {}: size={v} > capacity={cap}",
                    path.display()
                ));
            }
            sizes.push(v);
        }

        instances.push(Instance {
            name: format!("{stem}_{name}"),
            capacity: cap,
            sizes,
            opt_bins,
        });
    }

    Ok(instances)
}

pub fn load_bpp_instances_from_file(path: impl AsRef<Path>) -> Result<Vec<Instance>, String> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    if let Ok(inst) = parse_simple_single_instance(path, &content) {
        return Ok(vec![inst]);
    }
    if let Ok(insts) = parse_binpack_multi(path, &content) {
        return Ok(insts);
    }

    Err(format!(
        "Unrecognized dataset format in {}. Supported: simple integer instance, or BinPack multi-instance files.",
        path.display()
    ))
}

pub fn load_bpp_instances_from_dir(dir: impl AsRef<Path>) -> Result<Vec<Instance>, String> {
    let dir = dir.as_ref();
    let mut paths: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(dir).map_err(|e| format!("Failed to read dir {}: {e}", dir.display()))? {
        let ent = ent.map_err(|e| format!("Failed to read dir entry in {}: {e}", dir.display()))?;
        let p = ent.path();
        if p.is_file() {
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
            paths.push(p);
        }
    }
    paths.sort();

    let mut instances = Vec::new();
    let mut errors = Vec::new();
    for p in paths {
        match load_bpp_instances_from_file(&p) {
            Ok(insts) => instances.extend(insts),
            Err(e) => errors.push(e),
        }
    }

    if instances.is_empty() {
        if errors.is_empty() {
            return Err(format!("No files found in {}", dir.display()));
        }
        return Err(errors.join("\n"));
    }
    Ok(instances)
}

pub fn example_instance_tp2() -> Instance {
    Instance {
        name: "TP2-example".to_string(),
        capacity: 60,
        // Item sizes (1-indexed in the report): [22,17,45,12,38,27,19]
        sizes: vec![22, 17, 45, 12, 38, 27, 19],
        opt_bins: Some(4),
    }
}

pub fn synthetic_instance(
    name: &str,
    n_items: usize,
    capacity: u32,
    min_size: u32,
    max_size: u32,
    seed: u64,
) -> Instance {
    let mut rng = XorShift64::new(seed);
    let mut sizes = Vec::with_capacity(n_items);
    for _ in 0..n_items {
        sizes.push(rng.gen_range_u32(min_size, max_size));
    }
    Instance {
        name: name.to_string(),
        capacity,
        sizes,
        opt_bins: None,
    }
}

pub fn default_batch_instances() -> Vec<Instance> {
    vec![
        example_instance_tp2(),
        synthetic_instance("synthetic-60", 60, 150, 10, 100, 1),
        synthetic_instance("synthetic-120", 120, 150, 10, 100, 2),
        synthetic_instance("synthetic-200", 200, 150, 10, 100, 3),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_opt_bins_from_binpack_header() {
        let content = "\
1
 t1
 100.0 5 3
 10.0
 20.0
 30.0
 40.0
 50.0
";
        let path = Path::new("binpack_test.txt");
        let insts = parse_binpack_multi(path, content).unwrap();
        assert_eq!(insts.len(), 1);
        assert_eq!(insts[0].name, "binpack_test_t1");
        assert_eq!(insts[0].opt_bins, Some(3));
    }
}
