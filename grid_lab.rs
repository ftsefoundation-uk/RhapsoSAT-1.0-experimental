#!/bin/sh
//bin/true 2>/dev/null; out="${TMPDIR:-/tmp}/prefsuf_grid_lab"; rustc --edition 2021 -O "$0" -o "$out" && exec "$out" "$@"; exit $?
// Grid laboratory: shift systems on boxes of Z^d (docs/research/05).
// Stand-alone: shares no code with prefsuf.rs.
//
//     sh grid_lab.rs            every check below (about half a minute)
//     sh grid_lab.rs bench      counting at scale: brute force vs formulas
//     sh grid_lab.rs count N1,N2[,..] A,B[,..] [C,D[,..] ...]
//                               group count of one system, three ways
//
// A box system is a box D = [0,n1) x ... x [0,nd) and a set P of non-zero
// integer vectors; the constraints are x_v = x_{v+q} whenever v and v+q are
// both in D. "Groups" are the classes of positions forced equal.
//
// What is checked here, and its status in docs/research/05:
//
//   G1  PROVED. Two linearly independent shifts u, v, any dimension, any
//       box: groups = V - E_u - E_v + S (points, links, squares).
//   G2  PROVED. In 2-D with n_i >= |u_i| + |v_i| this equals
//       |u1 v2| + |u2 v1| = |det| + kappa.
//   G3  PROVED. At most one shift is dominated (up to sign).
//   G4  PROVED (theorem 3 of note 02 + G1). Reduce-then-count agrees with
//       brute force.
//   G5  PROOF SKETCH + TESTED. Three or more shifts of rank 2 in 2-D, large
//       boxes: groups = (index of the shift lattice) + corner constant.
//   G6  COUNTEREXAMPLES. In 3-D with three independent shifts the count is
//       not V - E + S - C and can grow with the box.
//   G7  TESTED ONLY. Exact reduction chains through non-dominated steps.
//
// Two independent oracles: union-find over every link (`count_brute`) and
// breadth-first search over explicit neighbours (`count_bfs`).
// A failed check prints FAIL and the exit status is 1.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

type V = Vec<i64>;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
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

// ------------------------------------------------------------- vectors

fn neg(a: &V) -> V {
    a.iter().map(|x| -x).collect()
}
fn sub(a: &V, b: &V) -> V {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}
/// q and -q are the same constraint family: first non-zero entry positive.
fn normal(a: &V) -> V {
    match a.iter().find(|x| **x != 0) {
        Some(x) if *x < 0 => neg(a),
        _ => a.clone(),
    }
}
fn is_zero(a: &V) -> bool {
    a.iter().all(|x| *x == 0)
}
/// The family of q is non-empty in the box.
fn active(dims: &[i64], q: &V) -> bool {
    q.iter().zip(dims).all(|(c, n)| c.abs() < *n)
}
/// Conformal order: every coordinate of p lies between 0 and that of q.
fn below(p: &V, q: &V) -> bool {
    p.iter().zip(q).all(|(a, b)| (0 <= *a && a <= b) || (b <= a && *a <= 0))
}
fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}
fn independent(u: &V, v: &V) -> bool {
    for i in 0..u.len() {
        for j in i + 1..u.len() {
            if u[i] * v[j] - u[j] * v[i] != 0 {
                return true;
            }
        }
    }
    false
}
fn volume(dims: &[i64]) -> i128 {
    dims.iter().map(|x| (*x).max(0) as i128).product()
}
/// Distinct active shifts, normalised.
fn clean(dims: &[i64], shifts: &[V]) -> Vec<V> {
    let mut out: Vec<V> = Vec::new();
    for q in shifts {
        let q = normal(q);
        if !is_zero(&q) && active(dims, &q) && !out.contains(&q) {
            out.push(q);
        }
    }
    out
}

// ------------------------------------------------------------- oracles

fn find(u: &mut [u32], mut x: u32) -> u32 {
    while u[x as usize] != x {
        u[x as usize] = u[u[x as usize] as usize];
        x = u[x as usize];
    }
    x
}

/// Oracle 1: union-find over every link. Returns (groups, links looked at).
fn count_brute(dims: &[i64], shifts: &[V]) -> (i128, u64) {
    let d = dims.len();
    if dims.iter().any(|x| *x <= 0) {
        return (0, 0);
    }
    let total: usize = dims.iter().map(|x| *x as usize).product();
    let mut u: Vec<u32> = (0..total as u32).collect();
    let mut stride = vec![1i64; d];
    for i in (0..d.saturating_sub(1)).rev() {
        stride[i] = stride[i + 1] * dims[i + 1];
    }
    let mut groups = total as i128;
    let mut links = 0u64;
    let mut pt = vec![0i64; d];
    for idx in 0..total {
        let mut r = idx as i64;
        for i in 0..d {
            pt[i] = r / stride[i];
            r %= stride[i];
        }
        for q in shifts {
            let mut ok = true;
            let mut off = 0i64;
            for i in 0..d {
                let c = pt[i] + q[i];
                if c < 0 || c >= dims[i] {
                    ok = false;
                    break;
                }
                off += q[i] * stride[i];
            }
            if ok {
                links += 1;
                let a = find(&mut u, idx as u32);
                let b = find(&mut u, (idx as i64 + off) as u32);
                if a != b {
                    u[a as usize] = b;
                    groups -= 1;
                }
            }
        }
    }
    (groups, links)
}

/// Oracle 2: breadth-first search over explicit points (no union-find).
fn count_bfs(dims: &[i64], shifts: &[V]) -> i128 {
    let mut pts: Vec<V> = vec![vec![]];
    for &n in dims {
        pts = pts.into_iter().flat_map(|p| (0..n).map(move |c| { let mut q = p.clone(); q.push(c); q })).collect();
    }
    let mut seen: HashSet<V> = HashSet::new();
    let mut groups = 0;
    for p in &pts {
        if seen.contains(p) {
            continue;
        }
        groups += 1;
        seen.insert(p.clone());
        let mut queue = VecDeque::from([p.clone()]);
        while let Some(v) = queue.pop_front() {
            for q in shifts {
                for s in [1i64, -1] {
                    let w: V = v.iter().zip(q).map(|(a, b)| a + s * b).collect();
                    if w.iter().zip(dims).all(|(c, n)| 0 <= *c && c < n) && !seen.contains(&w) {
                        seen.insert(w.clone());
                        queue.push_back(w);
                    }
                }
            }
        }
    }
    groups
}

// ------------------------------------------------- the proved formulas

/// Number of v with v, v + q1, ..., and all partial sums in the box, for
/// the shifts in `qs`: the product of (n_i - sum |q_i|), or 0.
fn cells(dims: &[i64], qs: &[&V]) -> i128 {
    let mut out: i128 = 1;
    for (i, n) in dims.iter().enumerate() {
        let side = n - qs.iter().map(|q| q[i].abs()).sum::<i64>();
        if side <= 0 {
            return 0;
        }
        out *= side as i128;
    }
    out
}

/// G1 (PROVED): groups of a box under two linearly independent shifts.
/// Points - links + squares; valid for every box and every dimension.
fn count_two(dims: &[i64], u: &V, v: &V) -> i128 {
    volume(dims) - cells(dims, &[u]) - cells(dims, &[v]) + cells(dims, &[u, v])
}

/// The Euler characteristic V - E + S - C ... of the cubical complex for any
/// number of shifts. Equals the group count for one shift and for two
/// independent shifts; NOT in general for three (see G6).
fn euler(dims: &[i64], shifts: &[V]) -> i128 {
    let m = shifts.len();
    let mut out = 0i128;
    for mask in 0u32..(1 << m) {
        let qs: Vec<&V> = (0..m).filter(|i| mask >> i & 1 == 1).map(|i| &shifts[i]).collect();
        let c = cells(dims, &qs);
        out += if qs.len() % 2 == 0 { c } else { -c };
    }
    out
}

/// The dominated shift of an active, normalised shift set, if any. PROVED
/// unique up to sign (G3), so the answer does not depend on search order.
fn dominated(set: &[V]) -> Option<V> {
    set.iter().find(|p| set.iter().all(|q| q == *p || below(p, q) || below(p, &neg(q)))).cloned()
}

struct Chain {
    free: i128,     // points released by the reductions (sum of |F|)
    dims: Vec<i64>, // box of the core
    core: Vec<V>,   // active shifts of the core (none: trivial system)
    steps: u64,     // single reduction steps
    jumps: u64,     // runs of equal steps (the work actually done)
}

/// Positive volume: 0 as soon as one side is not positive.
fn volume_pos(dims: &[i64]) -> i128 {
    if dims.iter().all(|x| *x > 0) { volume(dims) } else { 0 }
}
fn shrink(dims: &[i64], p: &V, times: i64) -> Vec<i64> {
    dims.iter().zip(p).map(|(n, c)| n - times * c.abs()).collect()
}

/// Repeated reduction by the dominated shift (theorem 3 of note 02), one
/// step at a time. Every step is exact, so
/// groups(D, P) = free + groups(core). No point is ever touched.
/// Kept as the reference for `reduce_chain`.
fn reduce_chain_slow(dims: &[i64], shifts: &[V]) -> Chain {
    let mut dims = dims.to_vec();
    let mut set = clean(&dims, shifts);
    let (mut free, mut steps) = (0i128, 0u64);
    loop {
        set.retain(|q| active(&dims, q));
        let Some(p) = dominated(&set) else { break };
        // points with neither v + p nor v - p in the box
        free += volume(&dims) - 2 * volume(&shrink(&dims, &p, 1)) + volume_pos(&shrink(&dims, &p, 2));
        let mut next = vec![p.clone()];
        for q in &set {
            if *q != p {
                let q = if below(&p, q) { q.clone() } else { neg(q) };
                let d = normal(&sub(&q, &p));
                if !is_zero(&d) && !next.contains(&d) {
                    next.push(d);
                }
            }
        }
        set = next;
        dims = shrink(&dims, &p, 1);
        steps += 1;
    }
    Chain { free, dims, core: set, steps, jumps: steps }
}

/// The same chain with runs of equal steps done at once (PROVED, note 06):
/// if p is dominated and stays so for t steps, the t steps release
/// a_0 - a_1 - a_t + a_{t+1} points, where a_s is the volume of the box
/// shrunk s times by |p| (0 once a side is not positive). t is the largest
/// number with p below q - (t-1)p for every other shift q and p still
/// active. This is the division step of Euclid's algorithm.
fn reduce_chain(dims: &[i64], shifts: &[V]) -> Chain {
    let mut dims = dims.to_vec();
    let mut set = clean(&dims, shifts);
    let (mut free, mut steps, mut jumps) = (0i128, 0u64, 0u64);
    loop {
        set.retain(|q| active(&dims, q));
        let Some(p) = dominated(&set) else { break };
        // p stays active while n_i - s|p_i| > |p_i|: at most this many steps
        let mut t = dims.iter().zip(&p).filter(|(_, c)| **c != 0).map(|(n, c)| (n - 1) / c.abs()).min().unwrap();
        let mut oriented: Vec<V> = Vec::new();
        for q in &set {
            if *q != p {
                let q = if below(&p, q) { q.clone() } else { neg(q) };
                // p stays below q - s p for s < |q_i| / |p_i| in every coordinate where p_i != 0
                t = t.min(q.iter().zip(&p).filter(|(_, c)| **c != 0).map(|(a, c)| a.abs() / c.abs()).min().unwrap());
                oriented.push(q);
            }
        }
        free += volume(&dims) - volume(&shrink(&dims, &p, 1)) - volume_pos(&shrink(&dims, &p, t)) + volume_pos(&shrink(&dims, &p, t + 1));
        let mut next = vec![p.clone()];
        for q in &oriented {
            let d = normal(&q.iter().zip(&p).map(|(a, c)| a - t * c).collect());
            if !is_zero(&d) && !next.contains(&d) {
                next.push(d);
            }
        }
        set = next;
        dims = shrink(&dims, &p, t);
        steps += t as u64;
        jumps += 1;
    }
    Chain { free, dims, core: set, steps, jumps }
}

/// How `count_hybrid` finished.
#[derive(Clone, Copy, PartialEq, Debug)]
enum How {
    Trivial, // the chain reached a system without links
    OneShift,
    TwoShifts, // closed formula G1 on the core
    BruteCore, // union-find on the core only
}

/// Reduce first, then count the core: by formula when it has at most two
/// independent shifts, otherwise by union-find on the (smaller) core box.
/// Exact in every case. Returns (groups, how, links looked at).
fn count_hybrid(dims: &[i64], shifts: &[V]) -> (i128, How, u64) {
    let c = reduce_chain(dims, shifts);
    match c.core.len() {
        0 => (c.free + volume(&c.dims), How::Trivial, 0),
        1 => (c.free + volume(&c.dims) - cells(&c.dims, &[&c.core[0]]), How::OneShift, 0),
        2 if independent(&c.core[0], &c.core[1]) => (c.free + count_two(&c.dims, &c.core[0], &c.core[1]), How::TwoShifts, 0),
        _ => {
            let (g, links) = count_brute(&c.dims, &c.core);
            (c.free + g, How::BruteCore, links)
        }
    }
}

/// Index of the lattice generated by 2-D shifts in Z^2 (0 if rank < 2).
fn index2(s: &[V]) -> i64 {
    let mut g = 0;
    for i in 0..s.len() {
        for j in i + 1..s.len() {
            g = gcd(g, (s[i][0] * s[j][1] - s[i][1] * s[j][0]).abs());
        }
    }
    g
}

/// Corner constant measured directly: finite components of the shift graph
/// on the two kinds of quadrant, seen in an m x m corner window.
fn kappa_measured(shifts: &[V], m: i64) -> i128 {
    let mut total = 0;
    for sy in [1i64, -1] {
        let fl: Vec<V> = shifts.iter().map(|q| vec![q[0], q[1] * sy]).collect();
        let n = (m * m) as usize;
        let mut u: Vec<u32> = (0..n as u32).collect();
        for x in 0..m {
            for y in 0..m {
                for q in &fl {
                    let (a, b) = (x + q[0], y + q[1]);
                    if 0 <= a && a < m && 0 <= b && b < m {
                        let (r, s) = (find(&mut u, (x * m + y) as u32), find(&mut u, (a * m + b) as u32));
                        if r != s {
                            u[r as usize] = s;
                        }
                    }
                }
            }
        }
        let (mut all, mut escaping) = (HashSet::new(), HashSet::new());
        for x in 0..m {
            for y in 0..m {
                let r = find(&mut u, (x * m + y) as u32);
                all.insert(r);
                if x >= m / 2 || y >= m / 2 {
                    escaping.insert(r);
                }
            }
        }
        total += 2 * (all.len() - escaping.len()) as i128;
    }
    total
}

fn normals(d: usize, r: i64) -> Vec<V> {
    let mut out: Vec<V> = vec![vec![]];
    for _ in 0..d {
        out = out.into_iter().flat_map(|p| (-r..=r).map(move |c| { let mut q = p.clone(); q.push(c); q })).collect();
    }
    out.into_iter().filter(|v| !is_zero(v) && normal(v) == *v).collect()
}

// ------------------------------------------------------------- checks

fn g1(r: &mut Report) {
    println!("G1  two independent shifts: groups = points - links + squares (PROVED)");
    // 2-D: every pair with entries in [-4,4], every box up to 14 x 14 (thin ones included)
    let ns = normals(2, 4);
    let (mut pairs, mut boxes, mut bad) = (0u64, 0u64, 0u64);
    for i in 0..ns.len() {
        for j in i + 1..ns.len() {
            if !independent(&ns[i], &ns[j]) {
                continue;
            }
            pairs += 1;
            for a in 1..=14 {
                for b in 1..=14 {
                    boxes += 1;
                    if count_two(&[a, b], &ns[i], &ns[j]) != count_brute(&[a, b], &[ns[i].clone(), ns[j].clone()]).0 {
                        bad += 1;
                    }
                }
            }
        }
    }
    r.must(bad == 0, &format!("2-D: all {pairs} independent pairs with entries in [-4,4], every box from 1x1 to 14x14 ({boxes} systems)"));
    // 3-D and 4-D: random pairs and boxes
    let mut rng = Lcg(31);
    for (d, side, range, rounds) in [(3usize, 9i64, 3i64, 6000), (4, 6, 2, 3000)] {
        let (mut n, mut bad) = (0u64, 0u64);
        for _ in 0..rounds {
            let dims: Vec<i64> = (0..d).map(|_| rng.range(1, side)).collect();
            let u: V = (0..d).map(|_| rng.range(-range, range)).collect();
            let v: V = (0..d).map(|_| rng.range(-range, range)).collect();
            if !independent(&u, &v) {
                continue;
            }
            n += 1;
            if count_two(&dims, &u, &v) != count_brute(&dims, &[u, v]).0 {
                bad += 1;
            }
        }
        r.must(bad == 0, &format!("{d}-D: {n} random independent pairs (entries in [-{range},{range}]) on random boxes with sides up to {side}"));
    }
    // the second oracle agrees with the first
    let (mut n, mut bad) = (0u64, 0u64);
    for _ in 0..3000 {
        let d = rng.range(1, 3) as usize;
        let dims: Vec<i64> = (0..d).map(|_| rng.range(1, if d == 3 { 6 } else { 12 })).collect();
        let sh: Vec<V> = (0..rng.range(0, 4)).map(|_| (0..d).map(|_| rng.range(-3, 3)).collect::<V>()).filter(|v| !is_zero(v)).collect();
        n += 1;
        if count_bfs(&dims, &sh) != count_brute(&dims, &sh).0 {
            bad += 1;
        }
    }
    r.must(bad == 0, &format!("the two oracles (union-find, breadth-first search) agree on {n} random systems in 1 to 3 dimensions"));
}

fn g2(r: &mut Report) {
    println!("G2  2-D closed form above the threshold n_i >= |u_i| + |v_i| (PROVED)");
    let ns = normals(2, 5);
    let (mut pairs, mut bad, mut odd, mut kbad) = (0u64, 0u64, 0u64, 0u64);
    let mut hist: HashMap<i64, u64> = HashMap::new();
    for i in 0..ns.len() {
        for j in i + 1..ns.len() {
            let (u, v) = (&ns[i], &ns[j]);
            let det = (u[0] * v[1] - u[1] * v[0]).abs();
            if det == 0 {
                continue;
            }
            pairs += 1;
            let closed = (u[0] * v[1]).abs() + (u[1] * v[0]).abs();
            let kappa = closed - det;
            if kappa % 2 != 0 {
                odd += 1;
            }
            *hist.entry(kappa).or_insert(0) += 1;
            let (s1, s2) = (u[0].abs() + v[0].abs(), u[1].abs() + v[1].abs());
            for (a, b) in [(s1, s2), (s1 + 1, s2), (s1, s2 + 3), (s1 + 7, s2 + 2), (3 * s1 + 5, 2 * s2 + 9)] {
                if count_brute(&[a, b], &[u.clone(), v.clone()]).0 != closed as i128 {
                    bad += 1;
                }
            }
            // the same constant, measured as components trapped at the corners
            if i % 3 == 0 && kappa_measured(&[u.clone(), v.clone()], 44) != kappa as i128 {
                kbad += 1;
            }
        }
    }
    r.must(bad == 0, &format!("groups = |u1 v2| + |u2 v1| on 5 boxes at and above the threshold for each of {pairs} pairs (entries in [-5,5])"));
    r.must(odd == 0, "kappa = |u1 v2| + |u2 v1| - |det| is even for every pair");
    r.must(kbad == 0, "kappa equals the number of components trapped at the four corners (measured in corner windows, every third pair)");
    let mut h: Vec<_> = hist.into_iter().collect();
    h.sort();
    println!("    observation: kappa -> number of pairs: {h:?}");
    // below the threshold the closed form can fail, the G1 formula does not
    let (u, v) = (vec![2, 1], vec![1, 2]);
    let thin: Vec<i128> = (3..=8).map(|n| count_brute(&[2, n], &[u.clone(), v.clone()]).0).collect();
    r.must(thin == vec![5, 6, 7, 8, 9, 10], "counterexample to the old conjecture on thin boxes: (2,1),(1,2) on 2 x N has N + 2 groups, not 5");
}

fn subsets(items: &[V], k: usize) -> Vec<Vec<V>> {
    let mut out: Vec<Vec<V>> = vec![vec![]];
    for it in items {
        let mut more = vec![];
        for s in &out {
            if s.len() < k {
                let mut t = s.clone();
                t.push(it.clone());
                more.push(t);
            }
        }
        out.extend(more);
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}
fn all_active_normals(dims: &[i64]) -> Vec<V> {
    let mut out: Vec<V> = vec![vec![]];
    for &n in dims {
        out = out.into_iter().flat_map(|p| (-(n - 1)..=(n - 1)).map(move |c| { let mut q = p.clone(); q.push(c); q })).collect();
    }
    out.into_iter().filter(|v| !is_zero(v) && normal(v) == *v).collect()
}

fn g3_g4(r: &mut Report) {
    println!("G3  the dominated shift is unique up to sign (PROVED)");
    let (mut systems, mut multi, mut bad) = (0u64, 0u64, 0u64);
    let mut how: HashMap<String, u64> = HashMap::new();
    for (dims, k) in [(vec![3i64, 3], 3usize), (vec![4, 3], 3), (vec![4, 4], 3), (vec![5, 5], 2), (vec![2, 2, 2], 3), (vec![3, 3, 2], 2), (vec![9], 4)] {
        for s in subsets(&all_active_normals(&dims), k) {
            systems += 1;
            // every stage of the chain
            let mut d = dims.clone();
            let mut set = clean(&d, &s);
            loop {
                set.retain(|q| active(&d, q));
                let doms = set.iter().filter(|p| set.iter().all(|q| q == *p || below(p, q) || below(p, &neg(q)))).count();
                if doms > 1 {
                    multi += 1;
                }
                let c = reduce_chain(&d, &set);
                if c.steps == 0 {
                    break;
                }
                // one step only: redo by hand
                let p = dominated(&set).unwrap();
                let mut next = vec![p.clone()];
                for q in &set {
                    if *q != p {
                        let q = if below(&p, q) { q.clone() } else { neg(q) };
                        let dd = normal(&sub(&q, &p));
                        if !is_zero(&dd) && !next.contains(&dd) {
                            next.push(dd);
                        }
                    }
                }
                d = d.iter().zip(&p).map(|(n, c)| n - c.abs()).collect();
                set = next;
            }
            let (g, h, _) = count_hybrid(&dims, &s);
            *how.entry(format!("{h:?}")).or_insert(0) += 1;
            if g != count_brute(&dims, &s).0 {
                bad += 1;
            }
        }
    }
    r.must(multi == 0, &format!("no stage of any chain has two dominated shifts ({systems} systems, exhaustive on boxes up to 5x5, 3x3x2 and intervals of 9)"));
    println!("G4  reduce first, then count the core (PROVED: theorem 3 + G1)");
    r.must(bad == 0, &format!("reduce-then-count equals brute force on the same {systems} systems"));
    let mut h: Vec<_> = how.into_iter().collect();
    h.sort();
    println!("    observation: how the count was finished: {h:?}");
    let mut rng = Lcg(77);
    let (mut n, mut bad, mut jump_bad) = (0u64, 0u64, 0u64);
    let mut how: HashMap<String, u64> = HashMap::new();
    for _ in 0..6000 {
        let d = rng.range(1, 4) as usize;
        let side = [0, 60, 14, 8, 5][d];
        let dims: Vec<i64> = (0..d).map(|_| rng.range(1, side)).collect();
        let sh: Vec<V> = (0..rng.range(1, 4)).map(|_| (0..d).map(|_| rng.range(-4, 4)).collect::<V>()).filter(|v| !is_zero(v)).collect();
        n += 1;
        let (g, h, _) = count_hybrid(&dims, &sh);
        *how.entry(format!("{h:?}")).or_insert(0) += 1;
        if g != count_brute(&dims, &sh).0 {
            bad += 1;
        }
        let (fast, slow) = (reduce_chain(&dims, &sh), reduce_chain_slow(&dims, &sh));
        let mut cores = (fast.core.clone(), slow.core.clone());
        cores.0.sort();
        cores.1.sort();
        if fast.free != slow.free || fast.dims != slow.dims || cores.0 != cores.1 || fast.steps != slow.steps {
            jump_bad += 1;
        }
    }
    r.must(bad == 0, &format!("reduce-then-count equals brute force on {n} random systems in 1 to 4 dimensions"));
    r.must(jump_bad == 0, "the chain with runs done at once equals the step-by-step chain (released points, core, number of steps) on the same systems");
    let mut h: Vec<_> = how.into_iter().collect();
    h.sort();
    println!("    observation: how the count was finished: {h:?}");
}

fn g5(r: &mut Report) {
    println!("G5  2-D, three or four shifts of rank 2, large boxes: index + corner constant (PROOF SKETCH, TESTED)");
    let mut rng = Lcg(7);
    let (mut total, mut bad, mut unstable) = (0u64, 0u64, 0u64);
    for _ in 0..1200 {
        let m = rng.range(3, 4) as usize;
        let mut sh: Vec<V> = vec![];
        while sh.len() < m {
            let v = normal(&vec![rng.range(-4, 4), rng.range(-4, 4)]);
            if !is_zero(&v) && !sh.contains(&v) {
                sh.push(v);
            }
        }
        let idx = index2(&sh);
        if idx == 0 {
            continue;
        }
        total += 1;
        let k = kappa_measured(&sh, 48);
        if k != kappa_measured(&sh, 64) {
            unstable += 1;
        }
        for dims in [[24, 24], [24, 37], [40, 25]] {
            if count_brute(&dims, &sh).0 != idx as i128 + k {
                bad += 1;
            }
        }
    }
    r.must(bad == 0 && unstable == 0, &format!("groups = index + corner constant on boxes 24x24, 24x37, 40x25 for {total} random systems (entries in [-4,4])"));
}

fn det3(a: &V, b: &V, c: &V) -> i64 {
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0])
}

fn g6(r: &mut Report) {
    println!("G6  three independent shifts in 3-D (COUNTEREXAMPLES, observations)");
    let sh = vec![vec![1, 1, 0], vec![1, 1, 1], vec![2, 1, 0]];
    let counts: Vec<i128> = [6, 8, 10, 12, 16].iter().map(|n| count_brute(&[*n, *n, *n], &sh).0).collect();
    r.must(det3(&sh[0], &sh[1], &sh[2]).abs() == 1 && counts == vec![13, 17, 21, 25, 33], "(1,1,0),(1,1,1),(2,1,0): determinant 1, but 2N + 1 groups on an N x N x N cube for N = 6, 8, 10, 12, 16");
    r.must(euler(&[10, 10, 10], &sh) == 3, "the same system on 10 x 10 x 10: points - links + squares - cubes = 3, not 21 (the two-shift formula does not extend)");
    r.must(reduce_chain(&[10, 10, 10], &sh).free >= 20, "its first dominated step alone releases 2N points (lower bound, PROVED)");
    let ns = normals(3, 2);
    let mut rng = Lcg(11);
    let (mut total, mut constant, mut linear, mut other, mut euler_ok) = (0u64, 0u64, 0u64, 0u64, 0u64);
    while total < 150 {
        let pick = |rng: &mut Lcg| ns[(rng.next() % ns.len() as u64) as usize].clone();
        let sh = vec![pick(&mut rng), pick(&mut rng), pick(&mut rng)];
        if det3(&sh[0], &sh[1], &sh[2]) == 0 {
            continue;
        }
        total += 1;
        let cs: Vec<i128> = [10, 12, 14, 16].iter().map(|n| count_brute(&[*n, *n, *n], &sh).0).collect();
        let d: Vec<i128> = cs.windows(2).map(|w| w[1] - w[0]).collect();
        if d.iter().all(|x| *x == 0) {
            constant += 1;
        } else if d.iter().all(|x| *x == d[0]) {
            linear += 1;
        } else {
            other += 1;
        }
        if euler(&[16, 16, 16], &sh) == cs[3] {
            euler_ok += 1;
        }
    }
    println!("    observation: {total} random independent triples (entries in [-2,2]) on cubes N = 10..16: constant {constant}, exactly linear {linear}, neither {other}; Euler count right for {euler_ok}");
    r.must(other == 0, "no tested triple grows faster than linearly on cubes N = 10, 12, 14, 16 (conjecture: degree at most d - 2)");
}

// G7: exact chains through steps that the theorem does not cover.
fn labels_of(dims: &[i64], links: &[(V, V)]) -> Vec<u32> {
    let mut pts: Vec<V> = vec![vec![]];
    for &n in dims {
        pts = pts.into_iter().flat_map(|p| (0..n).map(move |c| { let mut q = p.clone(); q.push(c); q })).collect();
    }
    let idx: HashMap<V, u32> = pts.iter().cloned().enumerate().map(|(i, v)| (v, i as u32)).collect();
    let mut u: Vec<u32> = (0..pts.len() as u32).collect();
    for (a, b) in links {
        let (x, y) = (find(&mut u, idx[a]), find(&mut u, idx[b]));
        if x != y {
            u[x as usize] = y;
        }
    }
    let mut seen: HashMap<u32, u32> = HashMap::new();
    (0..pts.len()).map(|i| { let r = find(&mut u, i as u32); let n = seen.len() as u32; *seen.entry(r).or_insert(n) }).collect()
}
fn family_links(dims: &[i64], shifts: &[V], offset: &V) -> Vec<(V, V)> {
    // links of the shifts inside the box `dims` translated by `offset`
    let mut pts: Vec<V> = vec![vec![]];
    for &n in dims {
        pts = pts.into_iter().flat_map(|p| (0..n).map(move |c| { let mut q = p.clone(); q.push(c); q })).collect();
    }
    let mut out = vec![];
    for v in &pts {
        for q in shifts {
            let w: V = v.iter().zip(q).map(|(a, b)| a + b).collect();
            if w.iter().zip(dims).all(|(c, n)| 0 <= *c && c < n) {
                out.push((v.iter().zip(offset).map(|(a, b)| a + b).collect(), w.iter().zip(offset).map(|(a, b)| a + b).collect()));
            }
        }
    }
    out
}
/// Is there a chain of exact single-shift reductions (any shift, any
/// orientations, dominated or not) down to a system without links?
fn exact_chain(dims: &[i64], shifts: &[V], memo: &mut HashMap<(Vec<i64>, Vec<V>), bool>) -> bool {
    let mut act = clean(dims, shifts);
    act.sort();
    if act.is_empty() {
        return true;
    }
    let key = (dims.to_vec(), act.clone());
    if let Some(x) = memo.get(&key) {
        return *x;
    }
    let zero = vec![0; dims.len()];
    let truth = labels_of(dims, &family_links(dims, &act, &zero));
    let mut found = false;
    'search: for pi in 0..act.len() {
        for sign in [1i64, -1] {
            let p: V = act[pi].iter().map(|c| c * sign).collect();
            let others: Vec<&V> = act.iter().enumerate().filter(|(i, _)| *i != pi).map(|(_, q)| q).collect();
            // D' = { v : v + p in D } is the box of sides n_i - |p_i| at this offset
            let inner: Vec<i64> = dims.iter().zip(&p).map(|(n, c)| n - c.abs()).collect();
            let offset: V = p.iter().map(|c| (-c).max(0)).collect();
            for mask in 0..(1u32 << others.len()) {
                let mut reduced = vec![p.clone()];
                for (i, q) in others.iter().enumerate() {
                    let q = if mask >> i & 1 == 1 { neg(q) } else { (*q).clone() };
                    let d = sub(&q, &p);
                    if !is_zero(&d) {
                        reduced.push(d);
                    }
                }
                // (b) the reduced system on D', (a) links back from D \ D'
                let mut links = family_links(&inner, &reduced, &offset);
                let mut pts: Vec<V> = vec![vec![]];
                for &n in dims {
                    pts = pts.into_iter().flat_map(|v| (0..n).map(move |c| { let mut q = v.clone(); q.push(c); q })).collect();
                }
                for t in &pts {
                    let inside = |v: &V| v.iter().zip(dims).all(|(c, n)| 0 <= *c && c < n);
                    let fwd: V = t.iter().zip(&p).map(|(a, b)| a + b).collect();
                    let back: V = t.iter().zip(&p).map(|(a, b)| a - b).collect();
                    if !inside(&fwd) && inside(&back) {
                        links.push((t.clone(), back));
                    }
                }
                if labels_of(dims, &links) == truth && exact_chain(&inner, &reduced, memo) {
                    found = true;
                    break 'search;
                }
            }
        }
    }
    memo.insert(key, found);
    found
}

fn g7(r: &mut Report) {
    println!("G7  exact chains through non-dominated steps (TESTED ONLY; not covered by a theorem)");
    let mut memo = HashMap::new();
    let unit = vec![vec![1, 0], vec![0, 1]];
    let (mut as_expected, mut cells_tested) = (0, 0);
    let mut rows = vec![];
    for a in 2..=6i64 {
        let mut row = String::new();
        for b in 2..=6i64 {
            cells_tested += 1;
            let chain = exact_chain(&[a, b], &unit, &mut memo);
            let stuck = reduce_chain(&[a, b], &unit).steps == 0;
            row.push(if chain { 'E' } else { '-' });
            if stuck && chain == (a % 2 == 1 || b % 2 == 1) {
                as_expected += 1;
            }
        }
        rows.push(row);
    }
    r.must(as_expected == cells_tested, "unit shifts (1,0),(0,1) on a x b, 2 <= a, b <= 6: no dominated shift; an exact chain exists exactly when a or b is odd");
    println!("    rows a = 2..6, columns b = 2..6 (E: exact chain, -: none): {}", rows.join(" "));
    r.must(!exact_chain(&[2, 2], &unit, &mut memo) && exact_chain(&[3, 2], &unit, &mut memo), "2 x 2 has no exact chain; 3 x 2 has one although no shift is dominated");
    let (mut systems, mut dom, mut exact_only, mut none) = (0u64, 0u64, 0u64, 0u64);
    for (dims, k) in [(vec![3i64, 3], 3usize), (vec![4, 3], 3), (vec![4, 4], 2), (vec![2, 2, 2], 2)] {
        for s in subsets(&all_active_normals(&dims), k) {
            systems += 1;
            if reduce_chain(&dims, &s).core.is_empty() {
                dom += 1;
            } else if exact_chain(&dims, &s, &mut memo) {
                exact_only += 1;
            } else {
                none += 1;
            }
        }
    }
    println!("    observation: {systems} small systems: dominated chain completes {dom}; only a non-dominated exact chain {exact_only}; no exact chain {none}");
}

// -------------------------------------------------------------- bench

fn bench() {
    println!("Counting groups at scale: union-find over every link vs reduce-then-count.");
    println!("{:<34} {:<16} {:>14} {:>12} {:>9} {:>10} {:>9} {:>6}", "shifts", "box", "groups", "links", "brute s", "hybrid s", "finish", "steps/jumps");
    let cases: Vec<(Vec<i64>, Vec<V>)> = vec![
        (vec![3000, 3000], vec![vec![2, 1], vec![1, 2]]),
        (vec![3000, 3000], vec![vec![40, 3], vec![7, 55]]),
        (vec![3000, 3000], vec![vec![1, 1], vec![2, 2], vec![5, 5]]),
        (vec![3000, 3000], vec![vec![1, 0], vec![3, 1], vec![2, 5]]),
        (vec![3000, 3000], vec![vec![3, 2], vec![6, 5], vec![9, 9]]),
        (vec![200, 200, 200], vec![vec![1, 1, 0], vec![2, 3, 1]]),
        (vec![200, 200, 200], vec![vec![1, 1, 0], vec![1, 1, 1], vec![2, 1, 0]]),
        (vec![60, 60, 60, 60], vec![vec![1, 0, 2, 0], vec![0, 3, 1, 1]]),
        (vec![8_000_000], vec![vec![3], vec![5]]),
    ];
    for (dims, sh) in cases {
        let t = Instant::now();
        let (g, links) = count_brute(&dims, &sh);
        let tb = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let (h, how, _) = count_hybrid(&dims, &sh);
        let th = t.elapsed().as_secs_f64();
        let c = reduce_chain(&dims, &sh);
        println!("{:<34} {:<16} {:>14} {:>12} {:>9.3} {:>10.6} {:>9} {:>6}  {}", format!("{sh:?}"), format!("{dims:?}"), g, links, tb, th, format!("{how:?}"), format!("{}/{}", c.steps, c.jumps), if g == h { "agree" } else { "DISAGREE" });
    }
    println!("Boxes too large for brute force (formula only):");
    for (dims, sh) in [(vec![10i64.pow(9), 10i64.pow(9)], vec![vec![40i64, 3], vec![7, 55]]), (vec![10i64.pow(6); 5], vec![vec![1i64, 2, 0, 1, 3], vec![2, 0, 1, 1, 0]])] {
        let (h, how, _) = count_hybrid(&dims, &sh);
        println!("  {sh:?} on {dims:?}: {h} groups ({how:?})");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parse = |s: &str| -> V { s.split(',').map(|x| x.trim().parse().expect("whole numbers separated by commas")).collect() };
    match args.first().map(|s| s.as_str()) {
        None | Some("check") => {
            let mut r = Report { fails: 0 };
            g1(&mut r);
            g2(&mut r);
            g3_g4(&mut r);
            g5(&mut r);
            g6(&mut r);
            g7(&mut r);
            println!("grid checks failed: {}", r.fails);
            std::process::exit(if r.fails == 0 { 0 } else { 1 });
        }
        Some("bench") => bench(),
        Some("count") if args.len() >= 3 => {
            let dims = parse(&args[1]);
            let sh: Vec<V> = args[2..].iter().map(|s| parse(s)).collect();
            if sh.iter().any(|q| q.len() != dims.len()) {
                eprintln!("every shift needs as many numbers as the box has sides");
                std::process::exit(2);
            }
            let (h, how, _) = count_hybrid(&dims, &sh);
            println!("reduce-then-count: {h} groups (finished by {how:?})");
            if volume(&dims) <= 50_000_000 {
                println!("union-find over every link: {} groups", count_brute(&dims, &sh).0);
            }
            if volume(&dims) <= 200_000 {
                println!("breadth-first search: {} groups", count_bfs(&dims, &sh));
            }
        }
        _ => {
            eprintln!("usage: [check] | bench | count N1,N2[,..] A,B[,..] [more shifts]");
            std::process::exit(2);
        }
    }
}
