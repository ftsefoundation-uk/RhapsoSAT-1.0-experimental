#!/bin/sh
//bin/true 2>/dev/null; d="$(dirname "$0")"; t="${TMPDIR:-/tmp}/prefsuf_theory"; [ -f "$d/prefsuf.rs" ] || { echo "put prefsuf.rs in the same folder as $0"; exit 1; }; mkdir -p "$t" && tail -n +3 "$d/prefsuf.rs" | sed 's/^fn main() {/fn orig_main() {/' > "$t/core.rs" && tail -n +3 "$0" > "$t/main.rs" && rustc --edition 2021 -O "$t/main.rs" -o "$t/run" && exec "$t/run" "$@"; exit $?
// Theory laboratory for prefsuf.rs: machine checks of the statements in
// docs/research/. Keep prefsuf.rs in the same folder.
//
//     sh theory_lab.rs            every check below (about a minute)
//     sh theory_lab.rs big N      partition certificates on texts of length N
//     sh theory_lab.rs table      group counts of irreducible two-shift grids
//
// What is checked, and against what:
//
//   E1  General reduction theorem (docs/research/02): for finite sets of
//       points in Z^d and any shift p, the reduced system is always a
//       relaxation; it is exact whenever the domination condition holds;
//       for boxes the domination condition equals a coordinate formula.
//       Checked against plain union-find over every link.
//   E2  Reduction with a mirror constraint (palindromes), 1-D.
//   E3  Weighted shifts: offsets as equality semantics (exact), offsets as
//       inequalities (counterexample), weights as costs (counterexample).
//   E4  Grids: how often repeated reduction reaches the trivial system.
//   E5  Partition certificates for the 1-D solver: emitted from the
//       solver's derivation, checked by code that knows only the rules
//       (link validity, union-find, and the classical border table).
//
// A failed check prints FAIL and the exit status is 1. Statistics that are
// observations (not theorems) are printed without a verdict.
#![allow(dead_code)]
include!("core.rs");
use std::collections::HashMap;
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
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as usize) as i32
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

// ----------------------------------------------------------- union-find

fn uf_find(u: &mut [usize], mut x: usize) -> usize {
    while u[x] != x {
        u[x] = u[u[x]];
        x = u[x];
    }
    x
}
fn uf_union(u: &mut [usize], a: usize, b: usize) {
    let (a, b) = (uf_find(u, a), uf_find(u, b));
    if a != b {
        u[a.max(b)] = a.min(b);
    }
}
/// label[i] = smallest index in the group of i
fn uf_labels(u: &mut [usize]) -> Vec<usize> {
    (0..u.len()).map(|i| uf_find(u, i)).collect()
}
fn count_groups(labels: &[usize]) -> usize {
    labels.iter().enumerate().filter(|(i, l)| *i == **l).count()
}

// ------------------------------------- shift systems on finite point sets
//
// A system is a finite set D of points of Z^d and a finite set P of
// non-zero vectors. Its constraints are x_v = x_{v+q} for every q in P and
// every v with v and v + q both in D.

type Pt = Vec<i32>;

#[derive(Clone)]
struct Sys {
    pts: Vec<Pt>,
    index: HashMap<Pt, usize>,
    shifts: Vec<Pt>,
}

fn add(a: &Pt, b: &Pt) -> Pt {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}
fn sub(a: &Pt, b: &Pt) -> Pt {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}
fn neg(a: &Pt) -> Pt {
    a.iter().map(|x| -x).collect()
}
/// q and -q give the same constraints: make the first non-zero entry positive.
fn normal(a: &Pt) -> Pt {
    match a.iter().find(|x| **x != 0) {
        Some(x) if *x < 0 => neg(a),
        _ => a.clone(),
    }
}

fn sys_of(pts: Vec<Pt>, shifts: Vec<Pt>) -> Sys {
    let index = pts.iter().cloned().enumerate().map(|(i, p)| (p, i)).collect();
    Sys { pts, index, shifts }
}

fn box_points(dims: &[i32]) -> Vec<Pt> {
    let mut out: Vec<Pt> = vec![vec![]];
    for &n in dims {
        out = out.into_iter().flat_map(|p| (0..n).map(move |c| { let mut q = p.clone(); q.push(c); q })).collect();
    }
    out
}

impl Sys {
    fn has(&self, v: &Pt) -> Option<usize> {
        self.index.get(v).copied()
    }
    /// Groups forced by all constraints, by brute force.
    fn groups(&self) -> Vec<usize> {
        let mut u: Vec<usize> = (0..self.pts.len()).collect();
        for q in &self.shifts {
            for (i, v) in self.pts.iter().enumerate() {
                if let Some(j) = self.has(&add(v, q)) {
                    uf_union(&mut u, i, j);
                }
            }
        }
        uf_labels(&mut u)
    }
    /// "p splits q" for this orientation of q: whenever v and v + q are in
    /// D, so are v + p and v + q - p.
    fn splits(&self, p: &Pt, q: &Pt) -> bool {
        self.pts.iter().all(|v| {
            let w = add(v, q);
            self.has(&w).is_none() || (self.has(&add(v, p)).is_some() && self.has(&sub(&w, p)).is_some())
        })
    }
    /// The orientation of q (q or -q) that p splits, if there is one.
    /// q and -q describe the same constraints, but the reduced shift
    /// q - p depends on the choice, and only this choice is covered by the
    /// theorem.
    fn oriented(&self, p: &Pt, q: &Pt) -> Option<Pt> {
        [q.clone(), neg(q)].into_iter().find(|q| self.splits(p, q))
    }
    /// The domination condition for shift p, straight from its definition:
    /// every other shift has an orientation that p splits.
    fn dominated_by_definition(&self, p: &Pt) -> bool {
        self.shifts.iter().filter(|q| normal(q) != normal(p)).all(|q| self.oriented(p, q).is_some())
    }
}

struct Reduced {
    groups: Vec<usize>, // groups of D forced by the reduced description
    small: Sys,         // the system (D', P')
    free: usize,        // points in no constraint of the reduced description
}

/// Reduction by shift p:
///   D' = { v in D : v + p in D },   P' = { q - p : q in P, q != p } + { p },
///   where each q is taken in the orientation that p splits (if neither
///   orientation is split, q is taken as given and exactness is not promised),
///   (a) x_t = x_{t-p} for t in D \ D' with t - p in D,
///   (b) the restriction to D' satisfies the system (D', P').
fn reduce_sys(s: &Sys, p: &Pt) -> Reduced {
    let inner: Vec<Pt> = s.pts.iter().filter(|v| s.has(&add(v, p)).is_some()).cloned().collect();
    let mut shifts: Vec<Pt> = vec![normal(p)];
    for q in &s.shifts {
        if normal(q) != normal(p) {
            let q = s.oriented(p, q).unwrap_or_else(|| q.clone());
            let d = normal(&sub(&q, p));
            if d.iter().any(|x| *x != 0) && !shifts.contains(&d) {
                shifts.push(d);
            }
        }
    }
    let small = sys_of(inner, shifts);
    let mut u: Vec<usize> = (0..s.pts.len()).collect();
    let mut free = 0;
    for (i, t) in s.pts.iter().enumerate() {
        if small.has(t).is_none() {
            match s.has(&sub(t, p)) {
                Some(j) => uf_union(&mut u, i, j),
                None => free += 1,
            }
        }
    }
    for q in &small.shifts {
        for v in &small.pts {
            let w = add(v, q);
            if small.has(&w).is_some() {
                uf_union(&mut u, s.has(v).unwrap(), s.has(&w).unwrap());
            }
        }
    }
    Reduced { groups: uf_labels(&mut u), small, free }
}

/// Box version of the domination condition (Theorem B): for every other
/// shift q whose family is not empty, some orientation of q has every
/// coordinate of p between 0 and the same coordinate of q.
fn dominated_box(dims: &[i32], p: &Pt, shifts: &[Pt]) -> bool {
    shifts.iter().filter(|q| normal(q) != normal(p)).all(|q| {
        let empty = q.iter().zip(dims).any(|(c, n)| c.abs() >= *n);
        empty
            || [q.clone(), neg(q)].iter().any(|q| {
                p.iter().zip(q).all(|(pc, qc)| (0 <= *pc && pc <= qc) || (qc <= pc && *pc <= 0))
            })
    })
}

#[derive(Default)]
struct E1 {
    steps: u64,
    dominated: u64,
    dominated_exact: u64,
    other_exact: u64,
    other_inexact: u64,
    not_relaxation: u64,
    formula_mismatch: u64,
    count_mismatch: u64,
    systems: u64,
    systems_without_reducer: u64,
}

fn e1_try(s: &Sys, dims: Option<&[i32]>, t: &mut E1) {
    let truth = s.groups();
    t.systems += 1;
    let mut any = false;
    for p in s.shifts.clone() {
        // a shift whose own family is empty reduces nothing
        if s.pts.iter().all(|v| s.has(&add(v, &p)).is_none()) {
            continue;
        }
        t.steps += 1;
        let red = reduce_sys(s, &p);
        let dom = s.dominated_by_definition(&p);
        if let Some(d) = dims {
            if dom != dominated_box(d, &p, &s.shifts) {
                t.formula_mismatch += 1;
            }
        }
        // relaxation: the reduced description never forces more than the truth
        let fine = (0..truth.len()).all(|i| truth[red.groups[i]] == truth[i]);
        if !fine {
            t.not_relaxation += 1;
        }
        let exact = red.groups == truth;
        if dom {
            any = true;
            t.dominated += 1;
            if exact {
                t.dominated_exact += 1;
            }
            // counting form: groups(D, P) = groups(D', P') + free
            if count_groups(&truth) != count_groups(&red.small.groups()) + red.free {
                t.count_mismatch += 1;
            }
        } else if exact {
            t.other_exact += 1;
        } else {
            t.other_inexact += 1;
        }
    }
    if !any && s.shifts.iter().any(|p| s.pts.iter().any(|v| s.has(&add(v, p)).is_some())) {
        t.systems_without_reducer += 1;
    }
}

fn all_shifts(dims: &[i32]) -> Vec<Pt> {
    let ranges: Vec<i32> = dims.iter().map(|n| 2 * n - 1).collect();
    box_points(&ranges)
        .into_iter()
        .map(|v| v.iter().zip(dims).map(|(c, n)| c - (n - 1)).collect::<Pt>())
        .filter(|v| v.iter().any(|c| *c != 0) && normal(v) == *v)
        .collect()
}

fn subsets_upto(items: &[Pt], k: usize) -> Vec<Vec<Pt>> {
    let mut out: Vec<Vec<Pt>> = vec![vec![]];
    let mut layer: Vec<(usize, Vec<Pt>)> = vec![(0, vec![])];
    for _ in 0..k {
        let mut next = Vec::new();
        for (start, set) in &layer {
            for i in *start..items.len() {
                let mut s = set.clone();
                s.push(items[i].clone());
                out.push(s.clone());
                next.push((i + 1, s));
            }
        }
        layer = next;
    }
    out.retain(|s| !s.is_empty());
    out
}

fn experiment_1(r: &mut Report) {
    println!("E1  general reduction on point sets of Z^d");
    let show = |name: &str, t: &E1, r: &mut Report, expect_reducer: bool| {
        println!(
            "    {name}: {} systems, {} reduction steps; dominated {} (exact {}); not dominated: {} exact, {} inexact; systems with no dominated shift: {}",
            t.systems, t.steps, t.dominated, t.dominated_exact, t.other_exact, t.other_inexact, t.systems_without_reducer
        );
        r.must(t.not_relaxation == 0, &format!("{name}: the reduced description is always a relaxation"));
        r.must(t.dominated == t.dominated_exact && t.count_mismatch == 0, &format!("{name}: domination implies exactness and the counting formula"));
        r.must(t.formula_mismatch == 0, &format!("{name}: domination by definition = coordinate formula (boxes)"));
        if expect_reducer {
            r.must(t.systems_without_reducer == 0, &format!("{name}: every system has a dominated shift"));
        }
    };
    // 1-D intervals: every shift set
    let mut t = E1::default();
    for n in 1..=9 {
        let all: Vec<Pt> = (1..n).map(|q| vec![q]).collect();
        for mask in 1u32..(1 << all.len()) {
            let shifts: Vec<Pt> = (0..all.len()).filter(|i| mask >> i & 1 == 1).map(|i| all[i].clone()).collect();
            e1_try(&sys_of(box_points(&[n]), shifts), Some(&[n]), &mut t);
        }
    }
    show("intervals n<=9, all shift sets", &t, r, true);
    // 2-D boxes: every set of up to 3 shifts
    let mut t = E1::default();
    for dims in [[2, 2], [3, 2], [3, 3], [4, 3], [4, 4], [5, 3]] {
        for shifts in subsets_upto(&all_shifts(&dims), 3) {
            e1_try(&sys_of(box_points(&dims), shifts), Some(&dims), &mut t);
        }
    }
    show("2-D boxes up to 4x4 and 5x3, up to 3 shifts", &t, r, false);
    let two_d_stuck = t.systems_without_reducer;
    // 3-D boxes: every set of up to 2 shifts
    let mut t = E1::default();
    for dims in [[2, 2, 2], [3, 2, 2], [3, 3, 2]] {
        for shifts in subsets_upto(&all_shifts(&dims), 2) {
            e1_try(&sys_of(box_points(&dims), shifts), Some(&dims), &mut t);
        }
    }
    show("3-D boxes up to 3x3x2, up to 2 shifts", &t, r, false);
    // arbitrary point sets (not boxes)
    let mut t = E1::default();
    let mut rng = Lcg(0x5EED_1234_ABCD_0001);
    for _ in 0..4000 {
        let pts: Vec<Pt> = box_points(&[5, 5]).into_iter().filter(|_| rng.below(3) != 0).collect();
        let shifts: Vec<Pt> = (0..1 + rng.below(3)).map(|_| vec![rng.range(-3, 3), rng.range(-3, 3)]).filter(|v| v.iter().any(|c| *c != 0)).map(|v| normal(&v)).collect();
        let mut uniq: Vec<Pt> = Vec::new();
        for s in shifts {
            if !uniq.contains(&s) {
                uniq.push(s);
            }
        }
        if !pts.is_empty() && !uniq.is_empty() {
            e1_try(&sys_of(pts, uniq), None, &mut t);
        }
    }
    show("random subsets of a 5x5 grid", &t, r, false);
    r.must(two_d_stuck > 0, "2-D: systems with no dominated shift exist (the 1-D guarantee does not carry over)");
    // the explicit counterexamples quoted in docs/research/03
    let s = sys_of(box_points(&[2, 2]), vec![vec![1, 0], vec![0, 1]]);
    let red = reduce_sys(&s, &vec![1, 0]);
    r.must(
        count_groups(&s.groups()) == 1 && count_groups(&red.groups) == 2 && !s.dominated_by_definition(&vec![1, 0]) && !s.dominated_by_definition(&vec![0, 1]),
        "2x2 grid, shifts (1,0),(0,1): no dominated shift; reducing by (1,0) gives 2 groups, truth is 1",
    );
    let s = sys_of(box_points(&[3]), vec![vec![1], vec![2]]);
    let red = reduce_sys(&s, &vec![2]);
    r.must(count_groups(&s.groups()) == 1 && count_groups(&red.groups) == 2, "interval n=3, shifts 1,2: reducing by the larger shift gives 2 groups, truth is 1");
    let s = sys_of(box_points(&[8]), vec![vec![2], vec![4]]);
    let red = reduce_sys(&s, &vec![4]);
    r.must(!s.dominated_by_definition(&vec![4]) && red.groups == s.groups(), "interval n=8, shifts 2,4: reducing by 4 is exact although 4 is not dominated (condition is not necessary)");
}

// ----------------------------------------------- E2: mirror constraints

/// Groups of [0, n) under shift families `shifts` and, if `mirror`, the
/// constraint x_i = x_{n-1-i}. Brute force.
fn mirror_groups_brute(n: usize, shifts: &[usize], mirror: bool) -> Vec<usize> {
    let mut u: Vec<usize> = (0..n).collect();
    for &q in shifts {
        for i in 0..n.saturating_sub(q) {
            uf_union(&mut u, i, i + q);
        }
    }
    if mirror {
        for i in 0..n {
            uf_union(&mut u, i, n - 1 - i);
        }
    }
    uf_labels(&mut u)
}

/// Group count by reduction (Theorem M):
///   c(n, P, mirror) = c(k, P', mirror) + ceil(max(0, 2p - n) / 2)   with mirror,
///   c(n, {}, mirror) = ceil(n / 2).
fn mirror_count_reduce(mut n: usize, shifts: &[usize], mirror: bool) -> usize {
    let mut set: Vec<usize> = shifts.to_vec();
    let mut free = 0;
    loop {
        set.retain(|q| *q > 0 && *q < n);
        set.sort_unstable();
        set.dedup();
        let Some(&p) = set.first() else { break };
        let k = n - p;
        let strip = (2 * p).saturating_sub(n);
        free += if mirror { (strip + 1) / 2 } else { strip };
        let mut next: Vec<usize> = set.iter().filter(|q| **q > p).map(|q| q - p).collect();
        if p < k {
            next.push(p);
        }
        set = next;
        n = k;
    }
    free + if mirror { (n + 1) / 2 } else { n }
}

fn experiment_2(r: &mut Report) {
    println!("E2  reduction with a mirror (palindrome) constraint");
    let (mut cases, mut bad_count, mut bad_step) = (0u64, 0u64, 0u64);
    for n in 1..=12usize {
        for mask in 0u32..(1 << (n - 1)) {
            let shifts: Vec<usize> = (1..n).filter(|q| mask >> (q - 1) & 1 == 1).collect();
            cases += 1;
            let truth = mirror_groups_brute(n, &shifts, true);
            if mirror_count_reduce(n, &shifts, true) != count_groups(&truth) {
                bad_count += 1;
            }
            // one step, as a statement about groups:
            // (a) j = j - p for j >= max(p, k); (b) the system on [0, k) with
            // P' and the mirror of [0, k); (c) the strip [k, p) mirrored onto itself
            if let Some(&p) = shifts.first() {
                let k = n - p;
                let mut u: Vec<usize> = (0..n).collect();
                for j in p.max(k)..n {
                    uf_union(&mut u, j, j - p);
                }
                let mut next: Vec<usize> = shifts.iter().filter(|q| **q > p).map(|q| q - p).collect();
                if p < k {
                    next.push(p);
                }
                for &q in &next {
                    for i in 0..k.saturating_sub(q) {
                        uf_union(&mut u, i, i + q);
                    }
                }
                for i in 0..k {
                    uf_union(&mut u, i, k - 1 - i);
                }
                for j in k..p {
                    uf_union(&mut u, j, n - 1 - j);
                }
                if uf_labels(&mut u) != truth {
                    bad_step += 1;
                }
            }
        }
    }
    r.must(bad_step == 0, &format!("one reduction step preserves the groups exactly ({cases} systems, n <= 12)"));
    r.must(bad_count == 0, "repeated reduction gives the right number of groups");
    let mut rng = Lcg(77);
    let mut bad = 0;
    for _ in 0..3000 {
        let n = 1 + rng.below(3000);
        let shifts: Vec<usize> = (0..rng.below(6)).map(|_| 1 + rng.below(n)).collect();
        let mirror = rng.below(2) == 0;
        if mirror_count_reduce(n, &shifts, mirror) != count_groups(&mirror_groups_brute(n, &shifts, mirror)) {
            bad += 1;
        }
    }
    r.must(bad == 0, "3000 random systems up to length 3000, with and without mirror");
}

// ------------------------------------------------ E3: weighted shifts

/// All x in (Z_m)^n with x_{i+q} = x_i + w for every (q, w) in `rules` and
/// every i with i + q < n. A rule with q = 0 reads x_i = x_i + w.
fn offset_solutions(n: usize, m: u8, rules: &[(usize, u8)]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut x = vec![0u8; n];
    loop {
        if rules.iter().all(|&(q, w)| (0..n.saturating_sub(q)).all(|i| x[i + q] == (x[i] + w) % m)) {
            out.push(x.clone());
        }
        let mut i = 0;
        loop {
            if i == n {
                return out;
            }
            x[i] += 1;
            if x[i] < m {
                break;
            }
            x[i] = 0;
            i += 1;
        }
    }
}

fn experiment_3(r: &mut Report) {
    println!("E3  weighted shifts");
    // (i) offsets with equality semantics: the reduction carries weights
    //     w(q - p) = w(q) - w(p); contradictions appear as a zero shift
    //     with a non-zero weight.
    let m = 3u8;
    let (mut cases, mut bad, mut unsat) = (0u64, 0u64, 0u64);
    for n in 1..=6usize {
        let mut rules_all: Vec<(usize, u8)> = Vec::new();
        for q in 1..n {
            for w in 0..m {
                rules_all.push((q, w));
            }
        }
        for a in 0..rules_all.len() {
            for b in a..rules_all.len() {
                for c in b..rules_all.len() {
                    if cases % 3 != 0 && n == 6 {
                        cases += 1;
                        continue; // thin out the largest size
                    }
                    cases += 1;
                    let rules = if a == b && b == c { vec![rules_all[a]] } else if b == c { vec![rules_all[a], rules_all[b]] } else { vec![rules_all[a], rules_all[b], rules_all[c]] };
                    let truth = offset_solutions(n, m, &rules);
                    let (p, wp) = *rules.iter().min().unwrap();
                    let k = n - p;
                    let mut small: Vec<(usize, u8)> = Vec::new();
                    let mut first = true;
                    for &(q, w) in &rules {
                        if q == p && w == wp && first {
                            first = false; // the reducing rule itself
                            continue;
                        }
                        small.push((q - p, (w + m - wp) % m));
                    }
                    if p < k {
                        small.push((p, wp));
                    }
                    let inner = offset_solutions(k, m, &small);
                    // rebuild: free strip [k, p) takes every value; the rest follows by (a)
                    let strip = (2 * p).saturating_sub(n);
                    let mut rebuilt: Vec<Vec<u8>> = Vec::new();
                    for y in &inner {
                        for f in offset_solutions(strip, m, &[]) {
                            let mut x = vec![0u8; n];
                            x[..k].copy_from_slice(y);
                            x[k..k + strip].copy_from_slice(&f);
                            for j in p.max(k)..n {
                                x[j] = (x[j - p] + wp) % m;
                            }
                            rebuilt.push(x);
                        }
                    }
                    rebuilt.sort();
                    let mut t2 = truth.clone();
                    t2.sort();
                    if rebuilt != t2 {
                        bad += 1;
                    }
                    if truth.is_empty() {
                        unsat += 1;
                    }
                }
            }
        }
    }
    r.must(bad == 0, &format!("offsets in Z_3 as equalities: reduced solutions rebuild the solution set exactly ({cases} systems, {unsat} of them unsatisfiable)"));
    // (ii) offsets as inequalities x_{i+q} <= x_i + w: the same recipe is unsound
    let sat = |x: &[i32], rules: &[(usize, i32)], n: usize| rules.iter().all(|&(q, w)| (0..n.saturating_sub(q)).all(|i| x[i + q] <= x[i] + w));
    let original = [(1usize, 5i32), (2, 0)]; // n = 3
    let reduced_inner = [(1usize, -5i32), (1, 5)]; // on [0, 2): shift 2-1 with weight 0-5, and shift 1
    let x = [0, 0, 0];
    r.must(
        sat(&x, &original, 3) && !sat(&x[..2], &reduced_inner, 2),
        "offsets as inequalities (n=3, x1<=x0+5, x2<=x1+5, x2<=x0): (0,0,0) is a solution, the reduced system rejects it",
    );
    // (iii) weights as costs: reduction preserves groups, not distances
    let n = 5;
    let periods = [1usize, 4];
    let mut dist = vec![usize::MAX; n];
    dist[0] = 0;
    let mut queue = vec![0usize];
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        for &q in &periods {
            for v in [u.checked_sub(q), Some(u + q).filter(|v| *v < n)].into_iter().flatten() {
                if dist[v] == usize::MAX {
                    dist[v] = dist[u] + 1;
                    queue.push(v);
                }
            }
        }
    }
    // chain of reduction links from position 4 down to its root
    let root = equal_positions(n, &[Con::Border(n - 1), Con::Border(n - 4)]).unwrap();
    let mut steps = 0;
    let (mut len, mut at) = (n, 4usize);
    let mut set: Vec<usize> = periods.to_vec();
    while at != root[4] {
        set.retain(|q| *q < len);
        set.sort_unstable();
        let p = set[0];
        let k = len - p;
        if at >= p.max(k) {
            at -= p;
            steps += 1;
        }
        let mut next: Vec<usize> = set.iter().filter(|q| **q > p).map(|q| q - p).collect();
        if p < k {
            next.push(p);
        }
        set = next;
        len = k;
    }
    r.must(dist[4] == 1 && steps == 4, "weights as costs (n=5, shifts 1,4): positions 0 and 4 are 1 link apart, the reduction chain between them has 4 links");
}

// -------------------------------------------------- E4: grids, repeated

/// Repeated reduction of a box system using only dominated shifts.
/// Returns (free points accumulated, final dims, final shifts).
fn reduce_box_fully(dims: &[i32], shifts: &[Pt]) -> (i64, Vec<i32>, Vec<Pt>) {
    let mut dims = dims.to_vec();
    let mut set: Vec<Pt> = Vec::new();
    for s in shifts {
        let s = normal(s);
        if s.iter().any(|c| *c != 0) && !set.contains(&s) {
            set.push(s);
        }
    }
    let mut free = 0i64;
    let size = |d: &[i32]| d.iter().map(|x| (*x).max(0) as i64).product::<i64>();
    loop {
        set.retain(|q| q.iter().zip(&dims).all(|(c, n)| c.abs() < *n));
        // smallest dominated shift (by sum of absolute entries)
        let mut cands: Vec<Pt> = set.iter().filter(|p| dominated_box(&dims, p, &set)).cloned().collect();
        cands.sort_by_key(|p| p.iter().map(|c| c.abs()).sum::<i32>());
        let Some(p) = cands.first().cloned() else { break };
        let inner: Vec<i32> = dims.iter().zip(&p).map(|(n, c)| n - c.abs()).collect();
        let inner2: Vec<i32> = dims.iter().zip(&p).map(|(n, c)| n - 2 * c.abs()).collect();
        // points of D with neither v + p nor v - p in D
        free += size(&dims) - 2 * size(&inner) + size(&inner2);
        let mut next: Vec<Pt> = vec![p.clone()];
        for q in &set {
            if *q != p {
                // orientation of q that p is dominated by
                let q = if p.iter().zip(q).all(|(pc, qc)| (0 <= *pc && pc <= qc) || (qc <= pc && *pc <= 0)) { q.clone() } else { neg(q) };
                let d = normal(&sub(&q, &p));
                if d.iter().any(|c| *c != 0) && !next.contains(&d) {
                    next.push(d);
                }
            }
        }
        set = next;
        dims = inner;
    }
    (free, dims, set)
}

fn experiment_4(r: &mut Report) {
    println!("E4  grids: repeated reduction by dominated shifts");
    let mut rng = Lcg(2026);
    let (mut total, mut trivial, mut bad) = (0u64, 0u64, 0u64);
    let mut stuck_by_size: HashMap<usize, u64> = HashMap::new();
    for _ in 0..6000 {
        let dims = [rng.range(3, 12), rng.range(3, 12)];
        let shifts: Vec<Pt> = (0..1 + rng.below(4)).map(|_| vec![rng.range(-4, 4), rng.range(-4, 4)]).filter(|v| v.iter().any(|c| *c != 0)).collect();
        if shifts.is_empty() {
            continue;
        }
        total += 1;
        let (free, d2, s2) = reduce_box_fully(&dims, &shifts);
        let rest = sys_of(box_points(&d2), s2.clone());
        let truth = count_groups(&sys_of(box_points(&dims), shifts.clone()).groups());
        if truth as i64 != free + count_groups(&rest.groups()) as i64 {
            bad += 1;
        }
        if s2.is_empty() {
            trivial += 1;
        } else {
            *stuck_by_size.entry(s2.len()).or_insert(0) += 1;
        }
    }
    r.must(bad == 0, &format!("every chain of dominated reductions preserves the group count ({total} random 2-D systems)"));
    let mut sizes: Vec<(usize, u64)> = stuck_by_size.into_iter().collect();
    sizes.sort();
    println!("    observation: {trivial} of {total} reach the trivial system; the rest stop at a system with no dominated shift (shifts left -> count): {sizes:?}");
    // collinear shifts (all positive multiples of one vector) always reach the trivial system
    let (mut total, mut ok) = (0u64, 0u64);
    for _ in 0..3000 {
        let d = 2 + rng.below(2);
        let dims: Vec<i32> = (0..d).map(|_| rng.range(3, 9)).collect();
        let u: Pt = (0..d).map(|_| rng.range(-2, 2)).collect();
        if u.iter().all(|c| *c == 0) {
            continue;
        }
        let shifts: Vec<Pt> = (0..1 + rng.below(4)).map(|_| { let t = rng.range(1, 5); u.iter().map(|c| c * t).collect() }).collect();
        total += 1;
        let (free, d2, s2) = reduce_box_fully(&dims, &shifts);
        let truth = count_groups(&sys_of(box_points(&dims), shifts).groups());
        if s2.is_empty() && truth as i64 == free + d2.iter().map(|x| *x as i64).product::<i64>() {
            ok += 1;
        }
    }
    r.must(ok == total, &format!("collinear shift sets in 2-D and 3-D boxes always reduce to the trivial system, with the right count ({total} systems)"));
}

fn table() {
    println!("Group counts of two-shift systems with no dominated shift on N x N grids (brute force).");
    println!("{:<22} {:>5} {}", "shifts", "|det|", "groups for N = 6, 8, 12, 16, 24, 32, 48");
    let pairs: [([i32; 2], [i32; 2]); 7] = [([1, 0], [0, 1]), ([2, 0], [0, 3]), ([2, 1], [1, 2]), ([3, 1], [1, 2]), ([2, -1], [1, 3]), ([3, 0], [1, 2]), ([4, 1], [1, 3])];
    for (u, v) in pairs {
        let det = (u[0] * v[1] - u[1] * v[0]).abs();
        let counts: Vec<usize> = [6, 8, 12, 16, 24, 32, 48].iter().map(|n| count_groups(&sys_of(box_points(&[*n, *n]), vec![u.to_vec(), v.to_vec()]).groups())).collect();
        println!("{:<22} {:>5} {:?}", format!("{u:?} {v:?}"), det, counts);
    }
}

// --------------------------------------- E5: partition certificates (1-D)
//
// A certificate that the groups of positions of a border puzzle are
// exactly the classes of `labels`:
//   forest : original links (position, position, rule index)
//   labels : one number per position
// The checker below knows nothing about how they were produced.

struct PartitionCertificate {
    labels: Vec<u32>,
    forest: Vec<(u32, u32, u32)>,
}

/// Produced from the solver: labels from the grouping step, forest from
/// the derivation of every grouping link (cost of that derivation: measured
/// only, see docs/research/04).
fn emit_certificate(n: usize, cons: &[Con]) -> PartitionCertificate {
    let labels: Vec<u32> = equal_positions(n, cons).unwrap().iter().map(|x| *x as u32).collect();
    let mut d = Derivation::new(n, cons);
    let stack = (64usize << 20) + d.phases.len() * 512;
    let forest = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn_scoped(scope, || {
                for j in 0..n {
                    let to = d.link[j] as usize;
                    if to != j {
                        let r = d.phases.partition_point(|ph| ph.start > j) - 1;
                        d.claim(r, to);
                    }
                }
                std::mem::take(&mut d.kept)
            })
            .unwrap()
            .join()
            .unwrap()
    });
    PartitionCertificate { labels, forest }
}

/// Independent check. Accepts only if
///  (1) every forest link is a link of a Border rule of the puzzle,
///  (2) the groups joined by the forest are exactly the classes of `labels`
///      (so positions with equal labels are forced equal), and
///  (3) the word `labels` itself has every border the rules ask for (so it
///      is a solution in which different classes hold different letters,
///      hence no two classes are forced equal).
/// (3) uses the classical border table (failure function) of the word, not
/// the solver. Cost: O((n + m) * inverse-Ackermann).
fn check_certificate(n: usize, cons: &[Con], cert: &PartitionCertificate) -> Result<(), String> {
    if cert.labels.len() != n {
        return Err("labels: wrong length".into());
    }
    let mut u: Vec<usize> = (0..n).collect();
    for &(a, b, idx) in &cert.forest {
        let (lo, hi) = ((a.min(b)) as usize, (a.max(b)) as usize);
        match cons.get(idx as usize) {
            Some(Con::Border(k)) if *k <= n && hi < n && lo < *k && hi == lo + (n - *k) => {}
            _ => return Err(format!("forest: ({a}, {b}) is not a link of rule {idx}")),
        }
        uf_union(&mut u, lo, hi);
    }
    let mut label_of_group = vec![u32::MAX; n];
    let mut group_of_label: HashMap<u32, usize> = HashMap::new();
    for i in 0..n {
        let g = uf_find(&mut u, i);
        if label_of_group[g] == u32::MAX {
            label_of_group[g] = cert.labels[i];
            if group_of_label.insert(cert.labels[i], g).is_some() {
                return Err(format!("position {i}: label {} covers positions the forest does not join", cert.labels[i]));
            }
        } else if label_of_group[g] != cert.labels[i] {
            return Err(format!("position {i}: the forest joins positions with different labels"));
        }
    }
    // border table: fail[i] = length of the longest proper border of labels[..=i]
    let w = &cert.labels;
    let mut fail = vec![0usize; n];
    for i in 1..n {
        let mut k = fail[i - 1];
        while k > 0 && w[i] != w[k] {
            k = fail[k - 1];
        }
        if w[i] == w[k] {
            k += 1;
        }
        fail[i] = k;
    }
    let mut is_border = vec![false; n + 1];
    let mut k = if n > 0 { fail[n - 1] } else { 0 };
    while k > 0 {
        is_border[k] = true;
        k = fail[k - 1];
    }
    for (idx, c) in cons.iter().enumerate() {
        if let Con::Border(k) = c {
            if *k > n || (*k > 0 && *k < n && !is_border[*k]) {
                return Err(format!("rule {idx}: the label word does not have border {k}"));
            }
        }
    }
    Ok(())
}

/// With the groups certified, the solver's whole answer for a solvable
/// puzzle can be checked directly: letters per group from the words, and
/// the pattern cell by cell.
fn check_pattern(n: usize, cons: &[Con], labels: &[u32], pattern: &[Cell], free: usize) -> Result<(), String> {
    let mut letter: HashMap<u32, u8> = HashMap::new();
    for c in cons {
        let (w, off) = match c {
            Con::Prefix(w) => (w, 0),
            Con::Suffix(w) => (w, n - w.len()),
            Con::Border(_) => continue,
        };
        for (j, ch) in w.iter().enumerate() {
            if *letter.entry(labels[off + j]).or_insert(*ch) != *ch {
                return Err("two words give one group different letters: the puzzle is not solvable".into());
            }
        }
    }
    let mut id_of: HashMap<u32, usize> = HashMap::new();
    for i in 0..n {
        match (letter.get(&labels[i]), pattern[i]) {
            (Some(l), Cell::Letter(ch)) if *l == ch => {}
            (None, Cell::Free(id)) => {
                let next = id_of.len();
                if *id_of.entry(labels[i]).or_insert(next) != id || id >= free {
                    return Err(format!("position {i}: free-group numbering does not match the groups"));
                }
            }
            _ => return Err(format!("position {i}: pattern cell does not match the group's letter")),
        }
    }
    if id_of.len() != free {
        return Err("number of free groups is wrong".into());
    }
    Ok(())
}

fn experiment_5(r: &mut Report) {
    println!("E5  partition certificates for the 1-D solver");
    let mut rng = Lcg(31337);
    let (mut made, mut accepted, mut brute_bad, mut pattern_ok, mut patterns) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut tampered, mut caught) = (0u64, 0u64);
    for trial in 0..20_000 {
        let n = if trial < 15_000 { 1 + rng.below(40) } else { 40 + rng.below(1500) };
        let g = 1 + rng.below(1 + n / 4);
        let mut cons: Vec<Con> = (0..rng.below(10)).map(|_| Con::Border(match rng.below(3) { 0 => rng.below(n + 1), 1 => n - (g * (1 + rng.below(6))).min(n), _ => n / 2 + rng.below(2) })).collect();
        made += 1;
        let cert = emit_certificate(n, &cons);
        let verdict = check_certificate(n, &cons, &cert);
        if verdict.is_ok() {
            accepted += 1;
        } else {
            println!("    rejected: n={n} cons={cons:?}: {verdict:?}");
        }
        // tiny cases: number of 2-letter texts with all borders = 2^(number of labels)
        if n <= 12 && trial % 7 == 0 {
            let mut sols = 0u64;
            for bits in 0u32..(1 << n) {
                let s: Vec<u8> = (0..n).map(|i| (bits >> i & 1) as u8).collect();
                if cons.iter().all(|c| matches!(c, Con::Border(k) if s[..*k] == s[n - *k..])) {
                    sols += 1;
                }
            }
            let mut distinct = cert.labels.clone();
            distinct.sort_unstable();
            distinct.dedup();
            if sols != 1u64 << distinct.len() {
                brute_bad += 1;
            }
        }
        // tampering must be caught
        if n >= 3 {
            let mut distinct = cert.labels.clone();
            distinct.sort_unstable();
            distinct.dedup();
            // merge two classes
            if distinct.len() >= 2 {
                let (a, b) = (distinct[0], distinct[1]);
                let bad = PartitionCertificate { labels: cert.labels.iter().map(|l| if *l == b { a } else { *l }).collect(), forest: cert.forest.clone() };
                tampered += 1;
                caught += check_certificate(n, &cons, &bad).is_err() as u64;
            }
            // split a class: give one linked position a fresh label
            if let Some(&(a, _, _)) = cert.forest.first() {
                let mut labels = cert.labels.clone();
                labels[a as usize] = u32::MAX - 1;
                tampered += 1;
                caught += check_certificate(n, &cons, &PartitionCertificate { labels, forest: cert.forest.clone() }).is_err() as u64;
                // drop a forest link
                tampered += 1;
                caught += check_certificate(n, &cons, &PartitionCertificate { labels: cert.labels.clone(), forest: cert.forest[1..].to_vec() }).is_err() as u64;
                // forge a link between two positions of different groups
                if distinct.len() >= 2 {
                    let i = cert.labels.iter().position(|l| *l == distinct[0]).unwrap() as u32;
                    let j = cert.labels.iter().position(|l| *l == distinct[1]).unwrap() as u32;
                    let mut forest = cert.forest.clone();
                    forest.push((i, j, cert.forest[0].2));
                    tampered += 1;
                    caught += check_certificate(n, &cons, &PartitionCertificate { labels: cert.labels.clone(), forest }).is_err() as u64;
                }
            }
        }
        // whole answers of solvable puzzles, checked through the certificate
        if trial % 3 == 0 && n >= 1 {
            let ab = b"ab";
            let base = ab[rng.below(2)];
            let len = rng.below(n.min(5) + 1);
            cons.push(Con::Prefix(vec![base; len]));
            if let Ok(Outcome::Sat { pattern, free }) = solve(n, ab, &cons) {
                patterns += 1;
                pattern_ok += check_pattern(n, &cons, &cert.labels, &pattern, free).is_ok() as u64;
            }
        }
    }
    r.must(accepted == made, &format!("{made} certificates emitted from the solver, all accepted by the independent checker"));
    r.must(brute_bad == 0, "tiny cases: 2^(number of labels) equals the number of 2-letter texts with all borders, by enumeration");
    r.must(caught == tampered, &format!("{tampered} tampered certificates (merged classes, split class, missing link, forged link), all rejected"));
    r.must(pattern_ok == patterns, &format!("{patterns} solver answers for solvable puzzles confirmed cell by cell from the certified groups"));
}

fn big(n: usize) {
    let mut rng = Lcg(9);
    let h = n / 2;
    let hh = if h % 3 == 0 { h - 1 } else { h };
    let shapes: Vec<(&str, Vec<Con>)> = vec![
        ("no borders", vec![]),
        ("one border (half)", vec![Con::Border(h)]),
        ("16 random borders", (0..16).map(|_| Con::Border(1 + rng.below(n - 1))).collect()),
        ("1000 random borders", (0..1000).map(|_| Con::Border(1 + rng.below(n - 1))).collect()),
        ("every border", (1..n).map(Con::Border).collect()),
        ("even periods >= n/2", (h..n).filter(|q| q % 2 == 0).map(|q| Con::Border(n - q)).collect()),
        ("block twice, block period 3", (0..hh / 3).map(|j| Con::Border(hh - 3 * j)).chain([Con::Border(n - hh)]).collect()),
        ("two coprime periods at the Fine-Wilf bound", vec![Con::Border(n - h), Con::Border(n - h - 1)]),
    ];
    println!("partition certificates, n = {n}");
    let mut fails = 0;
    for (name, cons) in shapes {
        let t = Instant::now();
        let cert = emit_certificate(n, &cons);
        let t_emit = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let verdict = check_certificate(n, &cons, &cert);
        let t_check = t.elapsed().as_secs_f64();
        let mut groups = cert.labels.clone();
        groups.sort_unstable();
        groups.dedup();
        println!(
            "  {:<44} borders={:<8} groups={:<8} forest links={:<8} emit={:.3}s check={:.3}s {}",
            name,
            cons.len(),
            groups.len(),
            cert.forest.len(),
            t_emit,
            t_check,
            if verdict.is_ok() { "accepted".to_string() } else { fails += 1; format!("REJECTED {verdict:?}") }
        );
    }
    std::process::exit(if fails == 0 { 0 } else { 1 });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("big") => big(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1_000_000)),
        Some("table") => table(),
        None | Some("check") => {
            let mut r = Report { fails: 0 };
            experiment_1(&mut r);
            experiment_2(&mut r);
            experiment_3(&mut r);
            experiment_4(&mut r);
            experiment_5(&mut r);
            println!("theory checks failed: {}", r.fails);
            std::process::exit(if r.fails == 0 { 0 } else { 1 });
        }
        _ => {
            eprintln!("usage: [check] | big N | table");
            std::process::exit(2);
        }
    }
}
