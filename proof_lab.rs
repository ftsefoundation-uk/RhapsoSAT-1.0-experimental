#!/bin/sh
//bin/true 2>/dev/null; d="$(dirname "$0")"; t="${TMPDIR:-/tmp}/prefsuf_lab"; for f in prefsuf.rs adversarial.rs; do [ -f "$d/$f" ] || { echo "put $f in the same folder as $0"; exit 1; }; done; mkdir -p "$t" && tail -n +3 "$d/prefsuf.rs" | sed 's/^fn main() {/fn orig_main() {/' > "$t/core.rs" && tail -n +3 "$d/adversarial.rs" | sed 's/^fn main() {/fn adv_main() {/; /^#!\[allow(dead_code)\]$/d' > "$t/adv.rs" && tail -n +3 "$0" > "$t/main.rs" && rustc --edition 2021 -O "$t/main.rs" -o "$t/run" && exec "$t/run" "$@"; exit $?
// Proof-construction laboratory for prefsuf.rs. An experiment, kept apart
// from the solver: nothing in prefsuf.rs depends on this file.
// Keep prefsuf.rs and adversarial.rs in the same folder.
//
//     sh proof_lab.rs trace SHAPE N     where the time of a contradictory puzzle goes,
//                                       with counts of the work done by the search
//     sh proof_lab.rs derive SHAPE N    build the explanation by derivation instead
//                                       of search; validate it independently
//     sh proof_lab.rs check [SEED]      derived explanations on many puzzles, each
//                                       validated by the independent checker
//
// SHAPE is any contradictory shape of `adversarial.rs perf`.
//
// The solver's own code (core.rs) is used for the answer and for the
// baseline search. The validity of every explanation is decided by
// `cert_ok` from adversarial.rs, which is written from the rule definitions
// and shares no code with the solver or with the derivation below.
#![allow(dead_code)]
include!("adv.rs");

// ------------------------------------------- baseline search, instrumented
// A copy of bfs_path from prefsuf.rs with counters added. Same order of
// work, same result; used only to count.

#[derive(Default, Debug)]
struct SearchCount {
    expanded: u64,     // positions taken from the queue
    link_tests: u64,   // (position, period) pairs looked at
    discovered: u64,   // positions reached
    sorted_lists: u64, // positions for which a sub-list of periods was sorted
}

fn bfs_counted(n: usize, periods: &[(usize, usize)], src: usize, dst: usize, c: &mut SearchCount) -> (Vec<usize>, Vec<usize>) {
    if src == dst {
        return (vec![src], Vec::new());
    }
    let mut by_size = periods.to_vec();
    by_size.sort_unstable();
    let mut some: Vec<(usize, usize)> = Vec::new();
    let mut prev = vec![NONE; n];
    let mut prev_via = vec![NONE; n];
    let mut seen = vec![false; n];
    seen[src] = true;
    let mut queue = VecDeque::from([src]);
    'search: while let Some(u) = queue.pop_front() {
        c.expanded += 1;
        let reach = u.max(n - 1 - u);
        let usable = by_size.partition_point(|x| x.0 <= reach);
        let sort_cost = usable * (usize::BITS - usable.leading_zeros()) as usize;
        let list: &[(usize, usize)] = if sort_cost >= periods.len() {
            periods
        } else {
            c.sorted_lists += 1;
            some.clear();
            some.extend_from_slice(&by_size[..usable]);
            some.sort_unstable_by_key(|x| x.1);
            &some
        };
        c.link_tests += list.len() as u64;
        for &(p, idx) in list {
            for v in [u.checked_sub(p), Some(u + p).filter(|v| *v < n)].into_iter().flatten() {
                if !seen[v] {
                    seen[v] = true;
                    c.discovered += 1;
                    prev[v] = u;
                    prev_via[v] = idx;
                    if v == dst {
                        break 'search;
                    }
                    queue.push_back(v);
                }
            }
        }
    }
    let mut path = vec![dst];
    let mut via = Vec::new();
    let mut at = dst;
    while prev[at] != NONE {
        via.push(prev_via[at]);
        at = prev[at];
        path.push(at);
    }
    path.reverse();
    via.reverse();
    (path, via)
}

// --------------------------------------------------- proof by derivation
//
// The grouping step of the solver works in rounds. A round looks at the
// first `len` characters, takes their longest border k, and links every
// position j in the last part to position j - p, where p = len - k. Then it
// carries on with the first k characters. Consecutive rounds with the same
// p are one PHASE.
//
// A link made in the first phase is a link of an original Border rule. A
// link made in a later phase is not: it is a consequence of several rules.
// This file records, for each phase, WHY its links hold, and can turn any
// such link into a chain of original links on demand:
//
//   LabPhase r looks at the first `start` characters and uses border b, so
//   p = start - b. Write M_r(y) for the claim "position y = position y + p"
//   (it holds whenever y + p < start).
//
//   (a) b is a Border rule of the puzzle. Then y = y + (n - b) is an
//       original link. And y + p reaches the same position y + (n - b) by
//       stepping through the earlier phases, last one first: each earlier
//       phase q says "z = z + p_q", count_q times in a row.
//
//   (b) b arose at the end of an earlier phase q (the first part of the
//       text was found to repeat with shift p_q). Then y = y + p_q is the
//       claim M_q(y), and y + p reaches y + p_q by stepping through the
//       phases between q and r, last one first.
//
// Both cases only refer to earlier phases, so the recursion ends at
// original links. To keep the work down, a union-find structure remembers
// which positions are already joined by the original links collected so
// far; a claim whose two positions are already joined costs nothing more.
// Every original link that is kept joins two groups that were separate, so
// the kept links form a forest with fewer than n links, and the
// explanation is the unique path through that forest: never longer than
// the number of positions in the group.

#[derive(Clone, Copy, Debug)]
enum LabOrigin {
    Rule(usize),    // constraint index of a Border rule
    Derived(usize), // index of the phase at whose end this border arose
}

#[derive(Clone, Copy, Debug)]
struct LabPhase {
    start: usize, // text length looked at when the phase began
    p: usize,     // shift
    count: usize, // number of rounds
    origin: LabOrigin,
}

#[derive(Default, Debug)]
struct DeriveCount {
    phases: u64,
    claims: u64,       // calls asking for "y = y + p" in some phase
    claims_open: u64,  // ... that were not already known
    walks: u64,        // steps taken through whole phases
    walk_rounds: u64,  // individual rounds stepped through
    links_kept: u64,   // original links kept (forest size)
    links_tried: u64,  // original links looked at
    max_depth: u64,
}

struct LabDerivation {
    n: usize,
    phases: Vec<LabPhase>,
    link: Vec<u32>, // link[j] = position j was linked to, or j
    uf: Vec<u32>,
    kept: Vec<(u32, u32, u32)>, // (position, position, constraint index)
    count: DeriveCount,
    depth: u64,
}

impl LabDerivation {
    /// Re-run the grouping rounds of the solver, remembering the phases.
    fn new(n: usize, cons: &[Con]) -> LabDerivation {
        assert!(n < u32::MAX as usize, "experiment limited to texts under 4 billion characters");
        let mut given: Vec<(usize, usize)> = cons
            .iter()
            .enumerate()
            .filter_map(|(idx, c)| match c {
                Con::Border(k) if *k > 0 && *k < n => Some((*k, idx)),
                _ => None,
            })
            .collect();
        // longest first; for equal borders keep the earliest rule
        given.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        given.dedup_by_key(|x| x.0);
        let mut next = 0;
        let mut derived: BinaryHeap<(usize, usize)> = BinaryHeap::new(); // (border, phase)
        let mut link: Vec<u32> = (0..n as u32).collect();
        let mut phases: Vec<LabPhase> = Vec::new();
        let mut len = n;
        loop {
            let g = given.get(next).map_or(0, |x| x.0);
            let d = derived.peek().map_or(0, |x| x.0);
            if g == 0 && d == 0 {
                break;
            }
            // on a tie prefer the derived border: its justification is shorter
            let (mut k, origin) = if d >= g {
                let (k, q) = derived.pop().unwrap();
                (k, LabOrigin::Derived(q))
            } else {
                next += 1;
                (g, LabOrigin::Rule(given[next - 1].1))
            };
            if k >= len {
                continue;
            }
            let id = phases.len();
            let (start, p) = (len, len - k);
            let mut count = 0;
            loop {
                for j in p.max(k)..len {
                    link[j] = (j - p) as u32;
                }
                count += 1;
                len = k;
                if k <= p {
                    break;
                }
                let c = k - p;
                // borders as long as the text now looked at say nothing: drop them
                while given.get(next).map_or(false, |x| x.0 >= len) {
                    next += 1;
                }
                while derived.peek().map_or(false, |x| x.0 >= len) {
                    derived.pop();
                }
                let waiting = given.get(next).map_or(0, |x| x.0).max(derived.peek().map_or(0, |x| x.0));
                if c >= waiting {
                    k = c; // same shift again: the phase continues
                } else {
                    derived.push((c, id));
                    break;
                }
            }
            phases.push(LabPhase { start, p, count, origin });
        }
        let count = DeriveCount { phases: phases.len() as u64, ..DeriveCount::default() };
        LabDerivation { n, phases, link, uf: (0..n as u32).collect(), kept: Vec::new(), count, depth: 0 }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.uf[x] as usize != x {
            let g = self.uf[self.uf[x] as usize];
            self.uf[x] = g; // path halving
            x = g as usize;
        }
        x
    }

    fn joined(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    /// Look at one original link; keep it if it joins two separate groups.
    fn original(&mut self, a: usize, b: usize, idx: usize) {
        self.count.links_tried += 1;
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            // the smaller index becomes the root: cheap and good enough here
            if ra < rb { self.uf[rb] = ra as u32 } else { self.uf[ra] = rb as u32 }
            self.kept.push((a as u32, b as u32, idx as u32));
            self.count.links_kept += 1;
        }
    }

    /// Make sure "position y = position y + p_r" follows from kept links.
    fn claim(&mut self, r: usize, y: usize) {
        self.count.claims += 1;
        let LabPhase { start, p, origin, .. } = self.phases[r];
        debug_assert!(y + p < start);
        if self.joined(y, y + p) {
            return;
        }
        self.count.claims_open += 1;
        self.depth += 1;
        self.count.max_depth = self.count.max_depth.max(self.depth);
        let (target, from_phase) = match origin {
            LabOrigin::Rule(idx) => {
                let b = start - p;
                let far = y + (self.n - b);
                self.original(y, far, idx);
                (far, 0)
            }
            LabOrigin::Derived(q) => {
                self.claim(q, y);
                (y + self.phases[q].p, q + 1)
            }
        };
        // y + p must now be shown equal to `target` by stepping through the
        // phases from_phase .. r, last one first.
        let mut z = y + p;
        if !self.joined(z, target) {
            for q in (from_phase..r).rev() {
                z = self.walk(q, z);
            }
            debug_assert_eq!(z, target);
        }
        debug_assert!(self.joined(y, y + p));
        self.depth -= 1;
    }

    /// Step through every round of phase q, starting at position z.
    /// Works from the far end back towards z and stops as soon as z is
    /// known to be joined to the part already covered: when only the last
    /// few rounds are new, only those are looked at.
    fn walk(&mut self, q: usize, z: usize) -> usize {
        self.count.walks += 1;
        let LabPhase { p, count, .. } = self.phases[q];
        let end = z + count * p;
        let mut i = count;
        while i > 0 && !self.joined(z, z + i * p) {
            i -= 1;
            self.count.walk_rounds += 1;
            self.claim(q, z + i * p);
        }
        debug_assert!(self.joined(z, end));
        end
    }

    /// LabPhase in which position j was linked (j must not be its own link).
    fn phase_of(&self, j: usize) -> usize {
        // phases look at ever shorter texts; phase r linked positions in
        // [start_{r+1}, start_r)
        self.phases.partition_point(|ph| ph.start > j) - 1
    }

    /// Join position a to the smallest position of its group.
    fn anchor(&mut self, a: usize) -> usize {
        let mut j = a;
        while self.link[j] as usize != j {
            let r = self.phase_of(j);
            let to = self.link[j] as usize;
            self.claim(r, to);
            j = to;
        }
        j
    }

    /// A chain of original links from a to b (which must be in one group).
    fn path(&mut self, a: usize, b: usize) -> Option<(Vec<usize>, Vec<usize>)> {
        if self.anchor(a) != self.anchor(b) {
            return None;
        }
        if a == b {
            return Some((vec![a], Vec::new()));
        }
        // the kept links form a forest: find the one path from a to b
        let n = self.n;
        let mut deg = vec![0u32; n + 1];
        for &(u, v, _) in &self.kept {
            deg[u as usize + 1] += 1;
            deg[v as usize + 1] += 1;
        }
        for i in 0..n {
            deg[i + 1] += deg[i];
        }
        let mut fill = deg.clone();
        let mut adj = vec![(0u32, 0u32); self.kept.len() * 2];
        for &(u, v, idx) in &self.kept {
            adj[fill[u as usize] as usize] = (v, idx);
            fill[u as usize] += 1;
            adj[fill[v as usize] as usize] = (u, idx);
            fill[v as usize] += 1;
        }
        let mut prev = vec![u32::MAX; n];
        let mut via = vec![u32::MAX; n];
        prev[a] = a as u32;
        let mut stack = vec![a];
        while let Some(u) = stack.pop() {
            if u == b {
                break;
            }
            for e in deg[u] as usize..deg[u + 1] as usize {
                let (v, idx) = adj[e];
                if prev[v as usize] == u32::MAX {
                    prev[v as usize] = u as u32;
                    via[v as usize] = idx;
                    stack.push(v as usize);
                }
            }
        }
        if prev[b] == u32::MAX {
            return None;
        }
        let (mut path, mut vias, mut at) = (vec![b], Vec::new(), b);
        while at != a {
            vias.push(via[at] as usize);
            at = prev[at] as usize;
            path.push(at);
        }
        path.reverse();
        vias.reverse();
        Some((path, vias))
    }
}

/// Shorten a chain without searching the link graph: two positions are
/// directly linked exactly when their distance is a border period, which a
/// table answers at once. From each position jump to the farthest later
/// position of the chain that is directly linked. At most `budget` table
/// look-ups are spent; after that the rest of the chain is kept as it is.
/// The result is a valid chain, usually much shorter, not always shortest.
fn shorten(n: usize, cons: &[Con], path: &[usize], via: &[usize], budget: u64) -> (Vec<usize>, Vec<usize>, u64) {
    let mut rule_of = vec![u32::MAX; n]; // distance -> a Border rule with that period
    for (idx, c) in cons.iter().enumerate() {
        if let Con::Border(k) = c {
            if *k > 0 && *k < n && rule_of[n - *k] == u32::MAX {
                rule_of[n - *k] = idx as u32;
            }
        }
    }
    let (mut out, mut out_via, mut spent, mut i) = (vec![path[0]], Vec::new(), 0u64, 0usize);
    let last = path.len() - 1;
    while i < last {
        let mut jump = i + 1;
        let mut rule = via[i];
        let mut j = last;
        while j > i + 1 && spent < budget {
            spent += 1;
            let d = path[i].abs_diff(path[j]);
            if rule_of[d] != u32::MAX {
                jump = j;
                rule = rule_of[d] as usize;
                break;
            }
            j -= 1;
        }
        out.push(path[jump]);
        out_via.push(rule);
        i = jump;
    }
    (out, out_via, spent)
}

/// Replace the path of a solver-made certificate by a derived one.
fn derived_certificate(n: usize, cons: &[Con], cf: &Conflict) -> Option<(Conflict, DeriveCount)> {
    let mut d = LabDerivation::new(n, cons);
    let (path, via) = d.path(cf.path[0], cf.path[cf.path.len() - 1])?;
    Some((Conflict { path, via, ..cf.clone() }, d.count))
}

/// Run f on a thread with a large stack (the derivation recurses once per
/// phase in the worst case).
fn deep<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(1 << 30).spawn(f).unwrap().join().unwrap()
}

// ------------------------------------------------------------------ modes

fn trace(name: &str, n: usize) {
    let (al, cons, _) = shape(name, n);
    let t0 = Instant::now();
    validate(n, &al, &cons).unwrap();
    let t_validate = t0.elapsed().as_secs_f64();
    let t = Instant::now();
    let root = equal_positions(n, &cons).unwrap();
    let t_group = t.elapsed().as_secs_f64();
    // the letter scan of solve(), to find the clash
    let t = Instant::now();
    let mut letter = vec![0u8; n];
    let mut clash: Option<(usize, usize, usize)> = None; // (root, position, rule)
    'scan: for (idx, c) in cons.iter().enumerate() {
        let Some((word, offset)) = word_of(n, c) else { continue };
        for (j, &ch) in word.iter().enumerate() {
            let r = root[offset + j];
            if letter[r] == 0 {
                letter[r] = ch;
            } else if letter[r] != ch {
                clash = Some((r, offset + j, idx));
                break 'scan;
            }
        }
    }
    let t_scan = t.elapsed().as_secs_f64();
    let Some((r, pos, _)) = clash else {
        println!("{name} n={n}: no contradiction, nothing to trace");
        return;
    };
    let t = Instant::now();
    let (pos0, _) = first_literal(n, &cons, &root, r);
    let periods = border_periods(n, &cons);
    let t_prep = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let mut c = SearchCount::default();
    let (path, _via) = bfs_counted(n, &periods, pos0, pos, &mut c);
    let t_search = t.elapsed().as_secs_f64();
    let group_size = root.iter().filter(|x| **x == r).count();
    let mut groups = root.clone();
    groups.sort_unstable();
    groups.dedup();
    let links: u128 = periods.iter().map(|x| (n - x.0) as u128).sum();
    println!("{name} n={n}");
    println!("  workload : {} distinct borders, {links} links in total, {} groups, clash group has {group_size} positions", periods.len(), groups.len());
    println!("  clash    : position {pos0} against position {pos}; shortest explanation has {} positions", path.len());
    println!("  time     : check input {t_validate:.4}s, grouping {t_group:.4}s, letter scan {t_scan:.4}s, prepare {t_prep:.4}s, SEARCH {t_search:.4}s");
    println!("  search   : {} positions expanded, {} discovered, {} (position, border) pairs tested, {} sorted sub-lists", c.expanded, c.discovered, c.link_tests, c.sorted_lists);
    println!("  peak memory {:.0} MB", peak_rss_mb());
}

fn derive(name: &str, n: usize) {
    let (_al, cons, _) = shape(name, n);
    let t = Instant::now();
    let root = equal_positions(n, &cons).unwrap();
    let mut letter = vec![0u8; n];
    let mut clash: Option<(usize, usize, usize, u8)> = None;
    'scan: for (idx, c) in cons.iter().enumerate() {
        let Some((word, offset)) = word_of(n, c) else { continue };
        for (j, &ch) in word.iter().enumerate() {
            let r = root[offset + j];
            if letter[r] == 0 {
                letter[r] = ch;
            } else if letter[r] != ch {
                clash = Some((r, offset + j, idx, ch));
                break 'scan;
            }
        }
    }
    let Some((r, pos, idx, ch)) = clash else {
        println!("{name} n={n}: no contradiction");
        return;
    };
    let (pos0, idx0) = first_literal(n, &cons, &root, r);
    let t_solve = t.elapsed().as_secs_f64();
    let stub = Conflict { path: vec![pos0, pos], via: vec![], letter_a: letter[r], src_a: idx0, letter_b: ch, src_b: idx };
    let cons2 = cons.clone();
    let t = Instant::now();
    let made = deep(move || derived_certificate(n, &cons2, &stub));
    let t_derive = t.elapsed().as_secs_f64();
    let rss = peak_rss_mb();
    let Some((cf, c)) = made else {
        println!("FAIL {name} n={n}: derivation found no chain");
        std::process::exit(1);
    };
    let t = Instant::now();
    let verdict = cert_ok(&cf, n, &cons);
    let t_check = t.elapsed().as_secs_f64();
    let simple = {
        let mut p = cf.path.clone();
        p.sort_unstable();
        p.windows(2).all(|w| w[0] != w[1])
    };
    println!("{name}\tn={n}\tsolve={t_solve:.4}s\tderive={t_derive:.4}s\tcheck={t_check:.4}s\tcert={}\tno_repeats={simple}\tindependent_checker={}\tpeakMB={rss:.0}", cf.path.len(), if verdict.is_ok() { "accepted".to_string() } else { format!("REJECTED {verdict:?}") });
    println!("\t{c:?}");
    if verdict.is_err() {
        std::process::exit(1);
    }
    let t = Instant::now();
    let (p2, v2, spent) = shorten(n, &cons, &cf.path, &cf.via, 8 * cf.path.len() as u64 + 64);
    let t_short = t.elapsed().as_secs_f64();
    let short = Conflict { path: p2, via: v2, ..cf.clone() };
    let v = cert_ok(&short, n, &cons);
    println!("\tshortened: cert={}\ttime={t_short:.4}s\tlookups={spent}\tindependent_checker={}", short.path.len(), if v.is_ok() { "accepted".to_string() } else { format!("REJECTED {v:?}") });
    if v.is_err() {
        std::process::exit(1);
    }
}

#[derive(Default)]
struct LabTally {
    puzzles: u64,
    unsat: u64,
    accepted: u64,
    rejected: u64,
    missing: u64,
    longer: u64,
    sum_short: u64,
    sum_derived: u64,
    worst_ratio: f64,
    sum_shortened: u64,
    solver_fallback_ok: u64,
    solver_fallback_links: u64,
    shortened_longer: u64,
    worst_short_ratio: f64,
    pairs: u64,
    pairs_bad: u64,
    max_n: usize,
    max_cert: usize,
    max_borders: usize,
}

/// Is `path` a chain of valid links? Local to this file (used for pairs of
/// positions that no rule fixes, where no full certificate exists). Real
/// contradictions go through cert_ok instead.
fn chain_ok(n: usize, cons: &[Con], path: &[usize], via: &[usize]) -> bool {
    path.len() == via.len() + 1
        && path.iter().all(|x| *x < n)
        && (0..via.len()).all(|i| {
            let (u, v) = (path[i].min(path[i + 1]), path[i].max(path[i + 1]));
            matches!(cons.get(via[i]), Some(Con::Border(k)) if *k <= n && u < *k && v == u + (n - *k))
        })
}

fn try_puzzle(t: &mut LabTally, seed: u64, n: usize, al: &[u8], cons: &[Con], r: &mut X) {
    t.puzzles += 1;
    t.max_n = t.max_n.max(n);
    t.max_borders = t.max_borders.max(periods_of(n, cons).len());
    let out = match solve(n, al, cons) {
        Ok(o) => o,
        Err(_) => return,
    };
    let report = |what: &str| println!("FAIL seed={seed}: {what}\n     reproduce: {}", cli(n, al, cons));
    // any two positions of one group must get a valid chain
    if n >= 2 {
        let mut d = LabDerivation::new(n, cons);
        let periods = periods_of(n, cons);
        let (comp, _) = components(n, &periods);
        for _ in 0..4 {
            let a = r.below(n);
            let b = r.below(n);
            t.pairs += 1;
            let same = comp[a] == comp[b];
            match d.path(a, b) {
                Some((path, via)) => {
                    if !same || path[0] != a || path[path.len() - 1] != b || !chain_ok(n, cons, &path, &via) {
                        t.pairs_bad += 1;
                        report(&format!("bad chain between positions {a} and {b}"));
                    }
                }
                None => {
                    if same {
                        t.pairs_bad += 1;
                        report(&format!("no chain between positions {a} and {b} of one group"));
                    }
                }
            }
        }
    }
    if let Outcome::Unsat(cf) = out {
        t.unsat += 1;
        // the solver's own fallback (search limit 0), judged by the independent checker
        match solve_with(n, al, cons, 0) {
            Ok(Outcome::Unsat(fb)) => {
                let mut seen = fb.path.clone();
                seen.sort_unstable();
                seen.dedup();
                let ends = fb.path[0] == cf.path[0] && fb.path.last() == cf.path.last();
                if cert_ok(&fb, n, cons).is_ok() && ends && seen.len() == fb.path.len() && fb.via.len() >= cf.via.len() {
                    t.solver_fallback_ok += 1;
                    t.solver_fallback_links += fb.via.len() as u64;
                } else {
                    t.rejected += 1;
                    report("the solver's fallback explanation was rejected by the independent checker");
                }
            }
            _ => {
                t.rejected += 1;
                report("with search limit 0 the solver changed its answer");
            }
        }
        match derived_certificate(n, cons, &cf) {
            None => {
                t.missing += 1;
                report("derivation found no chain for a real contradiction");
            }
            Some((mine, _)) => match cert_ok(&mine, n, cons) {
                Ok(()) => {
                    let (p2, v2, _) = shorten(n, cons, &mine.path, &mine.via, 8 * mine.path.len() as u64 + 64);
                    let short = Conflict { path: p2, via: v2, ..mine.clone() };
                    if cert_ok(&short, n, cons).is_err() || short.via.len() < cf.via.len() {
                        t.rejected += 1;
                        report("shortened certificate rejected by the independent checker (or shorter than shortest)");
                    }
                    t.sum_shortened += short.via.len() as u64;
                    if short.via.len() > cf.via.len() {
                        t.shortened_longer += 1;
                        t.worst_short_ratio = t.worst_short_ratio.max(short.via.len() as f64 / cf.via.len().max(1) as f64);
                    }
                    t.accepted += 1;
                    t.max_cert = t.max_cert.max(mine.path.len());
                    t.sum_short += cf.via.len() as u64;
                    t.sum_derived += mine.via.len() as u64;
                    if mine.via.len() > cf.via.len() {
                        t.longer += 1;
                        t.worst_ratio = t.worst_ratio.max(mine.via.len() as f64 / cf.via.len().max(1) as f64);
                    }
                    if mine.via.len() < cf.via.len() {
                        report("derived chain is shorter than the 'shortest' one");
                        t.rejected += 1;
                    }
                }
                Err(e) => {
                    t.rejected += 1;
                    report(&format!("independent checker rejected the derived certificate: {e}"));
                }
            },
        }
    }
}

fn check(seed: u64) {
    let mut r = X(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut t = LabTally::default();
    let ab = b"ab";
    // 1. every set of borders on short texts with a letter at each end
    for n in 1..=10usize {
        for mask in 0u32..(1 << (n + 1)) {
            let mut cons: Vec<Con> = (0..=n).filter(|k| mask >> k & 1 == 1).map(Con::Border).collect();
            cons.push(Con::Prefix(b"a".to_vec()));
            cons.push(Con::Suffix(b"b".to_vec()));
            try_puzzle(&mut t, seed, n, ab, &cons, &mut r);
        }
    }
    // 2. random puzzles, many borders, words that often clash
    for trial in 0..40_000 {
        let n = if trial < 30_000 { 2 + r.below(60) } else { 60 + r.below(3000) };
        let g = 1 + r.below(1 + n / 5);
        let mut cons: Vec<Con> = Vec::new();
        for _ in 0..r.below(if trial % 7 == 0 { 60 } else { 12 }) {
            let p = match r.below(6) {
                0 => 1 + r.below(n - 1),
                1 | 2 => (g * (1 + r.below(9))).min(n - 1),
                3 => n - 1 - r.below(1 + n / 8).min(n - 2),
                4 => 1 + r.below(1 + n / 10),
                _ => (n / 2 + r.below(5)).min(n - 1).max(1),
            };
            cons.push(Con::Border(n - p));
        }
        if r.below(8) == 0 {
            cons.push(Con::Border(0));
            cons.push(Con::Border(n));
        }
        for _ in 0..1 + r.below(3) {
            let cap = if r.below(3) == 0 { n } else { 6 };
            let len = 1 + r.below(n.min(cap));
            let base = ab[r.below(2)];
            let w: Vec<u8> = (0..len).map(|_| if r.below(4) == 0 { ab[r.below(2)] } else { base }).collect();
            cons.push(if r.below(2) == 0 { Con::Prefix(w) } else { Con::Suffix(w) });
        }
        r.shuffle(&mut cons);
        try_puzzle(&mut t, seed, n, ab, &cons, &mut r);
    }
    // 3. the families of adversarial section C, each with a clash
    for &n in &[7usize, 50, 501, 3000] {
        let all: Vec<usize> = (0..=n).collect();
        let fib = |by_period: bool| {
            let (mut a, mut b, mut v) = (1usize, 2usize, Vec::new());
            while b < n {
                v.push(if by_period { n - b } else { b });
                (a, b) = (b, a + b);
            }
            v
        };
        let families: Vec<Vec<usize>> = vec![
            all.clone(),
            all.iter().copied().filter(|k| (n - k) % 2 == 0).collect(),
            all.iter().copied().filter(|k| (n - k) % 3 == 0).collect(),
            all.iter().copied().filter(|k| *k <= n / 2).collect(),
            all.iter().copied().filter(|k| *k >= n / 2).collect(),
            all.iter().copied().filter(|_| r.below(2) == 0).collect(),
            all.iter().copied().filter(|k| (n - k) % 6 == 0 || (n - k) % 10 == 0).collect(),
            all.iter().copied().filter(|k| (n - k) % 2 == 0 && n - k >= n / 2).collect(),
            (0..=n / 6).map(|j| n / 2 - 3 * j).chain([n - n / 2]).collect(),
            fib(true),
            fib(false),
            all.iter().copied().filter(|_| r.below(10) == 0).collect(),
            (0..(n as f64).sqrt() as usize + 1).map(|_| r.below(n + 1)).collect(),
            all.iter().copied().filter(|k| (n - k) % 4 == 0 || n - k == 2).collect(),
        ];
        for ks in families {
            for order in 0..3 {
                let mut ks = ks.clone();
                match order {
                    0 => {}
                    1 => ks.reverse(),
                    _ => r.shuffle(&mut ks),
                }
                let borders: Vec<Con> = ks.iter().map(|k| Con::Border(*k)).collect();
                let mut cons = borders.clone();
                cons.push(Con::Prefix(b"a".to_vec()));
                cons.push(Con::Suffix(b"b".to_vec()));
                try_puzzle(&mut t, seed, n, ab, &cons, &mut r);
                let mut cons = borders.clone();
                cons.push(Con::Prefix(b"a".to_vec()));
                cons.push(Con::Suffix(b"ba".to_vec()));
                try_puzzle(&mut t, seed, n, ab, &cons, &mut r);
                let mut w = vec![b'a'; n / 2 + 1];
                w[n / 2] = b'b';
                let mut cons = borders.clone();
                cons.push(Con::Prefix(w));
                try_puzzle(&mut t, seed, n, ab, &cons, &mut r);
            }
        }
    }
    // 4. the contradictory perf shapes at a size where the shortest proof is affordable
    for name in ["chain_unsat", "fwpath_unsat", "allborders_unsat", "allborders_mid_unsat", "chain_tiny3000_unsat", "mult4_2_unsat", "rand16_unsat"] {
        for n in [1000usize, 20_000] {
            let (al, cons, _) = shape(name, n);
            try_puzzle(&mut t, seed, n, &al, &cons, &mut r);
        }
    }
    println!(
        "seed {seed}: {} puzzles ({} contradictory); derived certificates: {} accepted by the independent checker, {} rejected, {} missing",
        t.puzzles, t.unsat, t.accepted, t.rejected, t.missing
    );
    println!(
        "  length: {} of them longer than the shortest proof; total links {} derived vs {} shortest; worst ratio {:.1}x; longest {} positions",
        t.longer, t.sum_derived, t.sum_short, t.worst_ratio, t.max_cert
    );
    println!(
        "  after the shortening pass: {} still longer than the shortest proof; total links {}; worst ratio {:.1}x",
        t.shortened_longer, t.sum_shortened, t.worst_short_ratio
    );
    println!(
        "  solver's own fallback (prefsuf.rs with search limit 0): {} accepted by the independent checker, total links {}",
        t.solver_fallback_ok, t.solver_fallback_links
    );
    println!(
        "  chains between random pairs of positions: {} tried, {} wrong; longest text {}, most distinct borders {}",
        t.pairs, t.pairs_bad, t.max_n, t.max_borders
    );
    std::process::exit(if t.rejected + t.missing + t.pairs_bad == 0 && t.solver_fallback_ok == t.unsat { 0 } else { 1 });
}

/// Cost of deriving a chain for EVERY position (a full spanning forest of
/// original links), as work per character, over many border sets.
fn cost(seed: u64, n: usize) {
    let mut r = X(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let all: Vec<usize> = (1..n).collect();
    let fib = |by_period: bool| {
        let (mut a, mut b, mut v) = (1usize, 2usize, Vec::new());
        while b < n {
            v.push(if by_period { n - b } else { b });
            (a, b) = (b, a + b);
        }
        v
    };
    let g = 2 + r.below(40);
    let mut families: Vec<(String, Vec<usize>)> = vec![
        ("all borders".into(), all.clone()),
        ("even periods".into(), all.iter().copied().filter(|k| (n - k) % 2 == 0).collect()),
        ("short borders (k <= n/2)".into(), all.iter().copied().filter(|k| *k <= n / 2).collect()),
        ("long borders (k >= n/2)".into(), all.iter().copied().filter(|k| *k >= n / 2).collect()),
        ("random half".into(), all.iter().copied().filter(|_| r.below(2) == 0).collect()),
        ("random tenth".into(), all.iter().copied().filter(|_| r.below(10) == 0).collect()),
        ("random 1%".into(), all.iter().copied().filter(|_| r.below(100) == 0).collect()),
        ("periods 6j and 10j".into(), all.iter().copied().filter(|k| (n - k) % 6 == 0 || (n - k) % 10 == 0).collect()),
        ("even periods >= n/2".into(), all.iter().copied().filter(|k| (n - k) % 2 == 0 && n - k >= n / 2).collect()),
        ("block twice, block period 3".into(), (0..=n / 6).map(|j| n / 2 - 3 * j).filter(|k| *k > 0).chain([n - n / 2]).collect()),
        ("Fibonacci periods".into(), fib(true)),
        ("Fibonacci border lengths".into(), fib(false)),
        ("sqrt(n) random borders".into(), (0..(n as f64).sqrt() as usize + 1).map(|_| 1 + r.below(n - 1)).collect()),
        ("16 random borders".into(), (0..16).map(|_| 1 + r.below(n - 1)).collect()),
        ("periods 4j and 2".into(), all.iter().copied().filter(|k| (n - k) % 4 == 0 || n - k == 2).collect()),
        (format!("random multiples of {g} as periods, 10%"), all.iter().copied().filter(|k| (n - k) % g == 0 && r.below(10) == 0).collect()),
        (format!("random multiples of {g} as borders, 10%"), all.iter().copied().filter(|k| k % g == 0 && r.below(10) == 0).collect()),
        ("two coprime periods at the Fine-Wilf bound".into(), vec![n - n / 2, n - (n + 1 - n / 2)]),
        ("borders n/2, n/4, n/8, ...".into(), (1..40).map(|i| n >> i).filter(|k| *k > 0).collect()),
        ("borders n/2+1, n/4+1, ... (near halves)".into(), (1..40).map(|i| (n >> i) + 1).filter(|k| *k < n).collect()),
        ("periods n/3 +- 0..30".into(), (0..60).map(|i| n - (n / 3 + i) + 30).filter(|k| *k > 0 && *k < n).collect()),
        ("random borders near sqrt-spaced grid".into(), { let s = (n as f64).sqrt() as usize + 1; (1..s).map(|i| (i * s + r.below(3)).min(n - 1)).collect() }),
    ];
    for i in 0..12 {
        let m = 2 + r.below(2000.min(n / 2));
        let gg = 1 + r.below(1 + n / 50);
        let ks: Vec<usize> = (0..m).map(|_| match r.below(4) {
            0 => 1 + r.below(n - 1),
            1 => n - (gg * (1 + r.below(n / gg))).min(n - 1),
            2 => 1 + r.below(1 + n / 20),
            _ => n - 1 - r.below(1 + n / 20).min(n - 2),
        }).collect();
        families.push((format!("random mix #{i} ({m} borders)"), ks));
    }
    let mut worst = 0.0f64;
    println!("n={n} seed={seed}: work per character to derive a chain for every position");
    for (name, ks) in families {
        let cons: Vec<Con> = ks.iter().map(|k| Con::Border(*k)).collect();
        let t = Instant::now();
        let mut d = LabDerivation::new(n, &cons);
        // one claim per linked position (earlier positions are done first,
        // so this derives every link of the grouping exactly once)
        for j in 0..n {
            let to = d.link[j] as usize;
            if to != j {
                let ph = d.phase_of(j);
                d.claim(ph, to);
            }
        }
        let secs = t.elapsed().as_secs_f64();
        // the kept links must give exactly the groups of the solver
        let root = equal_positions(n, &cons).unwrap();
        let mut ok = true;
        for j in 0..n {
            ok &= d.find(j) == d.find(root[j]);
        }
        let mut groups = root.clone();
        groups.sort_unstable();
        groups.dedup();
        ok &= d.kept.len() == n - groups.len();
        let work = d.count.claims + d.count.walks + d.count.links_tried;
        let per = work as f64 / n as f64;
        worst = worst.max(per);
        println!(
            "  {:<46} borders={:<7} phases={:<6} depth={:<5} claims={:<10} walks={:<10} kept={:<8} work/char={:<7.2} {:.3}s {}",
            name, periods_of(n, &cons).len(), d.count.phases, d.count.max_depth, d.count.claims, d.count.walks, d.count.links_kept, per, secs,
            if ok { "groups match" } else { "GROUPS DIFFER" }
        );
        if !ok {
            std::process::exit(1);
        }
    }
    println!("  worst work per character: {worst:.2}");
}

/// Work done by the derivation, taking the worst of three ways of using
/// it: every link from the first position up, every link from the last
/// position down, and a single request for one chain (last position).
fn full_cost(n: usize, ks: &[usize]) -> (u64, u64, u64) {
    let cons: Vec<Con> = ks.iter().map(|k| Con::Border(*k)).collect();
    let mut worst = (0u64, 0u64, 0u64);
    for mode in 0..3 {
        let mut d = LabDerivation::new(n, &cons);
        match mode {
            0 => {
                for j in 0..n {
                    let to = d.link[j] as usize;
                    if to != j {
                        let ph = d.phase_of(j);
                        d.claim(ph, to);
                    }
                }
            }
            1 => {
                for j in (0..n).rev() {
                    let to = d.link[j] as usize;
                    if to != j {
                        let ph = d.phase_of(j);
                        d.claim(ph, to);
                    }
                }
            }
            _ => {
                d.anchor(n - 1);
            }
        }
        let w = d.count.claims + d.count.walks + d.count.links_tried;
        if w > worst.0 {
            worst = (w, d.count.phases, d.count.max_depth);
        }
    }
    worst
}

/// Search for border sets that make the derivation expensive: random
/// starts, then keep any single change that raises work per character.
fn hunt(seed: u64, n: usize, tries: usize) {
    let mut r = X(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut best: (f64, Vec<usize>, u64, u64) = (0.0, vec![], 0, 0);
    for _ in 0..tries {
        let m = 1 + r.below(8);
        let mut ks: Vec<usize> = (0..m).map(|_| 1 + r.below(n - 1)).collect();
        let (mut w, mut ph, mut dp) = full_cost(n, &ks);
        for _ in 0..200 {
            let mut cand = ks.clone();
            match r.below(4) {
                0 if cand.len() < 14 => cand.push(1 + r.below(n - 1)),
                1 if cand.len() > 1 => { let i = r.below(cand.len()); cand.remove(i); }
                2 => { let i = r.below(cand.len()); cand[i] = 1 + r.below(n - 1); }
                _ => { let i = r.below(cand.len()); let d = 1 + r.below(3); cand[i] = if r.below(2) == 0 { (cand[i] + d).min(n - 1) } else { cand[i].saturating_sub(d).max(1) }; }
            }
            let (w2, ph2, dp2) = full_cost(n, &cand);
            if w2 > w {
                (ks, w, ph, dp) = (cand, w2, ph2, dp2);
            }
        }
        let per = w as f64 / n as f64;
        if per > best.0 {
            ks.sort_unstable_by(|a, b| b.cmp(a));
            best = (per, ks, ph, dp);
        }
    }
    println!("n={n} seed={seed} tries={tries}: worst work/char found {:.2} (phases {}, depth {}) with borders {:?}", best.0, best.2, best.3, best.1);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("hunt") if args.len() == 4 => {
            let (seed, n, tries) = (args[1].parse().expect("SEED"), args[2].parse().expect("N"), args[3].parse().expect("TRIES"));
            deep(move || hunt(seed, n, tries))
        }
        Some("cost") if args.len() == 3 => {
            let (seed, n) = (args[1].parse().expect("SEED"), args[2].parse().expect("N"));
            deep(move || cost(seed, n))
        }
        Some("trace") if args.len() == 3 => trace(&args[1], args[2].parse().expect("N must be a whole number")),
        Some("derive") if args.len() == 3 => derive(&args[1], args[2].parse().expect("N must be a whole number")),
        Some("check") | None => {
            let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            deep(move || check(seed))
        }
        _ => {
            eprintln!("usage: trace SHAPE N | derive SHAPE N | check [SEED]");
            std::process::exit(2);
        }
    }
}
