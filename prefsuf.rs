#!/bin/sh
//bin/true 2>/dev/null; out="${TMPDIR:-/tmp}/prefsuf_bin"; rustc --edition 2021 -O "$0" -o "$out" && exec "$out" "$@"; exit $?
//
// prefsuf.rs -- prefix/suffix constraint propagation, single-file Rust port.
//
// Run (Termux or any Unix with rustc):
//     sh prefsuf.rs                  build, run all self-tests, run the demos
//     sh prefsuf.rs 8 ab B5 Pab Sb   build, solve one puzzle (plain-English report)
//     sh prefsuf.rs --help           all options (--full, --compact)
//
// The two lines at the top make this file a shell script that compiles itself
// with rustc and then runs the result. To rustc they are a shebang and a
// comment. No cargo, no crates, standard library only.
//
// Illustrative problem (not a solution to any real-world task):
//     Unknown string s of known length n over a known finite alphabet.
//     Constraints, each one of:
//         Prefix(w)  s starts with w                      CLI: Pw
//         Suffix(w)  s ends with w                        CLI: Sw
//         Border(k)  length-k prefix == length-k suffix   CLI: Bk
//     Output: UNSAT with a checkable certificate, or a pattern in which each
//     position is a fixed letter or the id of a free class of positions that
//     must be equal. Every filling of the free classes is a solution and
//     there are no others.
//
// The solver never enumerates candidate strings. Enumeration appears only in
// the self-tests, as an oracle on tiny instances.

use std::collections::{BTreeSet, BinaryHeap, VecDeque};
use std::process::exit;

// ------------------------------------------------------------------ types

#[derive(Clone, Debug, PartialEq)]
enum Con {
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    Border(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Letter(u8),
    Free(usize),
}

/// Certificate of unsatisfiability: path[0] is fixed to letter_a by constraint
/// src_a, the last position of path is fixed to letter_b != letter_a by
/// constraint src_b, and each consecutive pair in path is forced equal by the
/// Border constraint whose index is in via (via.len() == path.len() - 1).
#[derive(Clone, Debug, PartialEq)]
struct Conflict {
    path: Vec<usize>,
    via: Vec<usize>,
    letter_a: u8,
    src_a: usize,
    letter_b: u8,
    src_b: usize,
}

#[derive(Clone, Debug, PartialEq)]
enum Outcome {
    Unsat(Conflict),
    Sat { pattern: Vec<Cell>, free: usize },
}

// ------------------------------------------------------------- validation

fn validate(n: usize, alphabet: &[u8], cons: &[Con]) -> Result<(), String> {
    if alphabet.is_empty() {
        return Err("the list of allowed letters must not be empty".into());
    }
    // Lookup table instead of scanning the alphabet for every letter.
    let mut allowed = [false; 256];
    for a in alphabet {
        if !a.is_ascii_graphic() {
            return Err("allowed letters must be visible ASCII characters (no spaces, no accents)".into());
        }
        if allowed[*a as usize] {
            return Err(format!("allowed letters must be distinct ('{}' is listed twice)", *a as char));
        }
        allowed[*a as usize] = true;
    }
    for (idx, c) in cons.iter().enumerate() {
        let rule = idx + 1;
        match c {
            Con::Prefix(w) | Con::Suffix(w) => {
                if w.len() > n {
                    return Err(format!(
                        "rule {rule}: the word has {} characters but the text only has {n}",
                        w.len()
                    ));
                }
                if let Some(ch) = w.iter().find(|ch| !allowed[**ch as usize]) {
                    return Err(format!(
                        "rule {rule}: the word uses '{}', which is not one of the allowed letters",
                        if ch.is_ascii_graphic() { (*ch as char).to_string() } else { format!("byte 0x{:02X}", *ch) }
                    ));
                }
            }
            Con::Border(k) => {
                if *k > n {
                    return Err(format!(
                        "rule {rule}: cannot compare {k} characters when the text only has {n}"
                    ));
                }
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------- solver

const NONE: usize = usize::MAX;

/// A vector of n copies of value, or a readable error if n is too large
/// for this machine (instead of aborting the program).
fn vec_of<T: Clone>(n: usize, value: T) -> Result<Vec<T>, String> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| too_long(n))?;
    v.resize(n, value);
    Ok(v)
}

fn too_long(n: usize) -> String {
    format!("a text of {n} characters is too long to fit in this computer's memory")
}

/// The distinct shifts ("periods") demanded by the Border constraints, each
/// with the index of the first constraint that demands it, in constraint
/// order. Border(k) on a text of length n says s[i] == s[i + p] for every
/// 0 <= i < k, where p = n - k. Border(0) and Border(n) are always true and
/// are left out; a repeated Border adds nothing and is left out too.
fn border_periods(n: usize, cons: &[Con]) -> Vec<(usize, usize)> {
    let mut all: Vec<(usize, usize)> = cons
        .iter()
        .enumerate()
        .filter_map(|(idx, c)| match c {
            Con::Border(k) if *k > 0 && *k < n => Some((n - *k, idx)),
            _ => None,
        })
        .collect();
    all.sort_unstable(); // by period, then by constraint index
    all.dedup_by_key(|x| x.0); // keeps the earliest constraint for each period
    all.sort_unstable_by_key(|x| x.1); // back to constraint order
    all
}

/// Step 1 of the solver: which positions must hold the same letter because
/// of the Border constraints? Returns root[i] = the smallest position that
/// is forced equal to position i (root[i] == i if there is none).
///
/// Method ("a border of a border is a border"). Let k be the longest border
/// of a text of length len, and p = len - k. Then the text has all its
/// borders exactly when
///   (a) position j equals position j - p for every j from max(p, k) to the
///       end of the text, and
///   (b) the first k characters, taken as a text of their own, have all the
///       remaining borders - and, if k > p, the border k - p as well.
/// Why: border k means "j equals j - p for every j >= p". The part of that
/// with j >= k is (a); the part with j < k (which exists only if k > p) says
/// the first k characters repeat with shift p, i.e. have border k - p. And a
/// shorter border of the whole text is the same thing as a border of its
/// first k characters, because the last k characters are a copy of them.
///
/// So each round links the positions from k upward to earlier positions
/// and then continues with the first k characters only. Every position is
/// linked at most once, whatever the number of borders: the work is
/// proportional to the text length plus (number of rounds) x log(borders),
/// and there are at most `n` rounds (a run of rounds with the same shift p
/// is done in one go, without touching the queue of borders).
fn equal_positions(n: usize, cons: &[Con]) -> Result<Vec<usize>, String> {
    let mut root = vec_of(n, 0usize)?;
    for (i, r) in root.iter_mut().enumerate() {
        *r = i;
    }
    // The given borders, longest first, plus the ones that arise on the way.
    let mut given: Vec<usize> = cons
        .iter()
        .filter_map(|c| match c {
            Con::Border(k) if *k > 0 && *k < n => Some(*k),
            _ => None,
        })
        .collect();
    given.sort_unstable_by(|a, b| b.cmp(a));
    let mut next = 0; // index of the longest given border not yet used
    let mut derived: BinaryHeap<usize> = BinaryHeap::new();
    let mut len = n;
    loop {
        // Longest border still waiting, from either source.
        let g = given.get(next).copied().unwrap_or(0);
        let d = derived.peek().copied().unwrap_or(0);
        let mut k = g.max(d);
        if k == 0 {
            break;
        }
        if g >= d {
            next += 1;
        } else {
            derived.pop();
        }
        if k >= len {
            continue; // a repeat of the border handled in the previous round
        }
        loop {
            let p = len - k;
            for j in p.max(k)..len {
                root[j] = j - p; // always an earlier position
            }
            len = k;
            if k <= p {
                break;
            }
            // The first k characters also have border k - p. If that is
            // longer than everything still waiting, carry straight on.
            let c = k - p;
            let waiting = given.get(next).copied().unwrap_or(0).max(derived.peek().copied().unwrap_or(0));
            if c > waiting {
                k = c;
            } else {
                if c < waiting {
                    derived.push(c);
                } // c == waiting: already in the queue
                break;
            }
        }
    }
    // Links only point to earlier positions, so one left-to-right pass
    // turns "linked to" into "smallest equal position".
    for i in 0..n {
        root[i] = root[root[i]];
    }
    Ok(root)
}

/// The positions a Prefix/Suffix constraint fixes: (word, position of its
/// first letter).
fn word_of(n: usize, c: &Con) -> Option<(&[u8], usize)> {
    match c {
        Con::Prefix(w) => Some((w, 0)),
        Con::Suffix(w) => Some((w, n - w.len())),
        Con::Border(_) => None,
    }
}

fn solve(n: usize, alphabet: &[u8], cons: &[Con]) -> Result<Outcome, String> {
    solve_with(n, alphabet, cons, SEARCH_BUDGET.saturating_add(32 * n as u64))
}

/// `solve` with an explicit limit on the shortest-explanation search (see
/// SEARCH_BUDGET). The answer itself never depends on the limit; only the
/// chain of links inside a contradiction's explanation can.
fn solve_with(n: usize, alphabet: &[u8], cons: &[Con], search_budget: u64) -> Result<Outcome, String> {
    validate(n, alphabet, cons)?;

    // Step 1: structural deduction. Every Border says "the text repeats
    // itself after a shift of p", so positions i and i + p hold the same
    // letter. Group the positions that are forced equal.
    let root = equal_positions(n, cons)?;

    // Step 2: literal propagation. Each group gets at most one letter.
    // letter[root] = the letter, or 0 for "none yet" (0 is never a valid
    // letter because letters are visible ASCII).
    let mut letter = vec_of(n, 0u8)?;
    for (idx, c) in cons.iter().enumerate() {
        let Some((word, offset)) = word_of(n, c) else { continue };
        for (j, &ch) in word.iter().enumerate() {
            let pos = offset + j;
            let r = root[pos];
            if letter[r] == 0 {
                letter[r] = ch;
            } else if letter[r] != ch {
                // Step 3: contradiction. Build a certificate and stop.
                let (pos0, idx0) = first_literal(n, cons, &root, r);
                let (path, via) = explain(n, cons, pos0, pos, search_budget);
                return Ok(Outcome::Unsat(Conflict {
                    path,
                    via,
                    letter_a: letter[r],
                    src_a: idx0,
                    letter_b: ch,
                    src_b: idx,
                }));
            }
        }
    }

    // Step 4: read off the solution set. Free groups are numbered in order
    // of first appearance.
    let mut id = vec_of(n, NONE)?;
    let mut free = 0usize;
    let mut pattern = Vec::new();
    pattern.try_reserve_exact(n).map_err(|_| too_long(n))?;
    for i in 0..n {
        let r = root[i];
        if letter[r] != 0 {
            pattern.push(Cell::Letter(letter[r]));
        } else {
            if id[r] == NONE {
                id[r] = free;
                free += 1;
            }
            pattern.push(Cell::Free(id[r]));
        }
    }
    Ok(Outcome::Sat { pattern, free })
}

/// The first position (in constraint order, then word order) that a
/// Prefix/Suffix constraint fixes inside the group with the given root,
/// and the index of that constraint. Only called once a contradiction is
/// found, so the group is known to contain a fixed position.
fn first_literal(n: usize, cons: &[Con], root: &[usize], r: usize) -> (usize, usize) {
    for (idx, c) in cons.iter().enumerate() {
        let Some((word, offset)) = word_of(n, c) else { continue };
        for pos in offset..offset + word.len() {
            if root[pos] == r {
                return (pos, idx);
            }
        }
    }
    unreachable!("a group with a letter has a position fixed by some constraint")
}

/// Shortest chain of equal-letter links from src to dst (breadth-first
/// search). The links are not stored anywhere: position u is linked to
/// u - p and u + p for every border period p, when those positions exist.
/// `periods` is in constraint order, which is also the order in which the
/// links of a position are tried (so ties are broken the same way as in the
/// original program, which stored the links).
///
/// Cost: for each position visited, the number of links it really has
/// (times a logarithm when there are many borders) - never more than the
/// total number of links, and the search stops as soon as dst is reached.
/// Only run when a contradiction has to be explained.
///
/// `budget` limits the number of (position, border) pairs looked at. If it
/// runs out before dst is reached the result is None and nothing is known
/// about the shortest chain; with u64::MAX the search always finishes.
fn bfs_path(n: usize, periods: &[(usize, usize)], src: usize, dst: usize, budget: u64) -> Option<(Vec<usize>, Vec<usize>)> {
    if src == dst {
        return Some((vec![src], Vec::new()));
    }
    let mut spent = 0u64;
    let mut by_size = periods.to_vec();
    by_size.sort_unstable(); // ascending period
    let mut some: Vec<(usize, usize)> = Vec::new();
    let mut prev = vec![NONE; n]; // position we came from
    let mut prev_via = vec![NONE; n]; // constraint that links the two
    let mut seen = vec![false; n];
    seen[src] = true;
    let mut queue = VecDeque::from([src]);
    'search: while let Some(u) = queue.pop_front() {
        // u - p exists only if p <= u, and u + p only if p <= n - 1 - u, so
        // periods above both limits give u no link at all. With many
        // borders most positions have few links; look at those only.
        let reach = u.max(n - 1 - u);
        let usable = by_size.partition_point(|x| x.0 <= reach);
        // Sorting the usable ones back into constraint order costs about
        // usable * log(usable); when that is no cheaper than trying every
        // period, just try every period.
        let sort_cost = usable * (usize::BITS - usable.leading_zeros()) as usize;
        let list: &[(usize, usize)] = if sort_cost >= periods.len() {
            periods
        } else {
            some.clear();
            some.extend_from_slice(&by_size[..usable]);
            some.sort_unstable_by_key(|x| x.1); // constraint order
            &some
        };
        spent = spent.saturating_add(list.len() as u64);
        if spent > budget {
            return None;
        }
        for &(p, idx) in list {
            if u >= p && !seen[u - p] {
                let v = u - p;
                seen[v] = true;
                prev[v] = u;
                prev_via[v] = idx;
                if v == dst {
                    break 'search;
                }
                queue.push_back(v);
            }
            if u + p < n && !seen[u + p] {
                let v = u + p;
                seen[v] = true;
                prev[v] = u;
                prev_via[v] = idx;
                if v == dst {
                    break 'search;
                }
                queue.push_back(v);
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
    Some((path, via))
}

// ------------------------------------------- explanation by derivation
//
// The search above finds the SHORTEST chain of links, but with very many
// different borders it may have to look at a number of links that grows
// with the square of the text length. The code below builds a chain a
// different way, without searching: it replays the grouping rounds of
// `equal_positions` and records WHY each round's links hold.
//
// Consecutive rounds with the same shift p form a PHASE. Phase r looks at
// the first `start` characters and uses their border b = start - p. Write
// M_r(y) for the claim "position y = position y + p", which holds whenever
// y + p < start. Each claim reduces to original links like this:
//
//   (a) b is a Border rule of the puzzle. Then y = y + (n - b) is an
//       original link. And y + p reaches that same position y + (n - b) by
//       stepping through the earlier phases, last one first, because the
//       shifts of the earlier phases add up to n - start.
//
//   (b) b arose at the end of an earlier phase q (the first part of the
//       text was found to repeat with shift p_q). Then y = y + p_q is the
//       claim M_q(y), and y + p reaches y + p_q by stepping through the
//       phases between q and r, last one first.
//
// Both cases refer only to earlier phases, so the reduction ends at
// original links. A union-find structure remembers which positions are
// already joined by the original links collected so far, so a claim whose
// two positions are already joined costs nothing more. An original link is
// kept only if it joins two separate groups, so the kept links form a
// forest (fewer than n links), and the explanation is the one path through
// that forest: valid, without repeated positions, never longer than the
// group - but usually NOT the shortest.

/// How many (position, border) pairs the shortest-chain search may look at
/// before the explanation is derived instead: this many, plus 32 per
/// character of the text. Below it, explanations are exactly the shortest
/// ones, as before.
const SEARCH_BUDGET: u64 = 200_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Origin {
    Rule(usize),    // constraint index of a Border rule
    Derived(usize), // the phase at whose end this border arose
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Phase {
    start: usize, // text length looked at when the phase began
    p: usize,     // shift
    count: usize, // number of rounds
    origin: Origin,
}

struct Derivation {
    n: usize,
    phases: Vec<Phase>,
    link: Vec<u32>,             // link[j] = the position j was linked to, or j
    group: Vec<u32>,            // union-find over the kept original links
    kept: Vec<(u32, u32, u32)>, // (position, position, constraint index)
}

impl Derivation {
    /// Replay the grouping rounds (see `equal_positions`), keeping the phases.
    fn new(n: usize, cons: &[Con]) -> Derivation {
        let mut given: Vec<(usize, usize)> = cons
            .iter()
            .enumerate()
            .filter_map(|(idx, c)| match c {
                Con::Border(k) if *k > 0 && *k < n => Some((*k, idx)),
                _ => None,
            })
            .collect();
        // longest first; for equal borders the earliest rule
        given.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        given.dedup_by_key(|x| x.0);
        let mut next = 0;
        let mut derived: BinaryHeap<(usize, usize)> = BinaryHeap::new(); // (border, phase)
        let mut link: Vec<u32> = (0..n as u32).collect();
        let mut phases: Vec<Phase> = Vec::new();
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
                (k, Origin::Derived(q))
            } else {
                next += 1;
                (g, Origin::Rule(given[next - 1].1))
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
                // borders as long as the text now looked at say nothing
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
            phases.push(Phase { start, p, count, origin });
        }
        Derivation { n, phases, link, group: (0..n as u32).collect(), kept: Vec::new() }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.group[x] as usize != x {
            let g = self.group[self.group[x] as usize];
            self.group[x] = g; // path halving
            x = g as usize;
        }
        x
    }

    fn joined(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    /// Look at one original link; keep it if it joins two separate groups.
    fn original(&mut self, a: usize, b: usize, idx: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            if ra < rb {
                self.group[rb] = ra as u32;
            } else {
                self.group[ra] = rb as u32;
            }
            self.kept.push((a as u32, b as u32, idx as u32));
        }
    }

    /// Make sure "position y = position y + p_r" follows from kept links.
    fn claim(&mut self, r: usize, y: usize) {
        let Phase { start, p, origin, .. } = self.phases[r];
        if self.joined(y, y + p) {
            return;
        }
        let (target, from_phase) = match origin {
            Origin::Rule(idx) => {
                let far = y + (self.n - (start - p));
                self.original(y, far, idx);
                (far, 0)
            }
            Origin::Derived(q) => {
                self.claim(q, y);
                (y + self.phases[q].p, q + 1)
            }
        };
        let mut z = y + p;
        if !self.joined(z, target) {
            for q in (from_phase..r).rev() {
                z = self.walk(q, z);
            }
        }
    }

    /// Step through every round of phase q starting at position z, from
    /// the far end back, stopping once z is joined to the part covered.
    fn walk(&mut self, q: usize, z: usize) -> usize {
        let Phase { p, count, .. } = self.phases[q];
        let mut i = count;
        while i > 0 && !self.joined(z, z + i * p) {
            i -= 1;
            self.claim(q, z + i * p);
        }
        z + count * p
    }

    /// Join position a to the smallest position of its group.
    fn anchor(&mut self, a: usize) -> usize {
        let mut j = a;
        while self.link[j] as usize != j {
            // phases look at ever shorter texts; the phase that linked j
            // is the last one whose text still contained j
            let r = self.phases.partition_point(|ph| ph.start > j) - 1;
            let to = self.link[j] as usize;
            self.claim(r, to);
            j = to;
        }
        j
    }

    /// The path from a to b through the forest of kept links.
    fn path(&mut self, a: usize, b: usize) -> Option<(Vec<usize>, Vec<usize>)> {
        if self.anchor(a) != self.anchor(b) || !self.joined(a, b) {
            return None;
        }
        if a == b {
            return Some((vec![a], Vec::new()));
        }
        let n = self.n;
        let mut first = vec![0u32; n + 1]; // first[u]..first[u + 1]: links of u
        for &(u, v, _) in &self.kept {
            first[u as usize + 1] += 1;
            first[v as usize + 1] += 1;
        }
        for i in 0..n {
            first[i + 1] += first[i];
        }
        let mut fill = first.clone();
        let mut links = vec![(0u32, 0u32); self.kept.len() * 2];
        for &(u, v, idx) in &self.kept {
            links[fill[u as usize] as usize] = (v, idx);
            fill[u as usize] += 1;
            links[fill[v as usize] as usize] = (u, idx);
            fill[v as usize] += 1;
        }
        let mut prev = vec![u32::MAX; n];
        let mut prev_via = vec![u32::MAX; n];
        prev[a] = a as u32;
        let mut stack = vec![a];
        while let Some(u) = stack.pop() {
            if u == b {
                break;
            }
            for e in first[u] as usize..first[u + 1] as usize {
                let (v, idx) = links[e];
                if prev[v as usize] == u32::MAX {
                    prev[v as usize] = u as u32;
                    prev_via[v as usize] = idx;
                    stack.push(v as usize);
                }
            }
        }
        if prev[b] == u32::MAX {
            return None;
        }
        let (mut path, mut via, mut at) = (vec![b], Vec::new(), b);
        while at != a {
            via.push(prev_via[at] as usize);
            at = prev[at] as usize;
            path.push(at);
        }
        path.reverse();
        via.reverse();
        Some((path, via))
    }
}

// ------------------------------------- the reduction without the text
//
// The phases of the grouping depend only on the border lengths, not on the
// positions, and a run of rounds with the same shift is one division. So
// they can be computed without an array as long as the text.
//
// PROVED (docs/research/06):
//   * the shifts of successive phases satisfy p[i+2] < p[i] / 2, so there
//     are at most 2 * log2(n) + 2 phases, whatever the number of borders;
//   * the Border rules that start a phase (the "generators", at most that
//     many) already force every equality that all the Border rules force.

/// The phases of the grouping (the same list as `Derivation::new` builds),
/// computed in time O((m + log n) log m) for m Border rules: no position
/// of the text is touched.
fn reduction_phases(n: usize, cons: &[Con]) -> Vec<Phase> {
    let mut given: Vec<(usize, usize)> = cons
        .iter()
        .enumerate()
        .filter_map(|(idx, c)| match c {
            Con::Border(k) if *k > 0 && *k < n => Some((*k, idx)),
            _ => None,
        })
        .collect();
    given.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    given.dedup_by_key(|x| x.0);
    let mut next = 0;
    let mut derived: BinaryHeap<(usize, usize)> = BinaryHeap::new(); // (border, phase)
    let mut phases: Vec<Phase> = Vec::new();
    let mut len = n;
    loop {
        let g = given.get(next).map_or(0, |x| x.0);
        let d = derived.peek().map_or(0, |x| x.0);
        if g == 0 && d == 0 {
            break;
        }
        let (k, origin) = if d >= g {
            let (k, q) = derived.pop().unwrap();
            (k, Origin::Derived(q))
        } else {
            next += 1;
            (g, Origin::Rule(given[next - 1].1))
        };
        if k >= len {
            continue;
        }
        let id = phases.len();
        let (start, p) = (len, len - k);
        let mut count = 1;
        len = k;
        // Further rounds with the same shift: the next border is len - p,
        // as long as that is at least the longest border still waiting.
        while len > p {
            while given.get(next).map_or(false, |x| x.0 >= len) {
                next += 1;
            }
            while derived.peek().map_or(false, |x| x.0 >= len) {
                derived.pop();
            }
            let waiting = given.get(next).map_or(0, |x| x.0).max(derived.peek().map_or(0, |x| x.0));
            // all the rounds that keep len - p above `waiting`, in one go
            let more = (len - waiting - 1) / p;
            len -= more * p;
            count += more;
            if len <= p {
                break;
            }
            let c = len - p; // now c <= waiting
            if c == waiting {
                len = c; // same border from two sources: one more round
                count += 1;
            } else {
                derived.push((c, id));
                break;
            }
        }
        phases.push(Phase { start, p, count, origin });
    }
    phases
}

/// Number of groups of positions, from the phases alone: what is left of
/// the text after the last phase, plus the positions each phase leaves
/// unlinked (those from its last border up to its shift).
fn groups_from_phases(n: usize, phases: &[Phase]) -> usize {
    let (mut len, mut free) = (n, 0);
    for ph in phases {
        len = ph.start - ph.count * ph.p;
        free += ph.p.saturating_sub(len);
    }
    len + free
}

/// The generators: (period, constraint index) of the Border rules that
/// start a phase, in phase order. Their links alone force every equality
/// that all the Border rules force, and there are at most 2 * log2(n) + 2
/// of them.
fn generator_periods(n: usize, cons: &[Con]) -> Vec<(usize, usize)> {
    reduction_phases(n, cons)
        .iter()
        .filter_map(|ph| match ph.origin {
            Origin::Rule(idx) => Some((n - (ph.start - ph.p), idx)),
            Origin::Derived(_) => None,
        })
        .collect()
}

/// A chain of original links from src to dst that uses generator borders
/// only: the shortest such chain, by breadth-first search. Looks at no
/// more than 2 * n * (number of generators) links, that is
/// O(n log n) - PROVED, unlike the cost of `derive_path`. None if the two
/// positions are not forced equal.
fn generator_path(n: usize, cons: &[Con], src: usize, dst: usize) -> Option<(Vec<usize>, Vec<usize>)> {
    let mut gens = generator_periods(n, cons);
    gens.sort_unstable_by_key(|x| x.1); // constraint order, as bfs_path expects
    let (path, via) = bfs_path(n, &gens, src, dst, u64::MAX)?;
    (path.first() == Some(&src) && path.last() == Some(&dst) && via.len() + 1 == path.len()).then_some((path, via))
}

/// A chain of original links from src to dst, built by derivation. None
/// if the two positions are not forced equal (or the text is too long for
/// the 32-bit tables used here).
fn derive_path(n: usize, cons: &[Con], src: usize, dst: usize) -> Option<(Vec<usize>, Vec<usize>)> {
    if n >= u32::MAX as usize || cons.len() >= u32::MAX as usize {
        return None;
    }
    let mut d = Derivation::new(n, cons);
    // `claim` calls itself at most once per phase, so give it a stack that
    // grows with the number of phases (memory is only used if needed).
    let stack = (16usize << 20).saturating_add(d.phases.len().saturating_mul(512));
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn_scoped(scope, || d.path(src, dst))
            .ok()
            .and_then(|h| h.join().ok())
            .flatten()
    })
}

/// First shortening pass ("farthest reach"). From the current position of
/// the chain, try every border period in both directions and jump to the
/// position that lies farthest along the chain; repeat from there. Only
/// the positions actually landed on are examined, so the cost is
/// 2 * (periods) look-ups per position of the RESULT, and the pass stops
/// shortening (keeping the rest of the chain as it is) once it has spent
/// 64 n + 10,000,000 look-ups. PROVED: the result is a valid chain, never
/// longer than the input. NOT claimed: that it is the shortest chain.
fn reach_shorten(n: usize, cons: &[Con], path: Vec<usize>, via: Vec<usize>) -> (Vec<usize>, Vec<usize>) {
    if path.len() < 3 || n >= u32::MAX as usize {
        return (path, via);
    }
    let periods = border_periods(n, cons);
    let step_cost = 2 * periods.len() as u64;
    let budget = 64 * n as u64 + 10_000_000;
    let mut at = vec![u32::MAX; n]; // position -> index in the chain
    for (i, &pos) in path.iter().enumerate() {
        at[pos] = i as u32;
    }
    let last = path.len() - 1;
    let (mut out, mut out_via, mut spent, mut i) = (vec![path[0]], Vec::new(), 0u64, 0usize);
    while i < last {
        if spent + step_cost > budget {
            out.extend_from_slice(&path[i + 1..]);
            out_via.extend_from_slice(&via[i..]);
            break;
        }
        spent += step_cost;
        let u = path[i];
        let (mut jump, mut rule) = (i + 1, via[i]);
        for &(q, idx) in &periods {
            if u + q < n && at[u + q] != u32::MAX && at[u + q] as usize > jump {
                (jump, rule) = (at[u + q] as usize, idx);
            }
            if u >= q && at[u - q] != u32::MAX && at[u - q] as usize > jump {
                (jump, rule) = (at[u - q] as usize, idx);
            }
        }
        out.push(path[jump]);
        out_via.push(rule);
        i = jump;
    }
    (out, out_via)
}

/// Shorten a chain without searching: two positions are directly linked
/// exactly when their distance is a border period, which a table answers
/// at once. From each position, jump to the farthest later position of the
/// chain that is directly linked to it. At most 8 look-ups per position of
/// the chain are spent; after that the rest is kept as it is. The result is
/// a valid chain, often far shorter, but not always the shortest.
fn shorten_path(n: usize, cons: &[Con], path: Vec<usize>, via: Vec<usize>) -> (Vec<usize>, Vec<usize>) {
    let (path, via) = reach_shorten(n, cons, path, via);
    let mut rule_of = vec![NONE; n]; // distance -> first Border rule with that period
    for (idx, c) in cons.iter().enumerate() {
        if let Con::Border(k) = c {
            if *k > 0 && *k < n && rule_of[n - *k] == NONE {
                rule_of[n - *k] = idx;
            }
        }
    }
    let budget = 8 * path.len() as u64 + 64;
    let (mut out, mut out_via, mut spent, mut i) = (vec![path[0]], Vec::new(), 0u64, 0usize);
    let last = path.len() - 1;
    while i < last {
        let (mut jump, mut rule) = (i + 1, via[i]);
        let mut j = last;
        while j > i + 1 && spent < budget {
            spent += 1;
            let found = rule_of[path[i].abs_diff(path[j])];
            if found != NONE {
                (jump, rule) = (j, found);
                break;
            }
            j -= 1;
        }
        out.push(path[jump]);
        out_via.push(rule);
        i = jump;
    }
    (out, out_via)
}

/// The chain of links shown as the explanation of a contradiction: the
/// shortest one if the search finds it within `budget`; otherwise the
/// shortest chain through generator borders only (cost O(n log n), proved),
/// shortened further where that is cheap. Valid, not necessarily shortest.
/// `derive_path` is the earlier fallback, kept as a second line.
fn explain(n: usize, cons: &[Con], src: usize, dst: usize, budget: u64) -> (Vec<usize>, Vec<usize>) {
    let periods = border_periods(n, cons);
    if let Some(found) = bfs_path(n, &periods, src, dst, budget) {
        return found;
    }
    if let Some((path, via)) = generator_path(n, cons, src, dst) {
        return shorten_path(n, cons, path, via);
    }
    if let Some((path, via)) = derive_path(n, cons, src, dst) {
        return shorten_path(n, cons, path, via);
    }
    bfs_path(n, &periods, src, dst, u64::MAX).expect("positions of one group are connected")
}

// ----------------------------------------------------------- certificate
//
// A certificate is a text that lets a separate program (certcheck.rs,
// which shares no code with this file) confirm the answer without solving
// the puzzle. Format: docs/certificate-format.md. Positions count from 0,
// rules from 1.
//
//   solvable:       one group number per position ("labels") and a list of
//                   original links that joins exactly those groups. The
//                   links are taken from the generator borders only, so
//                   building them looks at O(n log n) links (PROVED).
//   contradiction:  the two clashing letters, the rules that fix them, and
//                   the chain of links between their positions.

fn certificate(n: usize, alphabet: &[u8], cons: &[Con]) -> Result<String, String> {
    use std::fmt::Write;
    let outcome = solve(n, alphabet, cons)?;
    if n >= u32::MAX as usize {
        return Err(too_long(n));
    }
    let mut out = String::new();
    let _ = writeln!(out, "prefsuf-certificate 1\nlength {n}\nletters {}\nrules {}", text(alphabet), cons.len());
    for c in cons {
        let _ = match c {
            Con::Prefix(w) if w.is_empty() => writeln!(out, "P"),
            Con::Suffix(w) if w.is_empty() => writeln!(out, "S"),
            Con::Prefix(w) => writeln!(out, "P {}", text(w)),
            Con::Suffix(w) => writeln!(out, "S {}", text(w)),
            Con::Border(k) => writeln!(out, "B {k}"),
        };
    }
    match outcome {
        Outcome::Unsat(cf) => {
            let _ = writeln!(out, "claim contradiction");
            let _ = writeln!(out, "first {} {} {}", cf.path[0], cf.letter_a as char, cf.src_a + 1);
            let _ = writeln!(out, "second {} {} {}", cf.path[cf.path.len() - 1], cf.letter_b as char, cf.src_b + 1);
            let _ = writeln!(out, "chain {}", cf.path.len());
            write_numbers(&mut out, cf.path.iter().copied());
            let _ = writeln!(out, "via");
            write_numbers(&mut out, cf.via.iter().map(|idx| idx + 1));
        }
        Outcome::Sat { free, .. } => {
            let root = equal_positions(n, cons)?;
            // group numbers in order of first appearance
            let mut id = vec_of(n, u32::MAX)?;
            let mut groups = 0u32;
            for i in 0..n {
                if root[i] == i {
                    id[i] = groups;
                    groups += 1;
                }
            }
            let _ = writeln!(out, "claim solvable\ngroups {groups}\nfree {free}\nlabels");
            write_numbers(&mut out, root.iter().map(|r| id[*r] as usize));
            drop(id);
            // spanning forest from the links of the generator borders
            let mut group: Vec<u32> = (0..n as u32).collect();
            fn top(group: &mut [u32], mut x: u32) -> u32 {
                while group[x as usize] != x {
                    group[x as usize] = group[group[x as usize] as usize];
                    x = group[x as usize];
                }
                x
            }
            let mut sections: Vec<(usize, Vec<u32>)> = Vec::new();
            let mut total = 0usize;
            for (q, idx) in generator_periods(n, cons) {
                let mut kept: Vec<u32> = Vec::new();
                for i in 0..n - q {
                    let (x, y) = (top(&mut group, i as u32), top(&mut group, (i + q) as u32));
                    if x != y {
                        group[x.max(y) as usize] = x.min(y);
                        kept.push(i as u32);
                    }
                }
                total += kept.len();
                if !kept.is_empty() {
                    sections.push((idx + 1, kept));
                }
            }
            let _ = writeln!(out, "forest {total}");
            for (rule, kept) in sections {
                let _ = writeln!(out, "links {rule} {}", kept.len());
                write_numbers(&mut out, kept.iter().map(|u| *u as usize));
            }
        }
    }
    out.push_str("end\n");
    Ok(out)
}

/// Whole numbers separated by spaces, 16 to a line.
fn write_numbers(out: &mut String, numbers: impl Iterator<Item = usize>) {
    use std::fmt::Write;
    let mut on_line = 0;
    for v in numbers {
        if on_line == 16 {
            out.push('\n');
            on_line = 0;
        } else if on_line > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{v}");
        on_line += 1;
    }
    if on_line > 0 {
        out.push('\n');
    }
}

// ------------------------------------------------------ result helpers

/// Exact number of solutions, or None if it does not fit in u128
/// (it is still exactly alphabet_len ^ free).
fn count(alphabet_len: usize, free: usize) -> Option<u128> {
    if alphabet_len == 1 {
        return Some(1); // one letter: one text, however many free classes
    }
    u32::try_from(free).ok().and_then(|f| (alphabet_len as u128).checked_pow(f))
}

fn is_unique(alphabet_len: usize, free: usize) -> bool {
    free == 0 || alphabet_len == 1
}

/// Solution obtained by giving free class i the letter assignment[i].
fn instantiate(pattern: &[Cell], assignment: &[u8]) -> Vec<u8> {
    pattern
        .iter()
        .map(|c| match c {
            Cell::Letter(ch) => *ch,
            Cell::Free(i) => assignment[*i],
        })
        .collect()
}

/// One solution: every free class set to the first alphabet letter.
fn witness(pattern: &[Cell], free: usize, alphabet: &[u8]) -> Vec<u8> {
    instantiate(pattern, &vec![alphabet[0]; free])
}

// ------------------------------------- independent checking utilities
// These use only the problem definition, not the solver's data structures.

/// Direct check of a candidate against the definition.
fn satisfies(s: &[u8], n: usize, alphabet: &[u8], cons: &[Con]) -> bool {
    if s.len() != n || s.iter().any(|ch| !alphabet.contains(ch)) {
        return false;
    }
    cons.iter().all(|c| match c {
        Con::Prefix(w) => s.starts_with(w),
        Con::Suffix(w) => s.ends_with(w),
        Con::Border(k) => s[..*k] == s[n - *k..],
    })
}

/// The same answer as `satisfies`, in time proportional to the text plus
/// the rules instead of the total length of all borders. The Border rules
/// are checked with the classical border table of the text (the "failure
/// function" of Knuth, Morris and Pratt): fail[i] is the longest border of
/// the first i characters, and the borders of the whole text are fail[n],
/// fail[fail[n]], ... This uses nothing of the solver's own reduction, so
/// it remains an independent double-check of the answer.
fn satisfies_fast(s: &[u8], n: usize, alphabet: &[u8], cons: &[Con]) -> bool {
    // With few or short borders the direct comparisons read less than one
    // pass over the text would; both give the same answer, so this choice
    // only affects cost.
    let direct_cost = cons.iter().fold(0usize, |sum, c| match c {
        Con::Border(k) if *k < n => sum.saturating_add(*k),
        _ => sum,
    });
    if cons.iter().any(|c| matches!(c, Con::Border(k) if *k > n)) {
        return false; // never a valid rule; the direct check must not be given it
    }
    if n >= u32::MAX as usize || direct_cost <= n.saturating_mul(16) {
        return satisfies(s, n, alphabet, cons);
    }
    let mut allowed = [false; 256];
    for a in alphabet {
        allowed[*a as usize] = true;
    }
    if s.len() != n || s.iter().any(|ch| !allowed[*ch as usize]) {
        return false;
    }
    let mut is_border: Vec<bool> = Vec::new(); // built only if a Border rule needs it
    cons.iter().all(|c| match c {
        Con::Prefix(w) => s.starts_with(w),
        Con::Suffix(w) => s.ends_with(w),
        Con::Border(k) if *k == 0 || *k == n => true,
        Con::Border(k) if *k > n => false,
        Con::Border(k) => {
            if is_border.is_empty() {
                let mut fail = vec![0u32; n + 1];
                let mut b = 0usize;
                for i in 1..n {
                    while b > 0 && s[i] != s[b] {
                        b = fail[b] as usize;
                    }
                    if s[i] == s[b] {
                        b += 1;
                    }
                    fail[i + 1] = b as u32;
                }
                is_border = vec![false; n + 1];
                let mut b = fail[n] as usize;
                while b > 0 {
                    is_border[b] = true;
                    b = fail[b] as usize;
                }
            }
            is_border[*k]
        }
    })
}

/// Validate an UNSAT certificate without trusting the solver.
fn check_conflict(cf: &Conflict, n: usize, cons: &[Con]) -> bool {
    let fixes = |idx: usize, pos: usize, ch: u8| -> bool {
        match cons.get(idx) {
            Some(Con::Prefix(w)) => pos < w.len() && w[pos] == ch,
            Some(Con::Suffix(w)) => {
                let start = n - w.len();
                pos >= start && pos - start < w.len() && w[pos - start] == ch
            }
            _ => false,
        }
    };
    if cf.letter_a == cf.letter_b || cf.path.is_empty() || cf.via.len() != cf.path.len() - 1 {
        return false;
    }
    if !fixes(cf.src_a, cf.path[0], cf.letter_a) {
        return false;
    }
    if !fixes(cf.src_b, *cf.path.last().unwrap(), cf.letter_b) {
        return false;
    }
    for (pair, &idx) in cf.path.windows(2).zip(&cf.via) {
        let (lo, hi) = (pair[0].min(pair[1]), pair[0].max(pair[1]));
        match cons.get(idx) {
            Some(Con::Border(k)) if hi - lo == n - *k && lo < *k => {}
            _ => return false,
        }
    }
    true
}

// --------------------------------------------------------- self-tests

struct Tally {
    passed: usize,
    failed: usize,
}

impl Tally {
    fn check(&mut self, name: &str, ok: bool) {
        if ok {
            self.passed += 1;
            println!("  ok    {name}");
        } else {
            self.failed += 1;
            println!("  FAIL  {name}");
        }
    }
}

fn p(w: &str) -> Con {
    Con::Prefix(w.as_bytes().to_vec())
}
fn s(w: &str) -> Con {
    Con::Suffix(w.as_bytes().to_vec())
}
fn b(k: usize) -> Con {
    Con::Border(k)
}

/// All strings of length n over alphabet, by counting in base |alphabet|.
fn all_strings(n: usize, alphabet: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut digits = vec![0usize; n];
    loop {
        out.push(digits.iter().map(|&d| alphabet[d]).collect());
        let mut i = 0;
        loop {
            if i == n {
                return out;
            }
            digits[i] += 1;
            if digits[i] < alphabet.len() {
                break;
            }
            digits[i] = 0;
            i += 1;
        }
    }
}

/// Brute-force oracle: used ONLY in tests, on tiny instances.
fn brute(n: usize, alphabet: &[u8], cons: &[Con]) -> BTreeSet<Vec<u8>> {
    all_strings(n, alphabet).into_iter().filter(|x| satisfies(x, n, alphabet, cons)).collect()
}

/// Expand the solver's answer into the explicit set it describes.
fn solution_set(out: &Outcome, alphabet: &[u8]) -> BTreeSet<Vec<u8>> {
    match out {
        Outcome::Unsat(_) => BTreeSet::new(),
        Outcome::Sat { pattern, free } => {
            all_strings(*free, alphabet).iter().map(|a| instantiate(pattern, a)).collect()
        }
    }
}

fn set_of(words: &[&str]) -> BTreeSet<Vec<u8>> {
    words.iter().map(|w| w.as_bytes().to_vec()).collect()
}

/// The solver's whole answer agrees with brute force on this instance.
fn agrees(n: usize, alphabet: &[u8], cons: &[Con]) -> bool {
    let out = match solve(n, alphabet, cons) {
        Ok(o) => o,
        Err(_) => return false,
    };
    let expected = brute(n, alphabet, cons);
    if solution_set(&out, alphabet) != expected {
        return false;
    }
    match &out {
        Outcome::Unsat(cf) => expected.is_empty() && check_conflict(cf, n, cons),
        Outcome::Sat { free, .. } => {
            count(alphabet.len(), *free) == Some(expected.len() as u128)
                && is_unique(alphabet.len(), *free) == (expected.len() == 1)
        }
    }
}

struct Rng(u64);
impl Rng {
    fn below(&mut self, m: usize) -> usize {
        // xorshift64*; deterministic, good enough for test-case generation
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        ((self.0.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as usize) % m
    }
}

fn random_instance(rng: &mut Rng, max_n: usize) -> (usize, Vec<u8>, Vec<Con>) {
    let n = rng.below(max_n + 1);
    let alphabet = b"abc"[..1 + rng.below(3)].to_vec();
    let mut cons = Vec::new();
    for _ in 0..rng.below(5) {
        let kind = rng.below(3);
        if kind == 2 {
            cons.push(Con::Border(rng.below(n + 1)));
        } else {
            let len = rng.below(n + 1);
            let w: Vec<u8> = (0..len).map(|_| alphabet[rng.below(alphabet.len())]).collect();
            cons.push(if kind == 0 { Con::Prefix(w) } else { Con::Suffix(w) });
        }
    }
    (n, alphabet, cons)
}

fn sat_parts(out: &Outcome) -> Option<(&Vec<Cell>, usize)> {
    match out {
        Outcome::Sat { pattern, free } => Some((pattern, *free)),
        Outcome::Unsat(_) => None,
    }
}

fn unique_is(n: usize, alphabet: &str, cons: &[Con], expected: &str) -> bool {
    let a = alphabet.as_bytes();
    match solve(n, a, cons) {
        Ok(out) => match sat_parts(&out) {
            Some((pat, free)) => is_unique(a.len(), free) && witness(pat, free, a) == expected.as_bytes(),
            None => false,
        },
        Err(_) => false,
    }
}

fn run_tests() -> bool {
    use Cell::{Free as F, Letter as L};
    let mut t = Tally { passed: 0, failed: 0 };
    let ab = b"ab";

    println!("ordinary cases");
    // Border(4) on length 6 means period 2; the prefix fixes one full period.
    t.check("unique from border and prefix", unique_is(6, "ab", &[b(4), p("ab")], "ababab"));
    t.check("prefix and suffix cover everything", unique_is(5, "abc", &[p("abc"), s("ca")], "abcca"));
    t.check("overlapping prefix and suffix agree", unique_is(4, "ab", &[p("aba"), s("bab")], "abab"));
    // On length 8, Border(6) gives period 2 and Border(5) gives period 3.
    // Periods 2 and 3 with 2 + 3 - gcd(2, 3) <= 8 force period 1 (Fine-Wilf).
    // The solver is never told this; it falls out of the equality closure.
    let out = solve(8, ab, &[b(6), b(5)]).unwrap();
    t.check(
        "implicit period from two borders",
        sat_parts(&out).map(|x| x.1) == Some(1)
            && solution_set(&out, ab) == set_of(&["aaaaaaaa", "bbbbbbbb"]),
    );

    println!("edge cases");
    t.check("empty string", unique_is(0, "ab", &[p(""), s(""), b(0)], ""));
    let out = solve(3, ab, &[b(0), b(3)]).unwrap();
    t.check(
        "trivial borders add nothing",
        sat_parts(&out).map(|x| x.1) == Some(3) && count(2, 3) == Some(8) && !is_unique(2, 3),
    );
    t.check("one-letter alphabet is unique", unique_is(4, "a", &[], "aaaa"));
    let out = solve(3, ab, &[p("ab"), p("ab"), p("a")]).unwrap();
    t.check(
        "duplicate constraints are harmless",
        sat_parts(&out) == Some((&vec![L(b'a'), L(b'b'), F(0)], 1)),
    );
    t.check("reject empty alphabet", solve(3, b"", &[]).is_err());
    t.check("reject repeated alphabet letter", solve(3, b"aa", &[]).is_err());
    t.check("reject prefix longer than n", solve(3, ab, &[p("abab")]).is_err());
    t.check("reject suffix letter outside alphabet", solve(3, ab, &[s("abc")]).is_err());
    t.check("reject border longer than n", solve(3, ab, &[b(4)]).is_err());

    println!("contradictions");
    let cons = [p("ab"), s("aa")]; // position 1 must be both b and a
    let out = solve(3, ab, &cons).unwrap();
    t.check(
        "direct prefix/suffix clash",
        matches!(&out, Outcome::Unsat(cf) if check_conflict(cf, 3, &cons) && cf.path.len() == 1),
    );
    // Length 7 with Border(4) has period 3, so positions 0 ~ 3 ~ 6.
    let cons = [b(4), p("a"), s("b")];
    let out = solve(7, ab, &cons).unwrap();
    t.check(
        "clash through border chain",
        matches!(&out, Outcome::Unsat(cf) if check_conflict(cf, 7, &cons)
            && cf.path[0] == 0 && *cf.path.last().unwrap() == 6),
    );
    let tampered = match &out {
        Outcome::Unsat(cf) => {
            let mut bad = cf.clone();
            *bad.path.last_mut().unwrap() = 5;
            !check_conflict(&bad, 7, &cons)
        }
        _ => false,
    };
    t.check("tampered certificate is rejected", tampered);

    println!("ambiguity / insufficient information");
    let out = solve(3, b"abc", &[]).unwrap();
    t.check(
        "no constraints: everything free",
        sat_parts(&out) == Some((&vec![F(0), F(1), F(2)], 3)) && count(3, 3) == Some(27),
    );
    let out = solve(5, ab, &[p("ab"), s("a")]).unwrap();
    t.check(
        "partial information reports exact gap",
        sat_parts(&out) == Some((&vec![L(b'a'), L(b'b'), F(0), F(1), L(b'a')], 2))
            && count(2, 2) == Some(4),
    );
    let out = solve(4, ab, &[b(2)]).unwrap(); // s0=s2, s1=s3, nothing fixed
    t.check(
        "border only links but does not fix",
        sat_parts(&out) == Some((&vec![F(0), F(1), F(0), F(1)], 2))
            && solution_set(&out, ab) == set_of(&["aaaa", "abab", "baba", "bbbb"]),
    );
    let cons = [b(3), p("b")];
    let out = solve(5, ab, &cons).unwrap();
    t.check(
        "witness is a solution",
        sat_parts(&out).map_or(false, |(pat, free)| satisfies(&witness(pat, free, ab), 5, ab, &cons)),
    );
    t.check("count overflow is reported as None", count(3, 200).is_none());

    println!("oracle comparisons");
    let mut rng = Rng(20261113);
    let mut bad: Option<(usize, Vec<u8>, Vec<Con>)> = None;
    for _ in 0..3000 {
        let (n, alphabet, cons) = random_instance(&mut rng, 7);
        if !agrees(n, &alphabet, &cons) {
            bad = Some((n, alphabet, cons));
            break;
        }
    }
    if let Some(inst) = &bad {
        println!("        counterexample: {inst:?}");
    }
    t.check("3000 random instances agree with brute force", bad.is_none());

    // Every instance with n <= 3, alphabet "ab", at most 2 constraints.
    let mut all_ok = true;
    let mut instances = 0usize;
    for n in 0..=3usize {
        let mut atoms: Vec<Con> = Vec::new();
        for k in 0..=n {
            for w in all_strings(k, ab) {
                atoms.push(Con::Prefix(w.clone()));
                atoms.push(Con::Suffix(w));
            }
            atoms.push(Con::Border(k));
        }
        instances += 1;
        all_ok &= agrees(n, ab, &[]);
        for x in &atoms {
            instances += 1;
            all_ok &= agrees(n, ab, &[x.clone()]);
            for y in &atoms {
                instances += 1;
                all_ok &= agrees(n, ab, &[x.clone(), y.clone()]);
            }
        }
    }
    t.check(&format!("exhaustive small universe ({instances} instances)"), all_ok);

    println!("shortcuts in the solver");
    t.check(
        "repeated and redundant borders",
        unique_is(12, "ab", &[b(10), b(10), b(8), b(6), b(0), b(12), p("ab")], "abababababab"),
    );
    // Up to 12 borders per instance, so that the "smallest period first"
    // shortcut and the skipping of repeated/implied borders are exercised.
    let mut rng = Rng(77);
    let mut bad: Option<(usize, Vec<Con>)> = None;
    for _ in 0..2000 {
        let n = 1 + rng.below(11);
        let mut cons: Vec<Con> = (0..rng.below(13)).map(|_| Con::Border(rng.below(n + 1))).collect();
        for _ in 0..rng.below(3) {
            let len = rng.below(n.min(3) + 1);
            let w: Vec<u8> = (0..len).map(|_| ab[rng.below(2)]).collect();
            cons.push(if rng.below(2) == 0 { Con::Prefix(w) } else { Con::Suffix(w) });
        }
        for i in (1..cons.len()).rev() {
            cons.swap(i, rng.below(i + 1));
        }
        if !agrees(n, ab, &cons) {
            bad = Some((n, cons));
            break;
        }
    }
    if let Some(inst) = &bad {
        println!("        counterexample: {inst:?}");
    }
    t.check("2000 many-border instances agree with brute force", bad.is_none());
    // Two identical borders: the explanation must cite the first one only.
    let cons = [b(4), b(4), p("a"), s("b")];
    let out = solve(7, ab, &cons).unwrap();
    t.check(
        "explanation cites the first of two identical borders",
        matches!(&out, Outcome::Unsat(cf) if check_conflict(cf, 7, &cons) && cf.via == vec![0, 0]),
    );
    // Border(6) links neighbours (6 steps from 0 to 6); Border(4) links
    // positions 3 apart (2 steps). The explanation must take the short way.
    let cons = [b(6), b(4), p("a"), s("b")];
    let out = solve(7, ab, &cons).unwrap();
    t.check(
        "explanation is the shortest chain",
        matches!(&out, Outcome::Unsat(cf) if check_conflict(cf, 7, &cons) && cf.path == vec![0, 3, 6]),
    );

    // The grouping step works down through "borders of borders". These
    // shapes make it recurse several levels: a block written twice whose
    // block itself repeats, every even period, and every border at once.
    let nested = agrees(14, ab, &[b(7), b(4), b(1)])
        && agrees(14, ab, &[b(1), b(4), b(7), p("ab")])
        && agrees(12, ab, &[b(6), b(4), b(2)])
        && agrees(13, ab, &(1..13).map(b).collect::<Vec<Con>>())
        && agrees(13, ab, &(1..13).rev().map(b).chain([p("a"), s("b")]).collect::<Vec<Con>>());
    t.check("nested and overlapping borders agree with brute force", nested);
    let cons: Vec<Con> = (1..9).rev().map(b).chain([p("a"), s("b")]).collect();
    let out = solve(9, ab, &cons).unwrap();
    t.check(
        "explanation among many borders is direct",
        matches!(&out, Outcome::Unsat(cf) if check_conflict(cf, 9, &cons) && cf.path == vec![0, 8]),
    );
    t.check(
        "one-letter alphabet: the count is 1 however large the text",
        count(1, 0) == Some(1) && count(1, usize::MAX) == Some(1) && count(2, usize::MAX).is_none(),
    );
    t.check(
        "a text too long for memory is refused, not crashed",
        solve(usize::MAX, ab, &[]).is_err() && solve(usize::MAX / 8, ab, &[b(5)]).is_err(),
    );

    // The reduction theorem behind the grouping step, in its counting form
    // (see docs/research/01): with p the smallest shift (p = n - longest
    // border) and k = n - p,
    //     groups(n, P) = groups(k, P') + max(0, 2p - n),
    //     P' = { q - p : q in P, q > p } and p itself if p < k.
    // Evaluated here directly from that recurrence, independently of
    // equal_positions, and compared with it.
    fn groups_by_theorem(mut n: usize, shifts: &[usize]) -> usize {
        let mut set: Vec<usize> = shifts.to_vec();
        let mut free = 0;
        loop {
            set.retain(|q| *q > 0 && *q < n);
            let Some(&p) = set.iter().min() else { return free + n };
            let k = n - p;
            free += (2 * p).saturating_sub(n);
            set = set.iter().filter(|q| **q > p).map(|q| q - p).chain((p < k).then_some(p)).collect();
            n = k;
        }
    }
    let mut rng = Rng(1965);
    let mut bad: Option<(usize, Vec<usize>)> = None;
    for _ in 0..3000 {
        let n = 1 + rng.below(300);
        let borders: Vec<usize> = (0..rng.below(8)).map(|_| rng.below(n + 1)).collect();
        let cons: Vec<Con> = borders.iter().map(|k| b(*k)).collect();
        let shifts: Vec<usize> = borders.iter().map(|k| n - k).collect();
        let mut roots = equal_positions(n, &cons).unwrap();
        roots.sort_unstable();
        roots.dedup();
        if roots.len() != groups_by_theorem(n, &shifts) {
            bad = Some((n, borders));
            break;
        }
    }
    if let Some(inst) = &bad {
        println!("        counterexample: {inst:?}");
    }
    t.check("number of groups equals the reduction theorem's count (3000 random border sets)", bad.is_none());

    println!("explanations built without searching");
    // Search limit 0 forces every explanation to be derived instead of
    // searched for. The answer must not change, and the derived chain must
    // be a valid proof between the same two positions, without repeats.
    let derived_ok = |n: usize, alphabet: &[u8], cons: &[Con]| -> bool {
        match (solve(n, alphabet, cons), solve_with(n, alphabet, cons, 0)) {
            (Ok(Outcome::Unsat(short)), Ok(Outcome::Unsat(d))) => {
                let mut seen = d.path.clone();
                seen.sort_unstable();
                seen.dedup();
                check_conflict(&d, n, cons)
                    && seen.len() == d.path.len()
                    && d.path.len() >= short.path.len()
                    && d.path[0] == short.path[0]
                    && d.path.last() == short.path.last()
                    && (d.letter_a, d.src_a, d.letter_b, d.src_b) == (short.letter_a, short.src_a, short.letter_b, short.src_b)
            }
            (a, b) => a == b, // solvable or rejected: identical
        }
    };
    let mut rng = Rng(4242);
    let mut bad: Option<(usize, Vec<Con>)> = None;
    let mut contradictions = 0;
    for _ in 0..4000 {
        let n = 2 + rng.below(40);
        let mut cons: Vec<Con> = (0..rng.below(10)).map(|_| Con::Border(rng.below(n + 1))).collect();
        for _ in 0..1 + rng.below(3) {
            let len = 1 + rng.below(n.min(4));
            let w: Vec<u8> = (0..len).map(|_| ab[rng.below(2)]).collect();
            cons.push(if rng.below(2) == 0 { Con::Prefix(w) } else { Con::Suffix(w) });
        }
        for i in (1..cons.len()).rev() {
            cons.swap(i, rng.below(i + 1));
        }
        if matches!(solve(n, ab, &cons), Ok(Outcome::Unsat(_))) {
            contradictions += 1;
        }
        if !derived_ok(n, ab, &cons) {
            bad = Some((n, cons));
            break;
        }
    }
    if let Some(inst) = &bad {
        println!("        counterexample: {inst:?}");
    }
    t.check(
        &format!("derived explanations are valid proofs ({contradictions} random contradictions)"),
        bad.is_none() && contradictions > 1000,
    );
    let mut all_ok = true;
    for n in 1..=8usize {
        for mask in 0u32..(1 << (n + 1)) {
            let mut cons: Vec<Con> = (0..=n).filter(|k| mask >> k & 1 == 1).map(b).collect();
            cons.push(p("a"));
            cons.push(s("b"));
            all_ok &= derived_ok(n, ab, &cons);
        }
    }
    t.check("derived explanations: every set of borders up to length 8", all_ok);
    // Periods 4, 8, ..., 36 and 2 on a text of 40: positions 0 and 38 are
    // two links apart. The derived chain walks through the even positions;
    // the shortening pass must bring it back to two links.
    let cons: Vec<Con> = (1..10).map(|j| b(40 - 4 * j)).chain([b(38), p("a"), s("ba")]).collect();
    let short = solve(40, ab, &cons).unwrap();
    let derived = solve_with(40, ab, &cons, 0).unwrap();
    let raw = derive_path(40, &cons, 0, 38).map_or(0, |x| x.0.len());
    t.check(
        "a long derived chain is shortened when a direct link exists",
        matches!((&short, &derived), (Outcome::Unsat(a), Outcome::Unsat(d))
            if a.path.len() == 3 && d.path.len() == 3 && check_conflict(d, 40, &cons))
            && raw == 20,
    );
    t.check(
        "positions that are not forced equal get no chain",
        derive_path(10, &[b(8)], 0, 1).is_none() && derive_path(10, &[b(8)], 0, 8).is_some() && derive_path(10, &[], 3, 3).is_some(),
    );
    // A search limit only changes which valid chain is shown, never the answer.
    let cons = [b(6), b(4), p("a"), s("b")];
    t.check(
        "a generous search limit gives the shortest chain, as before",
        matches!(solve_with(7, ab, &cons, u64::MAX), Ok(Outcome::Unsat(cf)) if cf.path == vec![0, 3, 6])
            && solve_with(7, ab, &cons, u64::MAX) == solve(7, ab, &cons),
    );

    println!("reports");
    let plain = Options { compact: false, full: false, help: false, certificate: false };
    let compact = Options { compact: true, ..plain };
    let demo1 = [b(5), p("ab"), s("b")];
    let demo2 = [b(4), p("a"), s("b")];
    t.check(
        "compact report, several solutions (original format)",
        report(8, ab, &demo1, compact).as_deref()
            == Ok("n = 8, alphabet = ab, constraints = [Border(5), Prefix(ab), Suffix(b)]\n\
                   status   MULTIPLE\n\
                   pattern  a b ?0 a b ?0 a b\n\
                   free     1 class(es)\n\
                   count    2\n\
                   witness  abaabaab\n\
                   checked  witness satisfies definition: true\n"),
    );
    t.check(
        "compact report, no solution (original format)",
        report(7, ab, &demo2, compact).as_deref()
            == Ok("n = 7, alphabet = ab, constraints = [Border(4), Prefix(a), Suffix(b)]\n\
                   status   UNSAT\n\
                   reason   position 0 must be 'a' (Prefix(a)) and position 6 must be 'b' (Suffix(b))\n\
                   \x20        position 0 = position 3 by Border(4)\n\
                   \x20        position 3 = position 6 by Border(4)\n\
                   checked  certificate valid: true\n"),
    );
    let has = |out: &Result<String, String>, parts: &[&str]| -> bool {
        out.as_ref().map_or(false, |o| parts.iter().all(|x| o.contains(x)))
    };
    t.check(
        "plain report, several solutions",
        has(
            &report(8, ab, &demo1, plain),
            &[
                "1. The first 5 characters are the same as the last 5 characters.",
                "2. The text starts with \"ab\".",
                "Result: SEVERAL SOLUTIONS - 2 different texts fit all the rules.",
                "Pattern  : a b ?1 a b ?1 a b",
                "Example  : abaabaab",
                "?1 = characters 3, 6",
                "every rule: confirmed.",
            ],
        ),
    );
    t.check(
        "plain report, one solution",
        has(
            &report(6, ab, &[b(4), p("ab")], plain),
            &["Result: ONE SOLUTION", "The text : ababab", "every rule: confirmed."],
        ),
    );
    t.check(
        "plain report, no solution (positions counted from 1)",
        has(
            &report(7, ab, &demo2, plain),
            &[
                "Result: NO SOLUTION",
                "Rule 2 needs character 1 to be \"a\".",
                "Rule 3 needs character 7 to be \"b\".",
                "character 1 = character 4   (rule 1)",
                "character 4 = character 7   (rule 1)",
                "against the rules: confirmed.",
            ],
        ),
    );
    t.check(
        "plain report, direct clash on one character",
        has(
            &report(3, ab, &[p("ab"), s("aa")], plain),
            &["Rule 1 needs character 2 to be \"b\".", "Rule 2 needs that same character to be \"a\"."],
        ),
    );
    let long = report(300, ab, &[b(50)], plain);
    let long_full = report(300, ab, &[b(50)], Options { full: true, ..plain });
    t.check(
        "long answers are shortened unless --full",
        has(&long, &["2^250 different texts", "(240 characters not shown)", "Add --full"])
            && has(&long_full, &["?50 = characters 50, 300", "?250 = character 250\n"])
            && !has(&long_full, &["not shown"])
            && !has(&long_full, &["Add --full"]),
    );
    t.check("large counts get thousands separators", commas(1234567) == "1,234,567" && commas(999) == "999");
    let argv: Vec<String> = ["--full", "8", "ab", "B5", "--compact"].iter().map(|x| x.to_string()).collect();
    let (o, rest) = split_options(&argv);
    t.check(
        "options are separated from the puzzle",
        o == Options { compact: true, full: true, help: false, certificate: false }
            && parse_args(&rest) == Ok((8, b"ab".to_vec(), vec![b(5)])),
    );
    t.check(
        "unreadable puzzles are refused",
        parse_args(&[]).is_err()
            && parse_args(&["x".to_string(), "ab".to_string()]).is_err()
            && parse_args(&["3".to_string(), "ab".to_string(), "Bx".to_string()]).is_err()
            && parse_args(&["3".to_string(), "ab".to_string(), "Qab".to_string()]).is_err(),
    );

    let strs = |v: &[&str]| -> Vec<String> { v.iter().map(|x| x.to_string()).collect() };
    t.check(
        "numbers must be plain digits",
        parse_args(&strs(&["+3", "ab"])).is_err()
            && parse_args(&strs(&["3", "ab", "B+2"])).is_err()
            && parse_args(&strs(&["3", "ab", "B-1"])).is_err()
            && parse_args(&strs(&["3", "ab", "B02"])) == Ok((3, b"ab".to_vec(), vec![b(2)])),
    );
    let (o, rest) = split_options(&strs(&["3", "-h"]));
    t.check(
        "'-h' can be used as the allowed letters",
        !o.help && parse_args(&rest) == Ok((3, b"-h".to_vec(), vec![])),
    );

    // The reduction computed without the text (docs/research/06).
    {
        let mut rng = Rng(2026);
        let (mut same, mut bound, mut count_ok, mut gen_ok) = (true, true, true, true);
        let mut most = 0;
        for round in 0..4000 {
            let n = 1 + rng.below(if round % 8 == 0 { 3000 } else { 200 });
            let m = rng.below(if round % 5 == 0 { 40 } else { 7 });
            let cons: Vec<Con> = (0..m)
                .map(|_| match round % 4 {
                    0 => b(n - 1 - rng.below(n.min(12))), // small shifts
                    1 => b(rng.below(n.min(12) + 1)),      // short borders
                    _ => b(rng.below(n + 1)),
                })
                .collect();
            let phases = reduction_phases(n, &cons);
            most = most.max(phases.len());
            same &= phases == Derivation::new(n, &cons).phases;
            bound &= phases.len() <= 2 * (usize::BITS - 1 - n.leading_zeros()) as usize + 2;
            let root = equal_positions(n, &cons).unwrap();
            let mut distinct = root.clone();
            distinct.sort_unstable();
            distinct.dedup();
            count_ok &= groups_from_phases(n, &phases) == distinct.len();
            // groups made by the links of the generator borders alone
            let mut group: Vec<usize> = (0..n).collect();
            fn top(group: &mut [usize], mut x: usize) -> usize {
                while group[x] != x {
                    group[x] = group[group[x]];
                    x = group[x];
                }
                x
            }
            for (q, _) in generator_periods(n, &cons) {
                for i in 0..n - q {
                    let (x, y) = (top(&mut group, i), top(&mut group, i + q));
                    if x != y {
                        group[x.max(y)] = x.min(y); // smallest position on top
                    }
                }
            }
            gen_ok &= (0..n).all(|i| top(&mut group, i) == root[i]);
        }
        t.check("the phases computed without the text are the phases of the grouping (4000 border sets)", same);
        t.check(&format!("never more than 2*log2(n)+2 phases (most seen: {most})"), bound);
        t.check("number of groups from the phases alone equals the grouping's", count_ok);
        t.check("the links of the generator borders alone give exactly the groups", gen_ok);
        // explanations through generators: valid on puzzles, whatever the search limit
        let mut valid = true;
        for _ in 0..3000 {
            let (n, alphabet, cons) = random_instance(&mut rng, 40);
            if let Ok(Outcome::Unsat(cf)) = solve_with(n, &alphabet, &cons, 0) {
                valid &= check_conflict(&cf, n, &cons);
                let (path, via) = generator_path(n, &cons, cf.path[0], *cf.path.last().unwrap()).unwrap();
                let probe = Conflict { path, via, ..cf.clone() };
                valid &= check_conflict(&probe, n, &cons);
            }
        }
        t.check("explanations through generator borders pass the checker (search limit 0)", valid);
    }

    // The linear-time double-check agrees with the direct one.
    {
        let mut rng = Rng(77);
        let mut same = true;
        let mut yes = 0;
        for round in 0..20_000 {
            let n = rng.below(if round % 10 == 0 { 400 } else { 30 });
            let letters: &[u8] = if round % 3 == 0 { b"a" } else { b"ab" };
            // a text with some period, sometimes damaged, sometimes with a foreign letter
            let p = 1 + rng.below(n.max(1).min(6));
            let mut text_: Vec<u8> = (0..n).map(|i| letters[(i % p) * 3 % letters.len()]).collect();
            if n > 0 && round % 4 == 0 {
                let at = rng.below(n);
                text_[at] = if round % 8 == 0 { b'z' } else { b'a' };
            }
            let cons: Vec<Con> = (0..rng.below(6))
                .map(|_| match rng.below(5) {
                    0 => Con::Prefix(text_[..rng.below(n + 1)].to_vec()),
                    1 => Con::Suffix(text_[n - rng.below(n + 1)..].to_vec()),
                    2 => Con::Prefix(vec![b'b'; rng.below(3)]),
                    _ => Con::Border(rng.below(n + 2)),
                })
                .collect();
            // Border(k) with k > n is rejected before solving; both checks must also agree on it,
            // so compare on rules the direct check can evaluate
            let ok_rules = cons.iter().all(|c| !matches!(c, Con::Border(k) if *k > n));
            let direct = ok_rules && satisfies(&text_, n, letters, &cons);
            yes += direct as usize;
            same &= satisfies_fast(&text_, n, letters, &cons) == direct;
            same &= !satisfies_fast(&text_, n + 1, letters, &cons);
        }
        t.check(&format!("the linear-time double-check agrees with the direct one (20000 texts, {yes} satisfying)"), same && yes > 1000);
    }

    // The test above never has enough border length to leave the direct
    // comparison (at most 5 rules, so the cost stays below 16 n). Here every
    // case has 40 or more long borders, so the border table is really used.
    {
        let mut rng = Rng(4242);
        let (mut same, mut yes, mut no, mut table_cases) = (true, 0, 0, 0);
        for round in 0..3_000 {
            let n = 40 + rng.below(360);
            let letters: &[u8] = if round % 5 == 0 { b"abc" } else { b"ab" };
            let p = 1 + rng.below(8);
            // a text with period p (its first p letters repeated)
            let seed: Vec<u8> = (0..p).map(|_| letters[rng.below(letters.len())]).collect();
            let mut text_: Vec<u8> = (0..n).map(|i| seed[i % p]).collect();
            if round % 3 == 0 {
                let at = rng.below(n);
                text_[at] = if text_[at] == b'a' { b'b' } else { b'a' }; // damaged copy
            }
            // long borders: with shift a multiple of p (true for an undamaged
            // text) or arbitrary (often false); 40 to 59 of them
            let mut cons: Vec<Con> = (0..40 + rng.below(20))
                .map(|_| {
                    let shift = if round % 2 == 0 { p * (1 + rng.below((n / 2) / p)) } else { 1 + rng.below(n / 2) };
                    Con::Border(n - shift)
                })
                .collect();
            if round % 4 == 0 {
                cons.push(Con::Prefix(text_[..rng.below(n + 1)].to_vec()));
                cons.push(Con::Suffix(text_[n - rng.below(n + 1)..].to_vec()));
            }
            let cost: usize = cons.iter().map(|c| if let Con::Border(k) = c { *k } else { 0 }).sum();
            table_cases += (cost > 16 * n) as usize;
            let direct = satisfies(&text_, n, letters, &cons);
            if direct { yes += 1 } else { no += 1 }
            same &= satisfies_fast(&text_, n, letters, &cons) == direct;
        }
        t.check(
            &format!("the border-table double-check agrees with the direct one on many long borders ({table_cases} table cases: {yes} satisfying, {no} not)"),
            same && table_cases == 3_000 && yes > 500 && no > 500,
        );
    }

    println!("\n{} passed, {} failed", t.passed, t.failed);
    t.failed == 0
}

// ------------------------------------------------------------ reporting

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn describe(c: &Con) -> String {
    match c {
        Con::Prefix(w) => format!("Prefix({})", text(w)),
        Con::Suffix(w) => format!("Suffix({})", text(w)),
        Con::Border(k) => format!("Border({k})"),
    }
}

/// The original terse, technical report (positions counted from 0).
/// Kept unchanged for scripts and for people who prefer it: --compact.
fn report_compact(n: usize, alphabet: &[u8], cons: &[Con]) -> Result<String, String> {
    use std::fmt::Write;
    let mut o = String::new();
    let list: Vec<String> = cons.iter().map(describe).collect();
    let _ = writeln!(o, "n = {n}, alphabet = {}, constraints = [{}]", text(alphabet), list.join(", "));
    match solve(n, alphabet, cons)? {
        Outcome::Unsat(cf) => {
            let _ = writeln!(o, "status   UNSAT");
            let _ = writeln!(
                o,
                "reason   position {} must be '{}' ({}) and position {} must be '{}' ({})",
                cf.path[0],
                cf.letter_a as char,
                describe(&cons[cf.src_a]),
                cf.path.last().unwrap(),
                cf.letter_b as char,
                describe(&cons[cf.src_b]),
            );
            for (pair, idx) in cf.path.windows(2).zip(&cf.via) {
                let _ = writeln!(o, "         position {} = position {} by {}", pair[0], pair[1], describe(&cons[*idx]));
            }
            let _ = writeln!(o, "checked  certificate valid: {}", check_conflict(&cf, n, cons));
        }
        Outcome::Sat { pattern, free } => {
            let shown: Vec<String> = pattern
                .iter()
                .map(|c| match c {
                    Cell::Letter(ch) => (*ch as char).to_string(),
                    Cell::Free(i) => format!("?{i}"),
                })
                .collect();
            let status = if is_unique(alphabet.len(), free) { "UNIQUE" } else { "MULTIPLE" };
            let _ = writeln!(o, "status   {status}");
            let _ = writeln!(o, "pattern  {}", shown.join(" "));
            let _ = writeln!(o, "free     {free} class(es)");
            match count(alphabet.len(), free) {
                Some(c) => {
                    let _ = writeln!(o, "count    {c}");
                }
                None => {
                    let _ = writeln!(o, "count    {}^{free}", alphabet.len());
                }
            }
            let w = witness(&pattern, free, alphabet);
            let _ = writeln!(o, "witness  {}", text(&w));
            let _ = writeln!(o, "checked  witness satisfies definition: {}", satisfies_fast(&w, n, alphabet, cons));
        }
    }
    Ok(o)
}

// ---- plain-English report (the default). Positions are counted from 1.

/// How much of a long answer to print before abbreviating (unless --full).
const SHOW_HEAD: usize = 40;
const SHOW_TAIL: usize = 20;
const SHOW_GROUPS: usize = 10;
const SHOW_MEMBERS: usize = 8;
const SHOW_STEPS_HEAD: usize = 6;
const SHOW_STEPS_TAIL: usize = 3;

fn chars(k: usize) -> String {
    if k == 1 { "1 character".into() } else { format!("{} characters", commas(k as u128)) }
}

/// 1234567 -> "1,234,567"
fn commas(v: u128) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(d);
    }
    out
}

/// One rule as an English sentence.
fn rule_sentence(c: &Con, n: usize) -> String {
    match c {
        Con::Prefix(w) if w.is_empty() => "The text starts with an empty word (always true).".into(),
        Con::Suffix(w) if w.is_empty() => "The text ends with an empty word (always true).".into(),
        Con::Prefix(w) => format!("The text starts with \"{}\".", text(w)),
        Con::Suffix(w) => format!("The text ends with \"{}\".", text(w)),
        Con::Border(0) => "The first 0 characters match the last 0 characters (always true).".into(),
        Con::Border(k) if *k == n => {
            format!("The first {} match the last {} (always true: that is the whole text).", chars(*k), chars(*k))
        }
        Con::Border(1) => "The first character is the same as the last character.".into(),
        Con::Border(k) => format!("The first {k} characters are the same as the last {k} characters."),
    }
}

/// Join `len` items with `sep`; if there are too many, keep the start and
/// the end. Items are produced on demand, so a shortened answer never
/// builds the part that is not shown.
fn abbreviate(len: usize, item: impl Fn(usize) -> String, sep: &str, full: bool, unit: &str) -> String {
    let join = |range: std::ops::Range<usize>| range.map(&item).collect::<Vec<String>>().join(sep);
    if full || len <= SHOW_HEAD + SHOW_TAIL {
        return join(0..len);
    }
    let hidden = len - SHOW_HEAD - SHOW_TAIL;
    format!(
        "{}{sep}... ({} {unit} not shown) ...{sep}{}",
        join(0..SHOW_HEAD),
        commas(hidden as u128),
        join(len - SHOW_TAIL..len)
    )
}

fn show_text(s: &[u8], full: bool) -> String {
    if s.is_empty() {
        return "(the empty text: zero characters)".into();
    }
    abbreviate(s.len(), |i| (s[i] as char).to_string(), "", full, "characters")
}

fn how_many_texts(alphabet_len: usize, free: usize) -> String {
    match count(alphabet_len, free) {
        Some(c) => format!("{} different texts fit all the rules.", commas(c)),
        None => format!(
            "{alphabet_len}^{free} different texts fit all the rules (that is {alphabet_len} multiplied by itself {} times - too large to write out).",
            commas(free as u128)
        ),
    }
}

fn report_plain(n: usize, alphabet: &[u8], cons: &[Con], full: bool) -> Result<String, String> {
    use std::fmt::Write;
    let outcome = solve(n, alphabet, cons)?;
    let mut o = String::new();
    let mut abbreviated = false;

    let _ = writeln!(o, "The puzzle");
    let _ = writeln!(o, "  Length of the text : {}", chars(n));
    let letters: Vec<String> = alphabet.iter().map(|ch| (*ch as char).to_string()).collect();
    let _ = writeln!(o, "  Allowed letters    : {}", letters.join(" "));
    if cons.is_empty() {
        let _ = writeln!(o, "  Rules              : none");
    } else {
        let _ = writeln!(o, "  Rules");
        for (i, c) in cons.iter().enumerate() {
            let _ = writeln!(o, "    {}. {}", i + 1, rule_sentence(c, n));
        }
    }
    let _ = writeln!(o);

    match outcome {
        Outcome::Unsat(cf) => {
            let _ = writeln!(o, "Result: NO SOLUTION - no text can fit all the rules; they contradict each other.");
            let _ = writeln!(o);
            let _ = writeln!(o, "  Why");
            let first = cf.path[0] + 1;
            let last = cf.path[cf.path.len() - 1] + 1;
            let _ = writeln!(o, "    Rule {} needs character {first} to be \"{}\".", cf.src_a + 1, cf.letter_a as char);
            if cf.path.len() == 1 {
                let _ = writeln!(o, "    Rule {} needs that same character to be \"{}\".", cf.src_b + 1, cf.letter_b as char);
            } else {
                let _ = writeln!(o, "    Rule {} needs character {last} to be \"{}\".", cf.src_b + 1, cf.letter_b as char);
                let _ = writeln!(o, "    But those two characters must hold the same letter, because:");
                // Format only the steps that will be printed: an explanation
                // can have as many steps as the text has characters.
                let total = cf.via.len();
                let step = |i: usize| {
                    format!("character {} = character {}   (rule {})", cf.path[i] + 1, cf.path[i + 1] + 1, cf.via[i] + 1)
                };
                if full || total <= SHOW_STEPS_HEAD + SHOW_STEPS_TAIL {
                    for i in 0..total {
                        let _ = writeln!(o, "      {}", step(i));
                    }
                } else {
                    abbreviated = true;
                    for i in 0..SHOW_STEPS_HEAD {
                        let _ = writeln!(o, "      {}", step(i));
                    }
                    let hidden = total - SHOW_STEPS_HEAD - SHOW_STEPS_TAIL;
                    let _ = writeln!(o, "      ... ({} more steps of the same kind) ...", commas(hidden as u128));
                    for i in total - SHOW_STEPS_TAIL..total {
                        let _ = writeln!(o, "      {}", step(i));
                    }
                }
            }
            let _ = writeln!(o);
            let verdict = if check_conflict(&cf, n, cons) { "confirmed" } else { "FAILED - please report this" };
            let _ = writeln!(o, "  Double-check: each step of this explanation was re-checked against the rules: {verdict}.");
        }
        Outcome::Sat { pattern, free } => {
            let example = witness(&pattern, free, alphabet);
            let verdict = if satisfies_fast(&example, n, alphabet, cons) { "confirmed" } else { "FAILED - please report this" };
            abbreviated |= !full && n > SHOW_HEAD + SHOW_TAIL;
            if is_unique(alphabet.len(), free) {
                let _ = writeln!(o, "Result: ONE SOLUTION - exactly one text fits all the rules.");
                let _ = writeln!(o);
                let _ = writeln!(o, "  The text : {}", show_text(&example, full));
                let _ = writeln!(o);
                let _ = writeln!(o, "  Double-check: this text was tested directly against every rule: {verdict}.");
            } else {
                let _ = writeln!(o, "Result: SEVERAL SOLUTIONS - {}", how_many_texts(alphabet.len(), free));
                let _ = writeln!(o);
                let cell = |i: usize| match pattern[i] {
                    Cell::Letter(ch) => (ch as char).to_string(),
                    Cell::Free(g) => format!("?{}", g + 1),
                };
                let _ = writeln!(o, "  Pattern  : {}", abbreviate(n, cell, " ", full, "characters"));
                let _ = writeln!(o, "  Example  : {}", show_text(&example, full));
                let _ = writeln!(o);
                let _ = writeln!(o, "  How to read the pattern (one entry per character, left to right)");
                let _ = writeln!(o, "    A plain letter is forced by the rules.");
                let _ = writeln!(o, "    ?1, ?2, ... are open choices. Each can be any allowed letter, but");
                let _ = writeln!(o, "    every place with the same number must get the same letter.");
                let _ = writeln!(o);
                let _ = writeln!(
                    o,
                    "  Open choices: {} (each has {} possible letters)",
                    commas(free as u128),
                    alphabet.len()
                );
                // Where each open choice sits (first few places of the first few choices).
                let listed = if full { free } else { free.min(SHOW_GROUPS) };
                let keep = if full { usize::MAX } else { SHOW_MEMBERS };
                let mut members: Vec<Vec<usize>> = vec![Vec::new(); listed];
                let mut total = vec![0usize; listed];
                for (i, c) in pattern.iter().enumerate() {
                    if let Cell::Free(g) = c {
                        if *g < listed {
                            total[*g] += 1;
                            if members[*g].len() < keep {
                                members[*g].push(i + 1);
                            }
                        }
                    }
                }
                for g in 0..listed {
                    let places: Vec<String> = members[g].iter().map(|p| p.to_string()).collect();
                    let more = total[g] - members[g].len();
                    let tail = if more > 0 {
                        abbreviated = true;
                        format!(", ... ({} places in total)", commas(total[g] as u128))
                    } else {
                        String::new()
                    };
                    let noun = if total[g] == 1 { "character" } else { "characters" };
                    let _ = writeln!(o, "    ?{} = {noun} {}{tail}", g + 1, places.join(", "));
                }
                if listed < free {
                    abbreviated = true;
                    let _ = writeln!(o, "    ... and {} more open choices", commas((free - listed) as u128));
                }
                let _ = writeln!(o);
                let _ = writeln!(o, "  Double-check: the example was tested directly against every rule: {verdict}.");
            }
        }
    }
    if abbreviated {
        let _ = writeln!(o);
        let _ = writeln!(o, "  (Long output was shortened. Add --full to see everything.)");
    }
    Ok(o)
}

const USAGE: &str = "\
How to use
  sh prefsuf.rs LENGTH LETTERS [RULE]...     solve one puzzle
  sh prefsuf.rs                              run the built-in self-tests and two demo puzzles

  LENGTH   how many characters the unknown text has, e.g. 8
  LETTERS  the letters the text may use, written together, e.g. ab
  RULE     any number of:
             Pword   the text starts with 'word'             e.g. Pab
             Sword   the text ends with 'word'               e.g. Sb
             Bk      the first k characters are the same as
                     the last k characters                   e.g. B5

Options
  --full      do not shorten long answers
  --compact   short technical output (positions counted from 0)
  --certificate
              print a certificate of the answer instead of the report, for
              the separate checker:  sh prefsuf.rs --certificate 8 ab B5 Pab Sb > c.txt
                                     sh certcheck.rs c.txt
  --help      show this message

Example
  sh prefsuf.rs 8 ab B5 Pab Sb";

#[derive(Clone, Copy, Debug, PartialEq)]
struct Options {
    compact: bool,
    full: bool,
    help: bool,
    certificate: bool,
}

/// Separate the --options from the puzzle description.
fn split_options(args: &[String]) -> (Options, Vec<String>) {
    let mut opts = Options { compact: false, full: false, help: false, certificate: false };
    let mut rest = Vec::new();
    for a in args {
        match a.as_str() {
            "--compact" => opts.compact = true,
            "--full" => opts.full = true,
            "--help" => opts.help = true,
            "--certificate" => opts.certificate = true,
            _ => rest.push(a.clone()),
        }
    }
    (opts, rest)
}

/// Digits only: no sign, no spaces, and small enough for this machine.
fn whole_number(s: &str) -> Option<usize> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn parse_args(args: &[String]) -> Result<(usize, Vec<u8>, Vec<Con>), String> {
    if args.len() < 2 {
        return Err("please give at least the text length and the allowed letters".into());
    }
    let n = whole_number(&args[0]).ok_or_else(|| {
        if !args[0].is_empty() && args[0].bytes().all(|b| b.is_ascii_digit()) {
            format!("the text length {} is too large for this computer", args[0])
        } else {
            format!("the text length must be a whole number, got '{}'", args[0])
        }
    })?;
    let alphabet = args[1].as_bytes().to_vec();
    let mut cons = Vec::new();
    for a in &args[2..] {
        let bytes = a.as_bytes();
        let rest = bytes.get(1..).unwrap_or(&[]);
        match bytes.first() {
            Some(b'P') => cons.push(Con::Prefix(rest.to_vec())),
            Some(b'S') => cons.push(Con::Suffix(rest.to_vec())),
            Some(b'B') => {
                let k = whole_number(&a[1..])
                    .ok_or_else(|| format!("'{a}': B must be followed by a whole number, e.g. B5"))?;
                cons.push(Con::Border(k));
            }
            _ if a.starts_with("--") => {
                return Err(format!("'{a}' is not a known option (the options are --full, --compact, --certificate and --help)"))
            }
            _ => return Err(format!("'{a}' is not a rule (rules look like Pword, Sword or Bk)")),
        }
    }
    Ok((n, alphabet, cons))
}

fn report(n: usize, alphabet: &[u8], cons: &[Con], opts: Options) -> Result<String, String> {
    if opts.certificate {
        certificate(n, alphabet, cons)
    } else if opts.compact {
        report_compact(n, alphabet, cons)
    } else {
        report_plain(n, alphabet, cons, opts.full)
    }
}

/// Write the answer. If it cannot be written (closed pipe, full disk) say so
/// on stderr and exit 2 instead of aborting with a panic.
fn emit(out: &str) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    if let Err(e) = stdout.write_all(out.as_bytes()).and_then(|_| stdout.flush()) {
        eprintln!("could not write the answer: {e}");
        exit(2);
    }
}

fn main() {
    // An argument that is not valid UTF-8 cannot be a length, a letter list
    // or a rule: report it like any other unreadable puzzle.
    let mut raw: Vec<String> = Vec::new();
    for a in std::env::args_os().skip(1) {
        match a.into_string() {
            Ok(s) => raw.push(s),
            Err(bad) => {
                eprintln!("Sorry, that puzzle could not be read: the argument {bad:?} is not valid text.\n\n{USAGE}");
                exit(2);
            }
        }
    }
    let (opts, args) = split_options(&raw);
    if opts.help {
        println!("{USAGE}");
        return;
    }
    if args.is_empty() {
        println!("== self-tests ==");
        let ok = run_tests();
        println!("\n== demo 1 ==  (same as: sh prefsuf.rs 8 ab B5 Pab Sb)\n");
        if let Ok(out) = report(8, b"ab", &[b(5), p("ab"), s("b")], opts) {
            print!("{out}");
        }
        println!("\n== demo 2 ==  (same as: sh prefsuf.rs 7 ab B4 Pa Sb)\n");
        if let Ok(out) = report(7, b"ab", &[b(4), p("a"), s("b")], opts) {
            print!("{out}");
        }
        println!("\nTo solve your own puzzle, see: sh prefsuf.rs --help");
        exit(if ok { 0 } else { 1 });
    }
    match parse_args(&args).and_then(|(n, alphabet, cons)| report(n, &alphabet, &cons, opts)) {
        Ok(out) => emit(&out),
        Err(e) => {
            eprintln!("Sorry, that puzzle could not be read: {e}.\n\n{USAGE}");
            exit(2);
        }
    }
}
