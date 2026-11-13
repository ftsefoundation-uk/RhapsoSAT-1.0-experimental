#!/bin/sh
//bin/true 2>/dev/null; d="$(dirname "$0")"; t="${TMPDIR:-/tmp}/prefsuf_reduce"; [ -f "$d/prefsuf.rs" ] || { echo "put prefsuf.rs in the same folder as $0"; exit 1; }; mkdir -p "$t" && tail -n +3 "$d/prefsuf.rs" | sed 's/^fn main() {/fn orig_main() {/' > "$t/core.rs" && tail -n +3 "$0" > "$t/main.rs" && rustc --edition 2021 -O "$t/main.rs" -o "$t/run" && exec "$t/run" "$@"; exit $?
// Reduction laboratory for prefsuf.rs (docs/research/06): does reducing
// the border set BEFORE building groups pay, and what does a derived
// explanation cost? Keep prefsuf.rs in the same folder.
//
//     sh reduce_lab.rs [check [SEED]]   correctness of every strategy (about 20 s)
//     sh reduce_lab.rs bench [N]        grouping strategies compared (default N = 2,000,000)
//     sh reduce_lab.rs proofs [N]       explanation strategies compared (default N = 1,000,000)
//     sh reduce_lab.rs scale SHAPE      one shape at growing sizes
//     sh reduce_lab.rs phases [N]       search for border sets with many phases
//
// Grouping strategies:
//   naive      union-find over every link of every Border rule
//   current    equal_positions of prefsuf.rs (border-of-border rounds)
//   phases     reduction first: the phases only, no array as long as the
//              text; gives the number of groups and a compact description
//   labels     reduction first, then one pass over the text per phase
//   generators reduction first, then union-find over the links of the
//              generator borders only (also yields a spanning forest, i.e.
//              a partition certificate)
//   query      no array at all: the group of one position from the phases
//
// A failed check prints FAIL and the exit status is 1.
#![allow(dead_code)]
include!("core.rs");
use std::time::Instant;

struct Lcg(u64);
impl Lcg {
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

struct Report {
    fails: usize,
}
impl Report {
    fn must(&mut self, ok: bool, what: &str) {
        if ok {
            println!("  ok    {what}");
        } else {
            self.fails += 1;
            println!("  FAIL  {what}");
        }
    }
}

// ----------------------------------------------------------- strategies

fn top(group: &mut [u32], mut x: u32) -> u32 {
    while group[x as usize] != x {
        group[x as usize] = group[group[x as usize] as usize];
        x = group[x as usize];
    }
    x
}

/// Union-find over the links of the given periods, smallest position on
/// top. Returns (root of every position, links looked at, links kept).
fn union_all(n: usize, periods: &[usize]) -> (Vec<usize>, u64, u64) {
    let mut group: Vec<u32> = (0..n as u32).collect();
    let (mut looked, mut kept) = (0u64, 0u64);
    for &q in periods {
        for i in 0..n - q {
            looked += 1;
            let (x, y) = (top(&mut group, i as u32), top(&mut group, (i + q) as u32));
            if x != y {
                group[x.max(y) as usize] = x.min(y);
                kept += 1;
            }
        }
    }
    ((0..n).map(|i| top(&mut group, i as u32) as usize).collect(), looked, kept)
}

fn all_periods(n: usize, cons: &[Con]) -> Vec<usize> {
    border_periods(n, cons).iter().map(|x| x.0).collect()
}

/// Reduction first, then one pass per phase: the same links as
/// `equal_positions` writes, taken from the phase list.
fn labels_from_phases(n: usize, phases: &[Phase]) -> (Vec<usize>, u64) {
    let mut root: Vec<usize> = (0..n).collect();
    let mut written = 0u64;
    for ph in phases {
        let end = ph.start - ph.count * ph.p;
        for j in ph.p.max(end)..ph.start {
            root[j] = j - ph.p;
        }
        written += (ph.start - ph.p.max(end)) as u64;
    }
    for i in 0..n {
        root[i] = root[root[i]];
    }
    (root, written)
}

/// The smallest position forced equal to position i, from the phases
/// alone: O(number of phases) arithmetic, no array.
fn root_query(phases: &[Phase], i: usize) -> usize {
    let mut j = i;
    for ph in phases {
        let end = ph.start - ph.count * ph.p;
        if j >= ph.start {
            continue;
        }
        if j >= end {
            // step down by p until below `end`, but never below 0
            let steps = ((j - end) / ph.p + 1).min(j / ph.p);
            j -= steps * ph.p;
            if j >= end {
                return j; // an unlinked position of this phase
            }
        }
    }
    j
}

// --------------------------------------------------------------- shapes

const SHAPES: [&str; 14] = ["free", "half", "p2p3", "fwpath", "rand16", "rand1000", "tiny1000", "allborders", "evenhalf", "ww3", "mult4_2", "fib", "smallperiods", "threequarters"];

/// Border sets by name. Sparse: free, half, p2p3, fwpath, rand16.
/// Dense: rand1000, tiny1000, allborders, evenhalf, threequarters.
/// Pathological: ww3, mult4_2, fib, smallperiods.
fn shape(name: &str, n: usize) -> Vec<Con> {
    let mut r = Lcg(0x9E3779B97F4A7C15 ^ n as u64);
    let h = n / 2;
    let borders: Vec<usize> = match name {
        "free" => vec![],
        "half" => vec![h],
        "p2p3" => vec![n - 2, n - 3],
        // two coprime periods exactly at the Fine-Wilf bound
        "fwpath" => vec![n - h, n - h - 1],
        "rand16" => (0..16).map(|_| 1 + r.below(n - 1)).collect(),
        "rand1000" => (0..1000).map(|_| 1 + r.below(n - 1)).collect(),
        "tiny1000" => (1..=1000.min(n - 1)).collect(),
        "allborders" => (1..n).collect(),
        "evenhalf" => (h..n).filter(|q| q % 2 == 0).map(|q| n - q).collect(),
        "threequarters" => (1..n).filter(|k| k % 4 != 0).collect(),
        "ww3" => {
            let hh = if h % 3 == 0 { h - 1 } else { h };
            (0..hh / 3).map(|j| hh - 3 * j).chain([n - hh]).collect()
        }
        // periods 4, 8, 12, ... and 2 (the slow case of the first study)
        "mult4_2" => (1..n / 4).map(|j| n - 4 * j).chain([n - 2]).collect(),
        // periods are Fibonacci numbers: Euclid's worst case
        "fib" => {
            let (mut a, mut b, mut out) = (1usize, 2usize, vec![]);
            while b < n {
                out.push(n - b);
                (a, b) = (b, a + b);
            }
            out
        }
        "smallperiods" => (0..2000).map(|_| n - 1 - r.below(5000.min(n - 1))).collect(),
        _ => {
            eprintln!("unknown shape {name}; known: {SHAPES:?}");
            std::process::exit(2);
        }
    };
    borders.into_iter().filter(|k| *k > 0 && *k < n).map(Con::Border).collect()
}

// ---------------------------------------------------------------- check

fn check(seed: u64) -> usize {
    let mut r = Report { fails: 0 };
    let mut rng = Lcg(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    println!("R1  every grouping strategy gives the same groups (seed {seed})");
    let (mut cases, mut naive_bad, mut labels_bad, mut gen_bad, mut query_bad, mut count_bad, mut forest_bad, mut bound_bad) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut most_phases = 0;
    let mut most_gens = 0;
    for round in 0..30_000 {
        let n = 1 + rng.below(match round % 10 {
            0 => 4000,
            1 | 2 => 600,
            _ => 90,
        });
        let m = rng.below(if round % 6 == 0 { 60 } else { 8 });
        let cons: Vec<Con> = (0..m)
            .map(|_| match round % 5 {
                0 => Con::Border(n - 1 - rng.below(n.min(9))),
                1 => Con::Border(rng.below(n.min(9) + 1)),
                _ => Con::Border(rng.below(n + 1)),
            })
            .collect();
        cases += 1;
        let current = equal_positions(n, &cons).unwrap();
        let (naive, _, _) = union_all(n, &all_periods(n, &cons));
        naive_bad += (naive != current) as u64;
        let phases = reduction_phases(n, &cons);
        most_phases = most_phases.max(phases.len());
        bound_bad += (phases.len() > 2 * (usize::BITS - 1 - n.leading_zeros()) as usize + 2) as u64;
        let (labels, _) = labels_from_phases(n, &phases);
        labels_bad += (labels != current) as u64;
        let gens: Vec<usize> = generator_periods(n, &cons).iter().map(|x| x.0).collect();
        most_gens = most_gens.max(gens.len());
        let (by_gens, _, kept) = union_all(n, &gens);
        gen_bad += (by_gens != current) as u64;
        let mut distinct = current.clone();
        distinct.sort_unstable();
        distinct.dedup();
        forest_bad += (kept as usize != n - distinct.len()) as u64;
        count_bad += (groups_from_phases(n, &phases) != distinct.len()) as u64;
        query_bad += (0..n).any(|i| root_query(&phases, i) != current[i]) as u64;
    }
    r.must(naive_bad == 0, &format!("current grouping equals union-find over every link ({cases} border sets)"));
    r.must(labels_bad == 0, "reduction first, then one pass per phase: same groups");
    r.must(gen_bad == 0, "reduction first, then union-find over generator links only: same groups");
    r.must(forest_bad == 0, "the generator forest has exactly (positions - groups) links");
    r.must(count_bad == 0, "number of groups from the phases alone");
    r.must(query_bad == 0, "the group of every single position from the phases alone, without any array");
    r.must(bound_bad == 0, &format!("at most 2*log2(n)+2 phases (most seen: {most_phases} phases, {most_gens} generators)"));

    println!("R2  the named shapes, at several sizes");
    let mut bad = 0;
    for name in SHAPES {
        for n in [16usize, 97, 1000, 20_000] {
            let cons = shape(name, n);
            let current = equal_positions(n, &cons).unwrap();
            let phases = reduction_phases(n, &cons);
            let gens: Vec<usize> = generator_periods(n, &cons).iter().map(|x| x.0).collect();
            let ok = union_all(n, &all_periods(n, &cons)).0 == current
                && labels_from_phases(n, &phases).0 == current
                && union_all(n, &gens).0 == current
                && (0..n).all(|i| root_query(&phases, i) == current[i])
                && phases == Derivation::new(n, &cons).phases;
            bad += !ok as u64;
        }
    }
    r.must(bad == 0, &format!("{} shapes x 4 sizes: all strategies agree", SHAPES.len()));

    println!("R3  explanations (chains of original links)");
    let (mut pairs, mut gen_invalid, mut old_invalid, mut not_shortest_possible) = (0u64, 0u64, 0u64, 0u64);
    let (mut shortened_total, mut shortest_total, mut exact) = (0u64, 0u64, 0u64);
    for round in 0..6000 {
        let n = 2 + rng.below(if round % 10 == 0 { 3000 } else { 120 });
        let m = 1 + rng.below(if round % 6 == 0 { 40 } else { 6 });
        let cons: Vec<Con> = (0..m).map(|_| Con::Border(rng.below(n + 1))).collect();
        let root = equal_positions(n, &cons).unwrap();
        let (a, b) = (rng.below(n), rng.below(n));
        let periods = border_periods(n, &cons);
        let valid = |path: &[usize], via: &[usize]| {
            path.first() == Some(&a)
                && path.last() == Some(&b)
                && via.len() + 1 == path.len()
                && path.windows(2).zip(via).all(|(w, idx)| match cons[*idx] {
                    Con::Border(k) => k > 0 && k < n && w[0].abs_diff(w[1]) == n - k,
                    _ => false,
                })
        };
        let g = generator_path(n, &cons, a, b);
        let o = derive_path(n, &cons, a, b);
        if root[a] != root[b] {
            // not forced equal: no strategy may produce a chain
            gen_invalid += g.is_some() as u64;
            old_invalid += o.is_some() as u64;
            continue;
        }
        pairs += 1;
        let shortest = bfs_path(n, &periods, a, b, u64::MAX).unwrap().0.len();
        match g {
            Some((path, via)) => {
                gen_invalid += !valid(&path, &via) as u64;
                not_shortest_possible += (path.len() < shortest) as u64;
                let before = path.len();
                let (p2, v2) = shorten_path(n, &cons, path, via);
                gen_invalid += !valid(&p2, &v2) as u64;
                gen_invalid += (p2.len() > before || p2.len() < shortest) as u64;
                shortened_total += p2.len() as u64;
                shortest_total += shortest as u64;
                exact += (p2.len() == shortest) as u64;
            }
            None => gen_invalid += 1,
        }
        match o {
            Some((path, via)) => old_invalid += !valid(&path, &via) as u64,
            None => old_invalid += 1,
        }
    }
    r.must(gen_invalid == 0, &format!("chains through generator borders are valid, before and after shortening, and shortening never lengthens ({pairs} pairs of equal positions); none exists for unequal positions"));
    r.must(old_invalid == 0, "chains from the earlier derivation are valid on the same pairs");
    println!("    observation: after shortening, {exact} of {pairs} chains are as short as the shortest; total positions {shortened_total} against {shortest_total} for the shortest chains");
    r.must(not_shortest_possible == 0, "no chain is shorter than the breadth-first shortest (sanity of the reference)");

    println!("R4  how many generators? (docs/research/07)");
    // lower bound (PROVED): borders n/2, n/4, ..., 1 for n = 2^k give k generators, and every one is needed
    let (mut fam_ok, mut needed_ok) = (true, true);
    for k in 1..=40usize {
        let n = 1usize << k;
        let cons: Vec<Con> = (0..k).map(|i| Con::Border(n >> (i + 1))).collect();
        fam_ok &= generator_periods(n, &cons).len() == k && groups_from_phases(n, &reduction_phases(n, &cons)) == 1;
        for drop in 0..k {
            let fewer: Vec<Con> = cons.iter().enumerate().filter(|(i, _)| *i != drop).map(|(_, c)| c.clone()).collect();
            needed_ok &= groups_from_phases(n, &reduction_phases(n, &fewer)) > 1;
        }
    }
    r.must(fam_ok, "n = 2^k with borders n/2, n/4, ..., 1 has exactly k = log2(n) generators and one group (k = 1..40)");
    r.must(needed_ok, "in that family no border can be left out: every smaller set of the borders leaves more than one group");
    // upper bound (CONJECTURE): never more than floor(log2 n) generators
    let (mut sets, mut over, mut attained) = (0u64, 0u64, true);
    for n in 2..=17usize {
        let mut best = 0;
        for mask in 0u32..(1 << (n - 1)) {
            let cons: Vec<Con> = (1..n).filter(|k| mask >> (k - 1) & 1 == 1).map(Con::Border).collect();
            let g = generator_periods(n, &cons).len();
            sets += 1;
            best = best.max(g);
            over += (g > (usize::BITS - 1 - n.leading_zeros()) as usize) as u64;
        }
        attained &= best == (usize::BITS - 1 - n.leading_zeros()) as usize;
    }
    for round in 0..400_000 {
        let n = 2 + rng.below([60, 5000, 1_000_000, 1usize << 40][round % 4]);
        let m = 1 + rng.below(if round % 3 == 0 { 30 } else { 6 });
        let cons: Vec<Con> = (0..m)
            .map(|_| match round % 5 {
                0 => Con::Border(n - 1 - rng.below(n.min(40) - 1)),
                1 => Con::Border(1 + rng.below(n.min(40) - 1)),
                2 => Con::Border((n >> (1 + rng.below(40))).max(1).min(n - 1)),
                _ => Con::Border(1 + rng.below(n - 1)),
            })
            .collect();
        sets += 1;
        over += (generator_periods(n, &cons).len() > (usize::BITS - 1 - n.leading_zeros()) as usize) as u64;
    }
    r.must(over == 0, &format!("conjecture, not a theorem: no border set has more than floor(log2 n) generators ({sets} sets: every set for n <= 17, random ones up to n = 2^40)"));
    r.must(attained, "for every n from 2 to 17 some border set has exactly floor(log2 n) generators");
    println!("reduction checks failed: {}", r.fails);
    r.fails
}

// ---------------------------------------------------------------- bench

fn mb(bytes: usize) -> String {
    format!("{:.1}", bytes as f64 / 1048576.0)
}

fn bench(n: usize) {
    println!("Grouping strategies, n = {n}. Times in seconds (single runs). Memory is the size of the arrays each strategy needs, in MB.");
    println!("'links' = relationships looked at; 'written' = positions written; forest = positions - groups (the same for every strategy).");
    println!(
        "{:<14} {:>8} {:>9} | {:>9} {:>13} | {:>8} {:>9} | {:>9} {:>6} {:>4} | {:>8} | {:>8} {:>12} | {:>9}",
        "shape", "borders", "groups", "naive s", "naive links", "curr s", "written", "phases s", "phases", "gen", "labels s", "gens s", "gen links", "forest"
    );
    for name in SHAPES {
        let cons = shape(name, n);
        let periods = all_periods(n, &cons);
        let total_links: u128 = periods.iter().map(|q| (n - q) as u128).sum();
        let t = Instant::now();
        let current = equal_positions(n, &cons).unwrap();
        let t_cur = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let phases = reduction_phases(n, &cons);
        let groups = groups_from_phases(n, &phases);
        let t_ph = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let (labels, written) = labels_from_phases(n, &reduction_phases(n, &cons));
        let t_lab = t.elapsed().as_secs_f64();
        let gens: Vec<usize> = generator_periods(n, &cons).iter().map(|x| x.0).collect();
        let t = Instant::now();
        let (by_gens, gen_links, kept) = union_all(n, &gens);
        let t_gen = t.elapsed().as_secs_f64();
        let (naive_s, naive_links) = if total_links <= 400_000_000 {
            let t = Instant::now();
            let (naive, looked, _) = union_all(n, &periods);
            assert!(naive == current, "naive differs");
            (format!("{:.3}", t.elapsed().as_secs_f64()), looked.to_string())
        } else {
            ("skipped".to_string(), format!("{:.1e}", total_links as f64))
        };
        assert!(labels == current && by_gens == current, "strategies differ on {name}");
        let mut probe = Lcg(7);
        assert!((0..2000).all(|_| { let i = probe.below(n); root_query(&phases, i) == current[i] }), "query differs on {name}");
        println!(
            "{:<14} {:>8} {:>9} | {:>9} {:>13} | {:>8.4} {:>9} | {:>9.6} {:>6} {:>4} | {:>8.4} | {:>8.4} {:>12} | {:>9}",
            name, periods.len(), groups, naive_s, naive_links, t_cur, written, t_ph, phases.len(), gens.len(), t_lab, t_gen, gen_links, kept
        );
    }
    println!("Array memory at this n: current and labels {} MB (one word per position, plus the border list); naive and generators {} MB (32-bit union-find); phases and query: the border list plus {} bytes per phase.", mb(n * 8), mb(n * 4), std::mem::size_of::<Phase>());
}

fn proofs(n: usize) {
    println!("Explanations between position 0 and the last position of its group, n = {n}.");
    println!("search = breadth-first over every border (shortest chain; given up beyond 300,000,000 link tests);");
    println!("gen = breadth-first over generator borders only (bound: 2 n g link tests); old = the earlier derivation. Lengths are positions in the chain, raw -> after shortening.");
    println!("{:<14} {:>8} {:>4} | {:>9} {:>9} | {:>8} {:>19} {:>13} | {:>8} {:>19}", "shape", "borders", "g", "search s", "shortest", "gen s", "gen raw -> short", "bound 2ng", "old s", "old raw -> short");
    for name in SHAPES {
        let cons = shape(name, n);
        let root = equal_positions(n, &cons).unwrap();
        let Some(dst) = (1..n).rev().find(|i| root[*i] == root[0]) else {
            println!("{name:<14} position 0 is alone in its group");
            continue;
        };
        let periods = border_periods(n, &cons);
        let g = generator_periods(n, &cons).len();
        let t = Instant::now();
        let search = bfs_path(n, &periods, 0, dst, 300_000_000);
        let t_s = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let (gp, gv) = generator_path(n, &cons, 0, dst).expect("generator path");
        let t_g = t.elapsed().as_secs_f64();
        let graw = gp.len();
        let gshort = shorten_path(n, &cons, gp, gv).0.len();
        let t = Instant::now();
        let (op, ov) = derive_path(n, &cons, 0, dst).expect("derived path");
        let t_o = t.elapsed().as_secs_f64();
        let oraw = op.len();
        let oshort = shorten_path(n, &cons, op, ov).0.len();
        println!(
            "{:<14} {:>8} {:>4} | {:>9.4} {:>9} | {:>8.4} {:>19} {:>13} | {:>8.4} {:>19}",
            name,
            periods.len(),
            g,
            t_s,
            search.map_or("gave up".to_string(), |x| x.0.len().to_string()),
            t_g,
            format!("{graw} -> {gshort}"),
            2 * n * g,
            t_o,
            format!("{oraw} -> {oshort}")
        );
    }
}

fn scale(name: &str) {
    println!("{name}: growth with n (seconds). gen/n is generator-path time per million positions.");
    println!("{:>10} {:>8} {:>6} {:>4} | {:>9} {:>10} {:>9} | {:>9} {:>9} {:>9}", "n", "borders", "phases", "g", "current", "phases", "gen union", "gen path", "per 1e6", "old path");
    for n in [125_000usize, 250_000, 500_000, 1_000_000, 2_000_000, 4_000_000, 8_000_000] {
        let cons = shape(name, n);
        let t = Instant::now();
        let root = equal_positions(n, &cons).unwrap();
        let t_cur = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let phases = reduction_phases(n, &cons);
        let t_ph = t.elapsed().as_secs_f64();
        let gens: Vec<usize> = generator_periods(n, &cons).iter().map(|x| x.0).collect();
        let t = Instant::now();
        let (by_gens, _, _) = union_all(n, &gens);
        let t_gu = t.elapsed().as_secs_f64();
        assert!(by_gens == root);
        let dst = (0..n).rev().find(|i| root[*i] == root[0]).unwrap();
        let t = Instant::now();
        let gp = generator_path(n, &cons, 0, dst).unwrap();
        let t_gp = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let op = derive_path(n, &cons, 0, dst).unwrap();
        let t_op = t.elapsed().as_secs_f64();
        let _ = (gp, op);
        println!("{:>10} {:>8} {:>6} {:>4} | {:>9.4} {:>10.6} {:>9.4} | {:>9.4} {:>9.4} {:>9.4}", n, cons.len(), phases.len(), gens.len(), t_cur, t_ph, t_gu, t_gp, t_gp / (n as f64 / 1e6), t_op);
    }
}

/// Hill-climbing search for border sets with many phases / generators.
fn hunt_phases(n: usize) {
    let bound = 2 * (usize::BITS - 1 - n.leading_zeros()) as usize + 2;
    let mut rng = Lcg(12345);
    let mut best: (usize, usize, Vec<usize>) = (0, 0, vec![]);
    for start in 0..40 {
        let mut set: Vec<usize> = if start == 0 {
            shape("fib", n).iter().map(|c| match c { Con::Border(k) => *k, _ => 0 }).collect()
        } else {
            (0..1 + rng.below(12)).map(|_| 1 + rng.below(n - 1)).collect()
        };
        let score = |set: &[usize]| {
            let cons: Vec<Con> = set.iter().map(|k| Con::Border(*k)).collect();
            let ph = reduction_phases(n, &cons);
            (ph.len(), ph.iter().filter(|p| matches!(p.origin, Origin::Rule(_))).count())
        };
        let mut cur = score(&set);
        for _ in 0..4000 {
            let mut cand = set.clone();
            match rng.below(3) {
                0 => cand.push(1 + rng.below(n - 1)),
                1 if !cand.is_empty() => {
                    let i = rng.below(cand.len());
                    cand[i] = 1 + rng.below(n - 1);
                }
                _ if cand.len() > 1 => {
                    let i = rng.below(cand.len());
                    cand.remove(i);
                }
                _ => {}
            }
            let s = score(&cand);
            if s >= cur {
                (set, cur) = (cand, s);
            }
        }
        if cur > (best.0, best.1) {
            best = (cur.0, cur.1, set);
        }
    }
    println!("n = {n}: proved bound {bound} phases; most found by search: {} phases, {} of them started by a Border rule", best.0, best.1);
    let mut b = best.2.clone();
    b.sort_unstable();
    b.dedup();
    println!("  with {} borders{}", b.len(), if b.len() <= 30 { format!(": {b:?}") } else { String::new() });
    let fib = shape("fib", n);
    let ph = reduction_phases(n, &fib);
    println!("  Fibonacci periods ({} borders): {} phases", fib.len(), ph.len());
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |i: usize, default: usize| args.get(i).and_then(|s| s.replace('_', "").parse().ok()).unwrap_or(default);
    match args.first().map(|s| s.as_str()) {
        None | Some("check") => std::process::exit(if check(num(1, 1) as u64) == 0 { 0 } else { 1 }),
        Some("bench") => bench(num(1, 2_000_000)),
        Some("proofs") => proofs(num(1, 1_000_000)),
        Some("scale") => scale(args.get(1).map(|s| s.as_str()).unwrap_or("mult4_2")),
        Some("phases") => hunt_phases(num(1, 1_000_000)),
        _ => {
            eprintln!("usage: [check [SEED]] | bench [N] | proofs [N] | scale SHAPE | phases [N]");
            std::process::exit(2);
        }
    }
}
