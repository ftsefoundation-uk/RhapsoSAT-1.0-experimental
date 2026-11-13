#!/bin/sh
//bin/true 2>/dev/null; d="$(dirname "$0")"; t="${TMPDIR:-/tmp}/prefsuf_hard"; [ -f "$d/prefsuf.rs" ] || { echo "put prefsuf.rs in the same folder as $0"; exit 1; }; mkdir -p "$t" && tail -n +3 "$d/prefsuf.rs" | sed 's/^fn main() {/fn orig_main() {/' > "$t/core.rs" && tail -n +3 "$0" > "$t/main.rs" && rustc --edition 2021 -O "$t/main.rs" -o "$t/run" && exec "$t/run" "$@"; exit $?
// Stress harness for prefsuf.rs. Keep prefsuf.rs in the same folder.
//
//     sh hard.rs                 seed 1, strings up to 500,000 positions
//     sh hard.rs 7               another seed
//     sh hard.rs 1 5000000       larger strings (needs several GB of RAM)
//
// The two lines at the top copy the solver code out of prefsuf.rs (dropping
// its shell lines and renaming its main), compile this harness against that
// copy, and run it. prefsuf.rs itself is not changed.
//
// Idea: no brute force. Each trial PLANTS a hidden solution and predicts the
// answer with code that shares nothing with the solver:
//   * components of positions are found by breadth-first search over the
//     border equalities (the solver uses union-find);
//   * the hidden string gets one random letter per component;
//   * Prefix/Suffix words are cut from the hidden string, so the instance
//     must be satisfiable and the hidden string must fit the solver's pattern;
//   * then one letter of one word is corrupted, and the harness predicts
//     from the components whether that must be UNSAT.
#![allow(dead_code)]
include!("core.rs");
use std::time::Instant;

struct X(u64);
impl X {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, m: usize) -> usize {
        (self.next() % m as u64) as usize
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Components by BFS over implicit edges i ~ i+p (valid iff i+p < n).
fn components(n: usize, periods: &[usize]) -> (Vec<u32>, usize) {
    let mut comp = vec![u32::MAX; n];
    let mut c = 0u32;
    let mut stack = Vec::new();
    for start in 0..n {
        if comp[start] != u32::MAX {
            continue;
        }
        comp[start] = c;
        stack.push(start);
        while let Some(u) = stack.pop() {
            for &p in periods {
                if u + p < n && comp[u + p] == u32::MAX {
                    comp[u + p] = c;
                    stack.push(u + p);
                }
                if u >= p && comp[u - p] == u32::MAX {
                    comp[u - p] = c;
                    stack.push(u - p);
                }
            }
        }
        c += 1;
    }
    (comp, c as usize)
}

/// Letter that constraint idx fixes at pos, straight from the definition.
fn literal(n: usize, cons: &[Con], idx: usize, pos: usize) -> Option<u8> {
    match &cons[idx] {
        Con::Prefix(w) => w.get(pos).copied(),
        Con::Suffix(w) => pos.checked_sub(n - w.len()).and_then(|o| w.get(o).copied()),
        Con::Border(_) => None,
    }
}

/// Certificate checker written separately from prefsuf.rs's check_conflict.
fn my_check(cf: &Conflict, n: usize, cons: &[Con]) -> bool {
    if cf.path.is_empty() || cf.via.len() + 1 != cf.path.len() || cf.letter_a == cf.letter_b {
        return false;
    }
    if cf.src_a >= cons.len() || cf.src_b >= cons.len() {
        return false;
    }
    if literal(n, cons, cf.src_a, cf.path[0]) != Some(cf.letter_a) {
        return false;
    }
    if literal(n, cons, cf.src_b, cf.path[cf.path.len() - 1]) != Some(cf.letter_b) {
        return false;
    }
    for i in 0..cf.via.len() {
        let (u, v) = (cf.path[i], cf.path[i + 1]);
        let (lo, hi) = if u < v { (u, v) } else { (v, u) };
        match cons.get(cf.via[i]) {
            Some(Con::Border(k)) if *k <= n && hi < n && hi - lo == n - *k => {}
            _ => return false,
        }
    }
    true
}

/// Does `hidden` fit the solver's pattern, with free ids <-> components one-to-one?
fn fits(pattern: &[Cell], free: usize, hidden: &[u8], comp: &[u32], fixed_comp: &[bool]) -> Result<(), String> {
    let mut id_to_comp = vec![u32::MAX; free];
    let mut comp_to_id = vec![usize::MAX; fixed_comp.len()]; // indexed by component
    for (i, c) in pattern.iter().enumerate() {
        match c {
            Cell::Letter(ch) => {
                if *ch != hidden[i] {
                    return Err(format!("position {i}: solver forces a letter the planted solution lacks"));
                }
                if !fixed_comp[comp[i] as usize] {
                    return Err(format!("position {i}: solver fixed a position that should be free"));
                }
            }
            Cell::Free(id) => {
                if fixed_comp[comp[i] as usize] {
                    return Err(format!("position {i}: solver left free a position that is forced"));
                }
                if id_to_comp[*id] == u32::MAX {
                    id_to_comp[*id] = comp[i];
                }
                if comp_to_id[comp[i] as usize] == usize::MAX {
                    comp_to_id[comp[i] as usize] = *id;
                }
                if id_to_comp[*id] != comp[i] || comp_to_id[comp[i] as usize] != *id {
                    return Err(format!("position {i}: free classes do not match components"));
                }
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct Stats {
    trials: usize,
    sat: usize,
    unsat: usize,
    corrupt_still_sat: usize,
    max_n: usize,
    max_m: usize,
    max_cert: usize,
    max_free: usize,
    max_classes: usize,
}

fn trial(r: &mut X, n: usize, st: &mut Stats) -> Result<(), String> {
    let k = [2usize, 2, 3, 5, 26, 94][r.below(6)];
    let al: Vec<u8> = (33u8..127).take(k).collect();

    // ---- borders: mixtures chosen to produce nested, overlapping, chained
    // and redundant structure without always collapsing to one class.
    let mut cons: Vec<Con> = Vec::new();
    let m = r.below(13);
    let g = 1 + r.below(1 + (n / 4).min(5000));
    for _ in 0..m {
        let p = match r.below(5) {
            0 => 1 + r.below(n - 1),                         // any period
            1 => (g * (1 + r.below(8))).min(n - 1).max(1),   // multiples of g (nested)
            2 => 1 + r.below(1 + n / 50),                    // tiny period, long chains
            3 => n - 1 - r.below(1 + n / 50).min(n - 2),     // huge period, short border
            _ => (n / 2 + r.below(3)).min(n - 1).max(1),     // around the half-way boundary
        };
        cons.push(Con::Border(n - p));
    }
    if r.below(4) == 0 {
        cons.push(Con::Border(0));
        cons.push(Con::Border(n));
    }
    let periods: Vec<usize> = cons
        .iter()
        .filter_map(|c| match c {
            Con::Border(b) if *b > 0 && *b < n => Some(n - *b),
            _ => None,
        })
        .collect();
    let (comp, ncomp) = components(n, &periods);

    // ---- planted solution
    let comp_letter: Vec<u8> = (0..ncomp).map(|_| al[r.below(k)]).collect();
    let hidden: Vec<u8> = comp.iter().map(|&c| comp_letter[c as usize]).collect();

    // ---- words cut from the planted solution
    for _ in 0..r.below(4) {
        let len = match r.below(4) {
            0 => r.below(n + 1),
            1 => r.below(1 + n / 20),
            2 => r.below(3),
            _ => n,
        };
        if r.below(2) == 0 {
            cons.push(Con::Prefix(hidden[..len].to_vec()));
        } else {
            cons.push(Con::Suffix(hidden[n - len..].to_vec()));
        }
    }
    // shuffle, then sometimes duplicate a constraint
    for i in (1..cons.len()).rev() {
        cons.swap(i, r.below(i + 1));
    }
    if !cons.is_empty() && r.below(3) == 0 {
        let c = cons[r.below(cons.len())].clone();
        cons.push(c);
    }

    // ---- independent prediction
    let mut occ = vec![0u32; ncomp]; // literal occurrences per component
    let mut lits: Vec<(usize, usize)> = Vec::new(); // (constraint, word offset)
    for (idx, c) in cons.iter().enumerate() {
        let (w, off) = match c {
            Con::Prefix(w) => (w, 0),
            Con::Suffix(w) => (w, n - w.len()),
            _ => continue,
        };
        for j in 0..w.len() {
            occ[comp[off + j] as usize] += 1;
            if lits.len() < 4096 || r.below(64) == 0 {
                lits.push((idx, j));
            }
        }
    }
    let fixed_comp: Vec<bool> = occ.iter().map(|&o| o > 0).collect();
    let want_free = fixed_comp.iter().filter(|f| !**f).count();

    // ---- solver on the planted instance: must be SAT and describe exactly
    // the predicted set.
    st.trials += 1;
    st.max_n = st.max_n.max(n);
    st.max_m = st.max_m.max(cons.len());
    st.max_classes = st.max_classes.max(ncomp);
    let out = solve(n, &al, &cons)?;
    let (pattern, free) = match &out {
        Outcome::Sat { pattern, free } => (pattern, *free),
        Outcome::Unsat(_) => return Err("planted instance reported UNSAT".into()),
    };
    st.sat += 1;
    st.max_free = st.max_free.max(free);
    if free != want_free {
        return Err(format!("free classes {free}, predicted {want_free}"));
    }
    fits(pattern, free, &hidden, &comp, &fixed_comp)?;
    if !satisfies(&witness(pattern, free, &al), n, &al, &cons) {
        return Err("witness violates constraints".into());
    }
    let fill: Vec<u8> = (0..free).map(|_| al[r.below(k)]).collect();
    if !satisfies(&instantiate(pattern, &fill), n, &al, &cons) {
        return Err("random filling of free classes violates constraints".into());
    }
    if is_unique(k, free) != (free == 0 || k == 1) {
        return Err("uniqueness flag wrong".into());
    }

    // ---- order independence: reversed constraint list gives the same pattern
    let rev: Vec<Con> = cons.iter().rev().cloned().collect();
    if solve(n, &al, &rev)? != out {
        return Err("answer depends on constraint order".into());
    }

    // ---- corruption: change one letter of one word
    if !lits.is_empty() && k >= 2 {
        let (idx, j) = lits[r.below(lits.len())];
        let mut bad = cons.clone();
        let (pos, old) = match &mut bad[idx] {
            Con::Prefix(w) => {
                let old = w[j];
                w[j] = al[(al.iter().position(|a| *a == old).unwrap() + 1 + r.below(k - 1)) % k];
                (j, old)
            }
            Con::Suffix(w) => {
                let old = w[j];
                let off = n - w.len();
                w[j] = al[(al.iter().position(|a| *a == old).unwrap() + 1 + r.below(k - 1)) % k];
                (off + j, old)
            }
            _ => unreachable!(),
        };
        let must_be_unsat = occ[comp[pos] as usize] > 1;
        match solve(n, &al, &bad)? {
            Outcome::Unsat(cf) => {
                st.unsat += 1;
                st.max_cert = st.max_cert.max(cf.path.len());
                if !must_be_unsat {
                    return Err("corrupted instance reported UNSAT but is satisfiable".into());
                }
                if !my_check(&cf, n, &bad) {
                    return Err("certificate rejected by independent checker".into());
                }
                if !check_conflict(&cf, n, &bad) {
                    return Err("certificate rejected by built-in checker".into());
                }
            }
            Outcome::Sat { pattern, free } => {
                st.corrupt_still_sat += 1;
                if must_be_unsat {
                    return Err("corrupted instance reported SAT but must be UNSAT".into());
                }
                if free != want_free {
                    return Err("free count changed after harmless corruption".into());
                }
                let w = witness(&pattern, free, &al);
                if !satisfies(&w, n, &al, &bad) || w[pos] == old {
                    return Err("witness wrong after harmless corruption".into());
                }
            }
        }
    }
    Ok(())
}

/// Fine-Wilf prediction, independent of both the solver and the BFS:
/// periods p, q with p + q - gcd(p, q) <= n leave exactly gcd(p, q) classes.
fn fine_wilf(r: &mut X, n: usize) -> Result<(), String> {
    let p = 1 + r.below(n / 2);
    let q = 1 + r.below(n / 2);
    let g = gcd(p, q);
    if p + q - g > n {
        return Ok(());
    }
    match solve(n, b"ab", &[Con::Border(n - p), Con::Border(n - q)])? {
        Outcome::Sat { free, .. } if free == g => Ok(()),
        other => Err(format!("Fine-Wilf: n={n} p={p} q={q} expected {g} classes, got {:?}",
            match other { Outcome::Sat { free, .. } => free, _ => usize::MAX })),
    }
}

fn main() {
    let seed: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let max_n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(500_000);
    let mut r = X(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    // (trials, min n, max n)
    let plan: [(usize, usize, usize); 6] = [
        (20000, 2, 40),
        (5000, 40, 2000),
        (600, 2000, 50_000),
        (60, 50_000, 500_000),
        (12, 500_000, 2_000_000),
        (3, 4_000_000, 5_000_000),
    ];
    let mut failures = 0;
    for (trials, lo, hi) in plan {
        if lo >= max_n {
            println!("n {lo}..{hi}: skipped (limit {max_n}; pass a larger second argument to run)");
            continue;
        }
        let hi = hi.min(max_n);
        let mut st = Stats::default();
        let t = Instant::now();
        for i in 0..trials {
            let n = lo + r.below(hi - lo + 1);
            if let Err(e) = trial(&mut r, n, &mut st) {
                failures += 1;
                println!("FAIL tier {lo}..{hi} trial {i} n={n}: {e}");
                if failures > 5 { std::process::exit(1); }
            }
        }
        let mut fw = 0;
        for _ in 0..trials.min(2000) {
            let n = lo.max(4) + r.below(hi - lo + 1);
            if let Err(e) = fine_wilf(&mut r, n) { failures += 1; println!("FAIL {e}"); }
            fw += 1;
        }
        println!(
            "n {lo}..{hi}: {} planted trials ({} SAT checked, {} corrupted->UNSAT, {} corrupted->still SAT), {fw} Fine-Wilf checks; largest n {}, most constraints {}, most classes {}, most free {}, longest certificate {}; {:.1}s",
            st.trials, st.sat, st.unsat, st.corrupt_still_sat, st.max_n, st.max_m, st.max_classes, st.max_free, st.max_cert, t.elapsed().as_secs_f64()
        );
    }
    println!("failures: {failures}");
    std::process::exit(if failures == 0 { 0 } else { 1 });
}
