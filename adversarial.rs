#!/bin/sh
//bin/true 2>/dev/null; d="$(dirname "$0")"; t="${TMPDIR:-/tmp}/prefsuf_adv"; [ -f "$d/prefsuf.rs" ] || { echo "put prefsuf.rs in the same folder as $0"; exit 1; }; mkdir -p "$t" && tail -n +3 "$d/prefsuf.rs" | sed 's/^fn main() {/fn orig_main() {/' > "$t/core.rs" && tail -n +3 "$0" > "$t/main.rs" && rustc --edition 2021 -O "$t/main.rs" -o "$t/run" && exec "$t/run" "$@"; exit $?
// Adversarial test harness for prefsuf.rs. Keep prefsuf.rs in the same folder.
//
//     sh adversarial.rs                  all correctness sections (A-G), seed 1
//     sh adversarial.rs check 7          same, another seed
//     sh adversarial.rs perf SHAPE N     one timing/memory measurement (Linux)
//
// Nothing here trusts the solver. Every answer is compared with something
// computed in this file from the definition of the rules:
//   * small puzzles: every possible text is tried (brute force) and the
//     solver's pattern is expanded and compared with that list exactly;
//   * larger puzzles: groups of equal positions are found by a plain
//     breadth-first search over the border links, the expected letters and
//     free groups are derived from them, and the solver's pattern must match
//     position by position;
//   * every "no solution" explanation is re-checked link by link, and its
//     length is compared with an independently computed shortest distance.
// A failure prints the section, the seed and the exact puzzle as a command
// line that reproduces it.
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
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// The puzzle as a command line (words longer than 60 letters are abbreviated).
fn cli(n: usize, al: &[u8], cons: &[Con]) -> String {
    let mut s = format!("sh prefsuf.rs {n} '{}'", String::from_utf8_lossy(al));
    for (i, c) in cons.iter().enumerate() {
        if i == 40 {
            s.push_str(&format!(" ...({} more rules)", cons.len() - 40));
            break;
        }
        let w = |w: &Vec<u8>| {
            if w.len() <= 60 {
                String::from_utf8_lossy(w).into_owned()
            } else {
                format!("<{} letters: {}...{}>", w.len(), String::from_utf8_lossy(&w[..8]), String::from_utf8_lossy(&w[w.len() - 8..]))
            }
        };
        match c {
            Con::Prefix(x) => s.push_str(&format!(" 'P{}'", w(x))),
            Con::Suffix(x) => s.push_str(&format!(" 'S{}'", w(x))),
            Con::Border(k) => s.push_str(&format!(" B{k}")),
        }
    }
    s
}

// ------------------------------------------------ independent definitions

/// Does text s obey every rule? Written from the rule definitions only.
fn holds(s: &[u8], cons: &[Con]) -> bool {
    let n = s.len();
    for c in cons {
        match c {
            Con::Prefix(w) => {
                if w.len() > n {
                    return false;
                }
                for i in 0..w.len() {
                    if s[i] != w[i] {
                        return false;
                    }
                }
            }
            Con::Suffix(w) => {
                if w.len() > n {
                    return false;
                }
                for i in 0..w.len() {
                    if s[n - w.len() + i] != w[i] {
                        return false;
                    }
                }
            }
            Con::Border(k) => {
                if *k > n {
                    return false;
                }
                for i in 0..*k {
                    if s[i] != s[n - *k + i] {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Every text of length n over al, as an odometer; calls f on each.
fn each_text(n: usize, al: &[u8], mut f: impl FnMut(&[u8])) {
    let mut d = vec![0usize; n];
    let mut s: Vec<u8> = vec![al[0]; n];
    loop {
        f(&s);
        let mut i = 0;
        loop {
            if i == n {
                return;
            }
            d[i] += 1;
            if d[i] < al.len() {
                s[i] = al[d[i]];
                break;
            }
            d[i] = 0;
            s[i] = al[0];
            i += 1;
        }
    }
}

fn all_solutions(n: usize, al: &[u8], cons: &[Con]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    each_text(n, al, |s| {
        if holds(s, cons) {
            out.push(s.to_vec());
        }
    });
    out.sort();
    out
}

fn pow_exact(base: usize, exp: usize) -> Option<u128> {
    let mut r: u128 = 1;
    for _ in 0..exp {
        r = r.checked_mul(base as u128)?;
        if base == 1 {
            break;
        }
    }
    Some(r)
}

/// Letter that rule idx fixes at position pos, if any.
fn fixed_by(n: usize, cons: &[Con], idx: usize, pos: usize) -> Option<u8> {
    match cons.get(idx)? {
        Con::Prefix(w) => w.get(pos).copied(),
        Con::Suffix(w) => {
            let start = n.checked_sub(w.len())?;
            w.get(pos.checked_sub(start)?).copied()
        }
        Con::Border(_) => None,
    }
}

/// Is the "no solution" explanation a real proof? Checked from the definition.
fn cert_ok(cf: &Conflict, n: usize, cons: &[Con]) -> Result<(), String> {
    if cf.path.is_empty() {
        return Err("certificate has an empty path".into());
    }
    if cf.via.len() + 1 != cf.path.len() {
        return Err("certificate: number of links does not match the path".into());
    }
    if cf.letter_a == cf.letter_b {
        return Err("certificate: the two letters are equal".into());
    }
    if cf.path.iter().any(|p| *p >= n) {
        return Err("certificate: position outside the text".into());
    }
    if fixed_by(n, cons, cf.src_a, cf.path[0]) != Some(cf.letter_a) {
        return Err("certificate: first letter is not fixed by the cited rule".into());
    }
    if fixed_by(n, cons, cf.src_b, cf.path[cf.path.len() - 1]) != Some(cf.letter_b) {
        return Err("certificate: second letter is not fixed by the cited rule".into());
    }
    for i in 0..cf.via.len() {
        let (u, v) = (cf.path[i].min(cf.path[i + 1]), cf.path[i].max(cf.path[i + 1]));
        let ok = match cons.get(cf.via[i]) {
            // Border(k): position j equals position j + (n - k) for every j < k.
            Some(Con::Border(k)) if *k <= n => u < *k && v == u + (n - *k),
            _ => false,
        };
        if !ok {
            return Err(format!("certificate: link {i} ({u} = {v}) does not follow from the cited rule"));
        }
    }
    Ok(())
}

fn periods_of(n: usize, cons: &[Con]) -> Vec<usize> {
    let mut p: Vec<usize> = cons
        .iter()
        .filter_map(|c| match c {
            Con::Border(k) if *k > 0 && *k < n => Some(n - *k),
            _ => None,
        })
        .collect();
    p.sort();
    p.dedup();
    p
}

/// Groups of positions forced equal, by breadth-first search over the links
/// u ~ u + p (p a border period, u + p < n). Cost: n * number of periods.
fn components(n: usize, periods: &[usize]) -> (Vec<u32>, usize) {
    let mut comp = vec![u32::MAX; n];
    let mut count = 0u32;
    let mut queue: Vec<usize> = Vec::new();
    for start in 0..n {
        if comp[start] != u32::MAX {
            continue;
        }
        comp[start] = count;
        queue.clear();
        queue.push(start);
        let mut head = 0;
        while head < queue.len() {
            let u = queue[head];
            head += 1;
            for &p in periods {
                if p > u && u + p >= n {
                    continue;
                }
                if u + p < n && comp[u + p] == u32::MAX {
                    comp[u + p] = count;
                    queue.push(u + p);
                }
                if u >= p && comp[u - p] == u32::MAX {
                    comp[u - p] = count;
                    queue.push(u - p);
                }
            }
        }
        count += 1;
    }
    (comp, count as usize)
}

/// Fewest links needed to get from a to b (None if not connected).
fn distance(n: usize, periods: &[usize], a: usize, b: usize) -> Option<usize> {
    let mut dist = vec![u32::MAX; n];
    dist[a] = 0;
    let mut queue = vec![a];
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        if u == b {
            return Some(dist[u] as usize);
        }
        for &p in periods {
            if u + p < n && dist[u + p] == u32::MAX {
                dist[u + p] = dist[u] + 1;
                queue.push(u + p);
            }
            if u >= p && dist[u - p] == u32::MAX {
                dist[u - p] = dist[u] + 1;
                queue.push(u - p);
            }
        }
    }
    None
}

// ------------------------------------------------------------ the checks

#[derive(Default, Clone, Copy)]
struct Info {
    sat: bool,
    free: usize,
    groups: usize,
    cert_len: usize,
}

/// Small puzzle: compare with the list of all texts that obey the rules.
fn check_small(n: usize, al: &[u8], cons: &[Con]) -> Result<Info, String> {
    let sols = all_solutions(n, al, cons);
    match solve(n, al, cons) {
        Err(e) => Err(format!("solver rejected a valid puzzle: {e}")),
        Ok(Outcome::Unsat(cf)) => {
            if !sols.is_empty() {
                return Err(format!("solver says no solution but {} exist, e.g. {}", sols.len(), String::from_utf8_lossy(&sols[0])));
            }
            cert_ok(&cf, n, cons)?;
            let d = distance(n, &periods_of(n, cons), cf.path[0], cf.path[cf.path.len() - 1]);
            if d != Some(cf.via.len()) {
                return Err(format!("explanation has {} links, shortest possible is {d:?}", cf.via.len()));
            }
            Ok(Info { sat: false, cert_len: cf.path.len(), ..Info::default() })
        }
        Ok(Outcome::Sat { pattern, free }) => {
            if pattern.len() != n {
                return Err("pattern has the wrong length".into());
            }
            if sols.is_empty() {
                return Err("solver says solvable but no text obeys the rules".into());
            }
            let total = pow_exact(al.len(), free).ok_or("count overflow on a small puzzle")?;
            if total != sols.len() as u128 {
                return Err(format!("pattern describes {total} texts, brute force finds {}", sols.len()));
            }
            let mut mine: Vec<Vec<u8>> = Vec::new();
            let mut bad_id = false;
            each_text(free, al, |fill| {
                mine.push(
                    pattern
                        .iter()
                        .map(|c| match c {
                            Cell::Letter(ch) => *ch,
                            Cell::Free(i) => {
                                if *i >= free {
                                    bad_id = true;
                                    al[0]
                                } else {
                                    fill[*i]
                                }
                            }
                        })
                        .collect(),
                );
            });
            if bad_id {
                return Err("pattern uses a free-group number that does not exist".into());
            }
            mine.sort();
            mine.dedup();
            if mine != sols {
                return Err("the texts described by the pattern are not exactly the solutions".into());
            }
            if count(al.len(), free) != Some(sols.len() as u128) {
                return Err(format!("count() says {:?}, brute force finds {}", count(al.len(), free), sols.len()));
            }
            if is_unique(al.len(), free) != (sols.len() == 1) {
                return Err("uniqueness flag disagrees with brute force".into());
            }
            Ok(Info { sat: true, free, ..Info::default() })
        }
    }
}

/// Larger puzzle: derive the answer from independently computed groups.
/// Cost about n * (number of distinct borders), so keep that product modest.
fn check_big(n: usize, al: &[u8], cons: &[Con]) -> Result<Info, String> {
    let periods = periods_of(n, cons);
    let (comp, ncomp) = components(n, &periods);
    let mut lit = vec![0u8; ncomp];
    let mut clash = false;
    for c in cons {
        let (w, off) = match c {
            Con::Prefix(w) => (w, 0),
            Con::Suffix(w) => (w, n - w.len()),
            Con::Border(_) => continue,
        };
        for j in 0..w.len() {
            let g = comp[off + j] as usize;
            if lit[g] == 0 {
                lit[g] = w[j];
            } else if lit[g] != w[j] {
                clash = true;
            }
        }
    }
    match solve(n, al, cons) {
        Err(e) => Err(format!("solver rejected a valid puzzle: {e}")),
        Ok(Outcome::Unsat(cf)) => {
            if !clash {
                return Err("solver says no solution, but the puzzle is solvable".into());
            }
            cert_ok(&cf, n, cons)?;
            let (a, b) = (cf.path[0], cf.path[cf.path.len() - 1]);
            if (n as u128) * (periods.len().max(1) as u128) <= 400_000_000 {
                let d = distance(n, &periods, a, b);
                if d != Some(cf.via.len()) {
                    return Err(format!("explanation has {} links, shortest possible is {d:?}", cf.via.len()));
                }
            }
            Ok(Info { sat: false, free: 0, groups: ncomp, cert_len: cf.path.len() })
        }
        Ok(Outcome::Sat { pattern, free }) => {
            if clash {
                return Err("solver says solvable, but two rules clash".into());
            }
            if pattern.len() != n {
                return Err("pattern has the wrong length".into());
            }
            let mut comp_id = vec![usize::MAX; ncomp];
            let mut id_used = vec![false; free];
            let mut next = 0usize;
            for i in 0..n {
                let g = comp[i] as usize;
                match pattern[i] {
                    Cell::Letter(ch) => {
                        if lit[g] == 0 {
                            return Err(format!("position {i}: solver fixed a letter the rules do not force"));
                        }
                        if lit[g] != ch {
                            return Err(format!("position {i}: solver fixed the wrong letter"));
                        }
                    }
                    Cell::Free(id) => {
                        if lit[g] != 0 {
                            return Err(format!("position {i}: solver left free a position the rules force"));
                        }
                        if id >= free {
                            return Err(format!("position {i}: free-group number out of range"));
                        }
                        if comp_id[g] == usize::MAX {
                            if id != next || id_used[id] {
                                return Err(format!("position {i}: free groups merged or misnumbered"));
                            }
                            comp_id[g] = id;
                            id_used[id] = true;
                            next += 1;
                        } else if comp_id[g] != id {
                            return Err(format!("position {i}: one group of equal positions got two free numbers"));
                        }
                    }
                }
            }
            if next != free {
                return Err(format!("solver reports {free} free groups, pattern uses {next}"));
            }
            let want_free = lit.iter().filter(|l| **l == 0).count();
            if free != want_free {
                return Err(format!("solver reports {free} free groups, expected {want_free}"));
            }
            Ok(Info { sat: true, free, groups: ncomp, cert_len: 0 })
        }
    }
}

struct Sec {
    name: &'static str,
    seed: u64,
    checks: usize,
    fails: usize,
    sat: usize,
    unsat: usize,
    max_n: usize,
    max_cert: usize,
    max_free: usize,
    max_borders: usize,
    start: Instant,
}

impl Sec {
    fn new(name: &'static str, seed: u64) -> Sec {
        println!("--- {name}");
        Sec { name, seed, checks: 0, fails: 0, sat: 0, unsat: 0, max_n: 0, max_cert: 0, max_free: 0, max_borders: 0, start: Instant::now() }
    }
    fn fail(&mut self, what: &str, n: usize, al: &[u8], cons: &[Con]) {
        self.fails += 1;
        if self.fails <= 8 {
            println!("FAIL [{}] seed={} : {what}\n     reproduce: {}", self.name, self.seed, cli(n, al, cons));
        }
    }
    fn record(&mut self, r: Result<Info, String>, n: usize, al: &[u8], cons: &[Con]) -> Option<Info> {
        self.checks += 1;
        self.max_n = self.max_n.max(n);
        self.max_borders = self.max_borders.max(periods_of(n, cons).len());
        match r {
            Ok(i) => {
                if i.sat { self.sat += 1 } else { self.unsat += 1 }
                self.max_cert = self.max_cert.max(i.cert_len);
                self.max_free = self.max_free.max(i.free);
                Some(i)
            }
            Err(e) => {
                self.fail(&e, n, al, cons);
                None
            }
        }
    }
    fn small(&mut self, n: usize, al: &[u8], cons: &[Con]) -> Option<Info> {
        self.record(check_small(n, al, cons), n, al, cons)
    }
    fn big(&mut self, n: usize, al: &[u8], cons: &[Con]) -> Option<Info> {
        self.record(check_big(n, al, cons), n, al, cons)
    }
    /// A claim that must hold; counted as its own check.
    fn expect(&mut self, ok: bool, what: &str, n: usize, al: &[u8], cons: &[Con]) {
        self.checks += 1;
        if !ok {
            self.fail(what, n, al, cons);
        }
    }
    fn done(&self) -> usize {
        println!(
            "    {} checks, {} FAILED; {} solvable, {} contradictory; longest text {}, most distinct borders {}, most free groups {}, longest explanation {}; {:.1}s",
            self.checks, self.fails, self.sat, self.unsat, self.max_n, self.max_borders, self.max_free, self.max_cert,
            self.start.elapsed().as_secs_f64()
        );
        self.fails
    }
}

fn words(len: usize, al: &[u8]) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    each_text(len, al, |s| v.push(s.to_vec()));
    v
}

// A. borders whose lengths differ by one, two or three characters
fn section_a(seed: u64) -> usize {
    let mut s = Sec::new("A nearly contradictory borders", seed);
    // every subset of {n-1, n-2, n-3, n-4} with every pair of short words
    for n in 1..=11usize {
        for al in [&b"ab"[..], &b"abc"[..]] {
            if al.len() == 3 && n > 8 {
                continue;
            }
            let cand: Vec<usize> = (1..=4).filter(|d| *d <= n).map(|d| n - d).collect();
            for mask in 1u32..(1 << cand.len()) {
                let borders: Vec<Con> = (0..cand.len()).filter(|i| mask >> i & 1 == 1).map(|i| Con::Border(cand[i])).collect();
                for pl in 0..=2usize.min(n) {
                    for sl in 0..=2usize.min(n) {
                        for pw in words(pl, al) {
                            for sw in words(sl, al) {
                                let mut cons = borders.clone();
                                cons.push(Con::Prefix(pw.clone()));
                                cons.push(Con::Suffix(sw.clone()));
                                s.small(n, al, &cons);
                            }
                        }
                    }
                }
            }
        }
    }
    // large texts: contradiction (or not) between the first and last character
    for &n in &[1000usize, 1001, 99_999, 100_000, 1_000_003, 2_000_000] {
        let h = n / 2;
        let t = n / 3;
        let sets: Vec<Vec<usize>> = vec![
            vec![1, 2], vec![2, 3], vec![1, 3], vec![2, 4], vec![3, 6], vec![4, 6], vec![6, 9],
            vec![1, 2, 3], vec![2, 3, 4], vec![2, 4, 6], vec![6, 10, 15],
            vec![500, 501], vec![500, 502], vec![500, 503], vec![499, 500, 501],
            vec![t, t + 1], vec![t, t + 2], vec![t, t + 3],
            vec![h - 1, h], vec![h, h + 1], vec![h + 1, h + 2], vec![h, h + 2], vec![h - 1, h + 1],
            vec![n - 3, n - 2], vec![n - 2, n - 1], vec![n - 3, n - 1], vec![n - 4, n - 2],
        ];
        for periods in sets {
            if periods.iter().any(|p| *p == 0 || *p >= n) {
                continue;
            }
            let borders: Vec<Con> = periods.iter().map(|p| Con::Border(n - p)).collect();
            // first and last character
            let mut cons = borders.clone();
            cons.push(Con::Prefix(b"a".to_vec()));
            cons.push(Con::Suffix(b"b".to_vec()));
            s.big(n, b"ab", &cons);
            // a longer prefix against a longer suffix, clash possible anywhere
            let mut cons = borders.clone();
            cons.push(Con::Suffix(b"abbab".to_vec()));
            cons.push(Con::Prefix(b"abbab".to_vec()));
            s.big(n, b"ab", &cons);
            // borders only
            s.big(n, b"ab", &borders);
        }
    }
    s.done()
}

// B. either side of the Fine-Wilf bound
fn section_b(seed: u64) -> usize {
    let mut s = Sec::new("B Fine-Wilf boundary", seed);
    let ab = b"ab";
    // Small: brute force, plus the theorem itself as a second opinion.
    // Periods p and q on a text of length n >= p + q - gcd(p, q) force period
    // gcd(p, q): exactly gcd(p, q) free groups. One character shorter, and
    // (when neither period divides the other) more freedom must remain.
    for p in 1..=9usize {
        for q in p..=9usize {
            let g = gcd(p, q);
            let bound = p + q - g;
            for n in (q + 1)..=(p + q + 2).min(17) {
                let cons = [Con::Border(n - p), Con::Border(n - q)];
                if let Some(i) = s.small(n, ab, &cons) {
                    if n >= bound {
                        s.expect(i.free == g, "at/above the Fine-Wilf bound the number of free groups must be gcd(p,q)", n, ab, &cons);
                    } else if n + 1 == bound && p % g == 0 && q % p != 0 {
                        s.expect(i.free > g, "one below the Fine-Wilf bound more freedom must remain", n, ab, &cons);
                    }
                }
                // with letters at both ends
                let cons = [Con::Border(n - p), Con::Prefix(b"a".to_vec()), Con::Border(n - q), Con::Suffix(b"b".to_vec())];
                s.small(n, ab, &cons);
            }
        }
    }
    // Large: random periods with a chosen common divisor, at bound - 1, bound, bound + 1.
    let mut r = X(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    for &scale in &[50usize, 2_000, 100_000, 1_000_000] {
        for &g in &[1usize, 2, 3, 7, 30] {
            for _ in 0..6 {
                let (a, b) = (1 + r.below(scale / g + 1), 1 + r.below(scale / g + 1));
                let (p, q) = (g * a.min(b), g * a.max(b));
                let gg = gcd(p, q);
                let bound = p + q - gg;
                for n in [bound.saturating_sub(1), bound, bound + 1, bound + r.below(scale) + 2] {
                    if n <= q {
                        continue;
                    }
                    let cons = [Con::Border(n - q), Con::Border(n - p)];
                    if let Some(i) = s.big(n, ab, &cons) {
                        if n >= bound {
                            s.expect(i.free == gg, "at/above the Fine-Wilf bound the number of free groups must be gcd(p,q)", n, ab, &cons);
                        } else if q % p != 0 {
                            s.expect(i.free > gg, "one below the Fine-Wilf bound more freedom must remain", n, ab, &cons);
                        }
                    }
                    // Letters at both ends: at or above the bound the first and
                    // last characters are in one group exactly when gcd divides n - 1.
                    let cons = [Con::Prefix(b"a".to_vec()), Con::Border(n - q), Con::Border(n - p), Con::Suffix(b"b".to_vec())];
                    if let Some(i) = s.big(n, ab, &cons) {
                        if n >= bound {
                            s.expect(i.sat == ((n - 1) % gg != 0), "first/last clash must happen exactly when gcd divides n-1", n, ab, &cons);
                        }
                    }
                }
            }
        }
    }
    s.done()
}

// C. as many distinct borders as possible
fn section_c(seed: u64) -> usize {
    let mut s = Sec::new("C maximum distinct borders", seed);
    let mut r = X(seed.wrapping_mul(0xD1B54A32D192ED03) | 1);
    let ab = b"ab";
    for &n in &[1usize, 2, 3, 7, 50, 501, 3000] {
        let all: Vec<usize> = (0..=n).collect();
        let families: Vec<(&str, Vec<usize>)> = vec![
            ("all", all.clone()),
            ("even periods", all.iter().copied().filter(|k| (n - k) % 2 == 0).collect()),
            ("periods divisible by 3", all.iter().copied().filter(|k| (n - k) % 3 == 0).collect()),
            ("short borders (k <= n/2)", all.iter().copied().filter(|k| *k <= n / 2).collect()),
            ("long borders (k >= n/2)", all.iter().copied().filter(|k| *k >= n / 2).collect()),
            ("random half", all.iter().copied().filter(|_| r.below(2) == 0).collect()),
            ("periods 6j and 10j", all.iter().copied().filter(|k| (n - k) % 6 == 0 || (n - k) % 10 == 0).collect()),
            // even periods from n/2 upward: two groups survive however many borders
            ("even periods >= n/2", all.iter().copied().filter(|k| (n - k) % 2 == 0 && n - k >= n / 2).collect()),
            // a block written twice, the block itself repeating every 3
            ("block twice, block period 3", {
                let h = n / 2;
                (0..=h / 3).map(|j| h - 3 * j).chain([n - h]).collect()
            }),
            // Fibonacci periods: the slowest case for Euclid-style reductions
            ("Fibonacci periods", {
                let (mut a, mut b, mut v) = (1usize, 2usize, Vec::new());
                while b < n {
                    v.push(n - b);
                    (a, b) = (b, a + b);
                }
                v
            }),
            ("Fibonacci border lengths", {
                let (mut a, mut b, mut v) = (1usize, 2usize, Vec::new());
                while b < n {
                    v.push(b);
                    (a, b) = (b, a + b);
                }
                v
            }),
            ("random tenth", all.iter().copied().filter(|_| r.below(10) == 0).collect()),
            ("random sqrt(n) borders", (0..(n as f64).sqrt() as usize + 1).map(|_| r.below(n + 1)).collect()),
        ];
        for (_, ks) in families {
            for order in 0..4 {
                let mut ks = ks.clone();
                match order {
                    0 => {}
                    1 => ks.reverse(),
                    2 => r.shuffle(&mut ks),
                    _ => {
                        // repeats mixed with distinct
                        let extra: Vec<usize> = (0..ks.len()).map(|_| ks[r.below(ks.len().max(1))]).collect();
                        ks.extend(extra);
                        r.shuffle(&mut ks);
                    }
                }
                let borders: Vec<Con> = ks.iter().map(|k| Con::Border(*k)).collect();
                let plain = s.big(n, ab, &borders);
                // same rules, same answer whatever the order
                if order > 0 {
                    let mut sorted: Vec<Con> = ks.iter().map(|k| Con::Border(*k)).collect();
                    sorted.sort_by_key(|c| if let Con::Border(k) = c { *k } else { 0 });
                    sorted.dedup();
                    let same = match (solve(n, ab, &borders), solve(n, ab, &sorted)) {
                        (Ok(x), Ok(y)) => x == y,
                        _ => false,
                    };
                    s.expect(same, "answer depends on the order or repetition of borders", n, ab, &borders);
                }
                let _ = plain;
                // with a letter at each end (usually a contradiction)
                let mut cons = borders.clone();
                cons.insert(cons.len() / 2, Con::Prefix(b"a".to_vec()));
                cons.push(Con::Suffix(b"b".to_vec()));
                s.big(n, ab, &cons);
                // clash in the middle of the text
                if n >= 4 {
                    let mut w = vec![b'a'; n / 2 + 1];
                    w[n / 2] = b'b';
                    let mut cons = borders.clone();
                    cons.push(Con::Prefix(w));
                    s.big(n, ab, &cons);
                }
            }
        }
    }
    // one bigger case: every border of a 20,000-character text
    let n = 20_000;
    let mut cons: Vec<Con> = (1..n).map(Con::Border).collect();
    s.big(n, ab, &cons);
    cons.push(Con::Prefix(b"a".to_vec()));
    cons.push(Con::Suffix(b"b".to_vec()));
    s.big(n, ab, &cons);
    s.done()
}

// D. contradictions that only show after a very long chain
fn section_d(seed: u64) -> usize {
    let mut s = Sec::new("D contradictions at extreme distance", seed);
    let mut r = X(seed.wrapping_mul(0xA24BAED4963EE407) | 1);
    let ab = b"ab";
    // Chain: every character equals its neighbour; first is a, last is b.
    for &n in &[2usize, 3, 1000, 100_000, 2_000_000] {
        let cons = [Con::Border(n - 1), Con::Prefix(b"a".to_vec()), Con::Suffix(b"b".to_vec())];
        if let Some(i) = s.big(n, ab, &cons) {
            s.expect(!i.sat && i.cert_len == n, "chain explanation must visit every character", n, ab, &cons);
        }
    }
    // Coprime periods p < q exactly at the Fine-Wilf bound, n = p + q - 1.
    // The links then form one single path through all n positions whose two
    // ends are positions p - 1 and q - 1. Fix 'a' at one end and 'b' at the
    // other: the only explanation walks through every character of the text.
    let mut pairs: Vec<(usize, usize)> = vec![(2, 3), (3, 5), (5, 7), (7, 12), (99, 100), (1000, 1001), (4999, 7001)];
    for &scale in &[100_000usize, 1_000_000] {
        for _ in 0..3 {
            loop {
                let (a, b) = (2 + r.below(scale), 2 + r.below(scale));
                if a != b && gcd(a, b) == 1 {
                    pairs.push((a.min(b), a.max(b)));
                    break;
                }
            }
        }
    }
    pairs.push((1_000_000, 1_000_001));
    for (p, q) in pairs {
        let n = p + q - 1;
        let mut tail = vec![b'a'; p];
        tail[0] = b'b'; // position q - 1
        let cons = [
            Con::Suffix(vec![b'a'; q]), // fixes position p - 1 first
            Con::Border(n - p),
            Con::Border(n - q),
            Con::Suffix(tail),
        ];
        if let Some(i) = s.big(n, ab, &cons) {
            s.expect(!i.sat && i.cert_len == n, "single-path explanation must visit every character", n, ab, &cons);
            s.expect(i.groups == 1, "coprime periods at the bound must give one group", n, ab, &cons);
        }
    }
    // Planted puzzles: many agreeing words, one corrupted letter far away.
    for trial in 0..400 {
        let n = if trial < 300 { 20 + r.below(3000) } else { 50_000 + r.below(200_000) };
        let m = 1 + r.below(4);
        let step = 1 + r.below(1 + n / 40);
        let cons_b: Vec<Con> = (0..m).map(|_| Con::Border(n - (step * (1 + r.below(6))).min(n - 1))).collect();
        let periods = periods_of(n, &cons_b);
        let (comp, ncomp) = components(n, &periods);
        let letters: Vec<u8> = (0..ncomp).map(|_| ab[r.below(2)]).collect();
        let hidden: Vec<u8> = comp.iter().map(|c| letters[*c as usize]).collect();
        let mut cons = cons_b.clone();
        for _ in 0..1 + r.below(4) {
            let len = 1 + r.below(n);
            cons.push(if r.below(2) == 0 { Con::Prefix(hidden[..len].to_vec()) } else { Con::Suffix(hidden[n - len..].to_vec()) });
        }
        r.shuffle(&mut cons);
        s.big(n, ab, &cons);
        // corrupt the last letter of the last word
        if let Some(Con::Prefix(w)) | Some(Con::Suffix(w)) = cons.iter_mut().rev().find(|c| !matches!(c, Con::Border(_))) {
            let j = w.len() - 1;
            w[j] = if w[j] == b'a' { b'b' } else { b'a' };
        }
        s.big(n, ab, &cons);
    }
    s.done()
}

// E. almost everything free
fn section_e(seed: u64) -> usize {
    let mut s = Sec::new("E maximum number of free choices", seed);
    let all94: Vec<u8> = (33u8..127).collect();
    for &n in &[0usize, 1, 2, 127, 128, 129, 1000, 1_000_000, 5_000_000] {
        for al in [&b"a"[..], &b"ab"[..], &all94[..]] {
            for cons in [vec![], vec![Con::Border(0), Con::Border(n)], vec![Con::Prefix(vec![]), Con::Suffix(vec![])]] {
                if let Some(i) = s.big(n, al, &cons) {
                    s.expect(i.sat && i.free == n, "with no real rules every character must be free", n, al, &cons);
                    s.expect(count(al.len(), i.free) == pow_exact(al.len(), i.free), "solution count wrong", n, al, &cons);
                    s.expect(is_unique(al.len(), i.free) == (n == 0 || al.len() == 1), "uniqueness flag wrong", n, al, &cons);
                }
            }
            if n >= 2 {
                // first character = last character: one choice fewer
                let cons = vec![Con::Border(1)];
                if let Some(i) = s.big(n, al, &cons) {
                    s.expect(i.free == n - 1, "Border(1) must remove exactly one choice", n, al, &cons);
                }
                // one fixed letter in a huge free text
                let cons = vec![Con::Suffix(vec![al[al.len() - 1]])];
                if let Some(i) = s.big(n, al, &cons) {
                    s.expect(i.free == n - 1, "one fixed letter must remove exactly one choice", n, al, &cons);
                }
            }
        }
    }
    // The count function against exact arithmetic written here, including
    // the exact point where the count stops fitting in 128 bits.
    let mut bad = 0;
    for k in 1..=94usize {
        for free in 0..=260usize {
            if count(k, free) != pow_exact(k, free) {
                bad += 1;
            }
        }
    }
    s.expect(bad == 0, "count() disagrees with exact arithmetic", 0, b"a", &[]);
    s.expect(count(2, 127) == Some(1u128 << 127) && count(2, 128).is_none(), "128-bit boundary of the count", 0, b"ab", &[]);
    s.expect(count(1, usize::MAX) == Some(1), "one-letter alphabet with a huge number of free groups", 0, b"a", &[]);
    s.expect(count(2, usize::MAX).is_none() && count(94, u32::MAX as usize + 5).is_none(), "huge exponents must not wrap around", 0, b"ab", &[]);
    s.done()
}

// F. heavily interconnected rules
fn section_f(seed: u64) -> usize {
    let mut s = Sec::new("F highly interconnected constraints", seed);
    let mut r = X(seed.wrapping_mul(0x94D049BB133111EB) | 1);
    let ab = b"ab";
    // Every set of borders on short texts, alone and with every pair of
    // one-letter words (exhaustive).
    for n in 0..=9usize {
        for mask in 0u32..(1 << (n + 1)) {
            let borders: Vec<Con> = (0..=n).filter(|k| mask >> k & 1 == 1).map(Con::Border).collect();
            s.small(n, ab, &borders);
            if n >= 1 {
                for x in ab {
                    for y in ab {
                        let mut cons = borders.clone();
                        cons.push(Con::Prefix(vec![*x]));
                        cons.push(Con::Suffix(vec![*y]));
                        s.small(n, ab, &cons);
                    }
                }
            }
        }
    }
    // Random small puzzles against brute force: many borders, several words.
    for _ in 0..60_000 {
        let n = r.below(13);
        let al = &b"abc"[..1 + r.below(if n > 9 { 2 } else { 3 })];
        let mut cons: Vec<Con> = (0..r.below(9)).map(|_| Con::Border(r.below(n + 1))).collect();
        for _ in 0..r.below(4) {
            let len = r.below(n + 1);
            // mostly one letter so that puzzles are often solvable
            let base = al[r.below(al.len())];
            let w: Vec<u8> = (0..len).map(|_| if r.below(5) == 0 { al[r.below(al.len())] } else { base }).collect();
            cons.push(if r.below(2) == 0 { Con::Prefix(w) } else { Con::Suffix(w) });
        }
        r.shuffle(&mut cons);
        s.small(n, al, &cons);
    }
    // Groups that only join up once several borders are combined:
    // periods 6, 10 and 15 share no common factor, but each pair does.
    for n in 16..=80usize {
        for set in [&[6usize, 10, 15][..], &[6, 10], &[10, 15], &[6, 15], &[4, 6, 9], &[12, 18, 27], &[8, 12, 18, 27]] {
            let cons: Vec<Con> = set.iter().filter(|p| **p < n).map(|p| Con::Border(n - p)).collect();
            s.big(n, ab, &cons);
            let mut c2 = cons.clone();
            c2.push(Con::Prefix(b"ab".to_vec()));
            c2.push(Con::Suffix(b"ba".to_vec()));
            s.big(n, ab, &c2);
        }
    }
    // Medium random: up to 24 borders built from a few common steps (cycles,
    // several routes between the same positions), words cut from a planted
    // text, then one letter corrupted.
    for trial in 0..6000 {
        let n = if trial < 5000 { 2 + r.below(600) } else { 600 + r.below(30_000) };
        let k = [2usize, 2, 3, 26][r.below(4)];
        let al: Vec<u8> = (b'a'..=b'z').take(k).collect();
        let g = 1 + r.below(1 + n / 6);
        let mut cons: Vec<Con> = Vec::new();
        for _ in 0..r.below(25) {
            let p = match r.below(5) {
                0 => 1 + r.below(n - 1),
                1 | 2 => (g * (1 + r.below(9))).min(n - 1),
                3 => n - 1 - r.below(1 + n / 8).min(n - 2),
                _ => (n / 2 + r.below(5)).min(n - 1).max(1),
            };
            cons.push(Con::Border(n - p));
        }
        let (comp, ncomp) = components(n, &periods_of(n, &cons));
        let letters: Vec<u8> = (0..ncomp).map(|_| al[r.below(k)]).collect();
        let hidden: Vec<u8> = comp.iter().map(|c| letters[*c as usize]).collect();
        for _ in 0..r.below(4) {
            let len = r.below(n + 1);
            cons.push(if r.below(2) == 0 { Con::Prefix(hidden[..len].to_vec()) } else { Con::Suffix(hidden[n - len..].to_vec()) });
        }
        r.shuffle(&mut cons);
        if let Some(i) = s.big(n, &al, &cons) {
            s.expect(i.sat, "planted puzzle must be solvable", n, &al, &cons);
        }
        if let Some(Con::Prefix(w)) | Some(Con::Suffix(w)) = cons.iter_mut().find(|c| match c { Con::Border(_) => false, Con::Prefix(w) | Con::Suffix(w) => !w.is_empty() }) {
            let j = r.below(w.len());
            w[j] = al[(al.iter().position(|a| *a == w[j]).unwrap() + 1) % k];
        }
        s.big(n, &al, &cons);
    }
    s.done()
}

// G. boundary and malformed inputs (library level; the command line is
// exercised separately)
fn section_g(seed: u64) -> usize {
    let mut s = Sec::new("G boundary and malformed inputs", seed);
    let ab = b"ab";
    let all94: Vec<u8> = (33u8..127).collect();
    let none: [Con; 0] = [];
    // must be accepted
    s.small(0, ab, &none);
    s.small(0, ab, &[Con::Prefix(vec![]), Con::Suffix(vec![]), Con::Border(0)]);
    s.small(1, ab, &none);
    s.small(1, ab, &[Con::Border(0), Con::Border(1)]);
    s.small(1, ab, &[Con::Prefix(b"a".to_vec()), Con::Suffix(b"a".to_vec())]);
    s.small(1, ab, &[Con::Prefix(b"a".to_vec()), Con::Suffix(b"b".to_vec())]);
    s.small(1, b"a", &[Con::Border(1)]);
    s.small(3, b"a", &[Con::Border(2), Con::Prefix(b"aaa".to_vec())]);
    s.small(4, ab, &[Con::Prefix(b"abab".to_vec()), Con::Suffix(b"baba".to_vec())]);
    s.small(4, ab, &[Con::Prefix(b"ab".to_vec()), Con::Suffix(b"ab".to_vec()), Con::Prefix(b"abb".to_vec())]);
    s.big(94, &all94, &[Con::Prefix(all94.clone())]);
    s.big(94, &all94, &[Con::Suffix(all94.clone()), Con::Border(1)]);
    s.big(188, &all94, &[Con::Suffix(all94.clone()), Con::Border(94)]);
    s.small(2, &all94[..3], &none);
    // the same rules a thousand times over
    let mut many: Vec<Con> = Vec::new();
    for _ in 0..1000 {
        many.push(Con::Border(5));
        many.push(Con::Prefix(b"ab".to_vec()));
        many.push(Con::Suffix(b"b".to_vec()));
    }
    let once = [Con::Border(5), Con::Prefix(b"ab".to_vec()), Con::Suffix(b"b".to_vec())];
    s.small(8, ab, &many);
    s.expect(solve(8, ab, &many).ok() == solve(8, ab, &once).ok(), "repeating rules changed the answer", 8, ab, &many);
    // must be rejected, with an error and without crashing
    let rejects: Vec<(&str, usize, Vec<u8>, Vec<Con>)> = vec![
        ("empty alphabet", 3, vec![], vec![]),
        ("space in alphabet", 3, b"a b".to_vec(), vec![]),
        ("control character in alphabet", 3, vec![b'a', 0x7f], vec![]),
        ("NUL in alphabet", 3, vec![0, b'a'], vec![]),
        ("non-ASCII byte in alphabet", 3, vec![b'a', 0xC3, 0xA9], vec![]),
        ("repeated letter", 3, b"aba".to_vec(), vec![]),
        ("95 letters (one repeated)", 3, { let mut v = all94.clone(); v.push(b'!'); v }, vec![]),
        ("prefix longer than text", 3, ab.to_vec(), vec![Con::Prefix(b"abab".to_vec())]),
        ("suffix longer than text", 0, ab.to_vec(), vec![Con::Suffix(b"a".to_vec())]),
        ("prefix letter outside alphabet", 3, ab.to_vec(), vec![Con::Prefix(b"ac".to_vec())]),
        ("suffix NUL letter", 3, ab.to_vec(), vec![Con::Suffix(vec![0])]),
        ("border n+1", 3, ab.to_vec(), vec![Con::Border(4)]),
        ("border on empty text", 0, ab.to_vec(), vec![Con::Border(1)]),
        ("border usize::MAX", 3, ab.to_vec(), vec![Con::Border(usize::MAX)]),
        ("border usize::MAX on huge n", usize::MAX - 1, ab.to_vec(), vec![Con::Border(usize::MAX)]),
        ("bad rule after good ones", 5, ab.to_vec(), vec![Con::Border(2), Con::Prefix(b"a".to_vec()), Con::Border(6)]),
    ];
    for (what, n, al, cons) in rejects {
        let r = std::panic::catch_unwind(|| solve(n, &al, &cons).is_err());
        s.expect(matches!(r, Ok(true)), &format!("should be rejected cleanly: {what}"), n, &al, &cons);
    }
    // the built-in certificate checker must refuse nonsense without crashing
    let cons = [Con::Border(4), Con::Prefix(b"a".to_vec()), Con::Suffix(b"b".to_vec())];
    if let Ok(Outcome::Unsat(good)) = solve(7, ab, &cons) {
        s.expect(cert_ok(&good, 7, &cons).is_ok() && check_conflict(&good, 7, &cons), "good certificate refused", 7, ab, &cons);
        let mut forged: Vec<Conflict> = Vec::new();
        let mut c = good.clone(); c.path[1] = 4; forged.push(c);
        let mut c = good.clone(); c.path.pop(); forged.push(c);
        let mut c = good.clone(); c.via.pop(); forged.push(c);
        let mut c = good.clone(); c.via[0] = 1; forged.push(c);
        let mut c = good.clone(); c.via[0] = usize::MAX; forged.push(c);
        let mut c = good.clone(); c.src_a = 99; forged.push(c);
        let mut c = good.clone(); c.src_b = 0; forged.push(c);
        let mut c = good.clone(); c.letter_b = b'a'; forged.push(c);
        let mut c = good.clone(); c.path[2] = usize::MAX; forged.push(c);
        let mut c = good.clone(); c.path = vec![]; c.via = vec![]; forged.push(c);
        let mut c = good.clone(); c.path = vec![0]; c.via = vec![]; forged.push(c);
        let mut c = good.clone(); c.path.reverse(); forged.push(c);
        for f in forged {
            let cons2 = cons.clone();
            let f2 = f.clone();
            let r = std::panic::catch_unwind(move || check_conflict(&f2, 7, &cons2));
            s.expect(matches!(r, Ok(false)) && cert_ok(&f, 7, &cons).is_err(), "forged certificate accepted (or checker crashed)", 7, ab, &cons);
        }
    }
    s.done()
}

// ------------------------------------------------------------------ perf

fn peak_rss_mb() -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for l in s.lines() {
        if l.starts_with("VmHWM:") {
            return l.split_whitespace().nth(1).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0) / 1024.0;
        }
    }
    0.0
}

/// (alphabet, rules, contradictory?). For contradictory shapes the LAST rule
/// is the one that causes the clash, so dropping it gives a solvable twin.
fn shape(name: &str, n: usize) -> (Vec<u8>, Vec<Con>, bool) {
    let ab = b"ab".to_vec();
    let a = || Con::Prefix(b"a".to_vec());
    let b = || Con::Suffix(b"b".to_vec());
    let mut r = X(0x9E3779B97F4A7C15 ^ n as u64);
    let h = n / 2;
    match name {
        // solvable
        "free" => (ab, vec![], false),
        "free94" => ((33u8..127).collect(), vec![], false),
        "half" => (ab, vec![Con::Border(h)], false),
        "chain" => (ab, vec![Con::Border(n - 1), a()], false),
        "p2p3" => (ab, vec![Con::Border(n - 2), Con::Border(n - 3)], false),
        "rand16" => (ab, (0..16).map(|_| Con::Border(1 + r.below(n - 1))).collect(), false),
        "rand1000" => (ab, (0..1000).map(|_| Con::Border(1 + r.below(n - 1))).collect(), false),
        "fwpath" => (ab, vec![Con::Border(n - h), Con::Border(n - h - 1)], false),
        "tiny1000" => (ab, (1..=1000.min(n - 1)).map(Con::Border).collect(), false),
        "allborders" => (ab, (1..n).map(Con::Border).collect(), false),
        // every even period from n/2 upward: two groups (odd/even positions) for ever
        "evenhalf" => (ab, (h..n).filter(|q| q % 2 == 0).map(|q| Con::Border(n - q)).collect(), false),
        // the text is a block repeated twice, and the block repeats every 3
        "ww3" => {
            let hh = if h % 3 == 0 { h - 1 } else { h };
            (ab, (0..hh / 3).map(|j| Con::Border(hh - 3 * j)).chain([Con::Border(n - hh)]).collect(), false)
        }
        // contradictory
        "chain_unsat" => (ab, vec![Con::Border(n - 1), a(), b()], true),
        "fwpath_unsat" => {
            // n = p + q - 1 with p = h, q = n + 1 - h (consecutive when n is even)
            let (p, q) = (h, n + 1 - h);
            let mut tail = vec![b'a'; p];
            tail[0] = b'b';
            (ab, vec![Con::Suffix(vec![b'a'; q]), Con::Border(n - p), Con::Border(n - q), Con::Suffix(tail)], true)
        }
        "allborders_unsat" => (ab, (1..n).map(Con::Border).chain([a(), b()]).collect(), true),
        "allborders_mid_unsat" => {
            let mut w = vec![b'a'; h + 1];
            w[h] = b'b';
            (ab, (1..n).rev().map(Con::Border).chain([Con::Prefix(w)]).collect(), true)
        }
        // neighbours linked, plus 3000 short borders, clash in the middle
        "chain_tiny3000_unsat" => {
            let mut w = vec![b'a'; h + 1];
            w[h] = b'b';
            (ab, (1..=3000.min(n / 4)).map(Con::Border).chain([Con::Border(n - 1), Con::Prefix(w)]).collect(), true)
        }
        // Periods 4, 8, 12, ... and then 2. The even positions form one
        // group; position n - 2 is two links from position 0, but the search
        // has to finish with ~n/4 positions of ~n/4 links each before it
        // reaches the one that leads there.
        "mult4_2_unsat" => {
            let n4 = n - n % 4;
            assert!(n4 == n, "use a length divisible by 4");
            (ab, (1..n / 4).map(|j| Con::Border(n - 4 * j)).chain([Con::Border(n - 2), a(), Con::Suffix(b"ba".to_vec())]).collect(), true)
        }
        "rand16_unsat" => (ab, (0..16).map(|_| Con::Border(1 + r.below(n - 1))).chain([a(), b()]).collect(), true),
        _ => {
            eprintln!("unknown shape {name}");
            std::process::exit(2);
        }
    }
}

fn perf(name: &str, n: usize) {
    let (al, cons, unsat) = shape(name, n);
    let distinct = periods_of(n, &cons).len();
    let links: u128 = periods_of(n, &cons).iter().map(|p| (n - p) as u128).sum();
    let t = Instant::now();
    let out = solve(n, &al, &cons).unwrap();
    let secs = t.elapsed().as_secs_f64();
    let rss = peak_rss_mb();
    let (status, free, cert) = match &out {
        Outcome::Sat { free, .. } => ("solvable", *free, 0),
        Outcome::Unsat(cf) => ("contradiction", 0, cf.path.len()),
    };
    if unsat != matches!(out, Outcome::Unsat(_)) {
        println!("FAIL {name} n={n}: expected contradiction={unsat}");
    }
    // Time of the explanation alone: the same puzzle without its last rule
    // is solvable, so the difference is the cost of noticing and explaining.
    let proof = if unsat {
        let t = Instant::now();
        let twin = solve(n, &al, &cons[..cons.len() - 1]).unwrap();
        let base = t.elapsed().as_secs_f64();
        drop(twin);
        format!("{:.4}", (secs - base).max(0.0))
    } else {
        "-".into()
    };
    let verified = match &out {
        Outcome::Unsat(cf) => cert_ok(cf, n, &cons).is_ok(),
        Outcome::Sat { pattern, free } => {
            let w: Vec<u8> = pattern.iter().map(|c| match c { Cell::Letter(ch) => *ch, Cell::Free(i) => al[*i % al.len()] }).collect();
            let _ = free;
            // direct check of one non-trivial filling, when affordable
            links > 4_000_000_000 || holds(&w, &cons)
        }
    };
    // groups of equal positions, by an independent method, when affordable
    let groups = if (n as u128) * (distinct as u128) <= 600_000_000 {
        components(n, &periods_of(n, &cons)).1.to_string()
    } else {
        "-".into()
    };
    println!(
        "{name}\tn={n}\tborders={distinct}\tlinks={links}\t{status}\tgroups={groups}\tfree={free}\tcert={cert}\ttime={secs:.4}s\tproof={proof}s\tpeakMB={rss:.0}\tverified={verified}"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("perf") {
        if args.len() < 3 {
            eprintln!("usage: perf SHAPE N");
            std::process::exit(2);
        }
        perf(&args[1], args[2].parse().expect("N must be a whole number"));
        return;
    }
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let only: Option<&str> = args.get(2).map(|s| s.as_str());
    println!("adversarial checks, seed {seed}");
    let sections: [(&str, fn(u64) -> usize); 7] = [
        ("A", section_a), ("B", section_b), ("C", section_c), ("D", section_d),
        ("E", section_e), ("F", section_f), ("G", section_g),
    ];
    let mut fails = 0;
    for (tag, f) in sections {
        if only.map_or(true, |o| o.contains(tag)) {
            fails += f(seed);
        }
    }
    println!("total failures: {fails}");
    std::process::exit(if fails == 0 { 0 } else { 1 });
}
