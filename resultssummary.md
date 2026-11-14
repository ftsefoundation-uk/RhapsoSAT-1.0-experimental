# Results summary

> **Note for this public repository.** The commits, tags and branches named
> below (`1b3e19d`, `4f8d4f4`, `185ffd1`, `ff5e6f1`, `baseline-proof-study`,
> `proof-study`, `main`) belong to the development history this repository
> was released from; they are not part of this repository, so the
> `git checkout` commands cannot be run here. This repository was released
> with the solver as it stood at the end of the last part,
> "Proof-construction study"; later changes are in this repository's own
> history. The report is otherwise left as written.

This file records the adversarial investigation of `prefsuf.rs` and the
answer to a follow-up question about NP-completeness. The fixes described
here are in commit `4f8d4f4` on `main`. The version that was tested first,
before any fix, is commit `1b3e19d`.

The adversarial testing found no wrong answers, but it did find real scaling
failures and several small bugs; all but one are now fixed.

**Update.** The one failure left open below (number 4, slow proof search on
dense border sets) was investigated afterwards. The result is in the last
part of this file, "Proof-construction study": on branch `proof-study`
(commit `185ffd1`) the slow test runs in 0.38 s instead of 31.3 s with the
same proof. The first two parts of this file describe the state at commit
`4f8d4f4` and are left as written.

# Adversarial report

## 1. What was tested

`adversarial.rs` trusts nothing in the solver. It has its own brute-force
enumerator, its own proof checker, and its own grouping method (breadth-first
search over the links). It was run against the unmodified pushed solver
first, then against the fixed one.

| Section | Content | Checks (seed 1) |
|---|---|---:|
| A. Nearly contradictory borders | Every subset of n−1…n−4 with short words, brute-forced to length 11; large texts up to 2,000,000 with period pairs differing by 1–3 | 21,246 |
| B. Fine–Wilf boundary | All period pairs up to 9 by brute force; random pairs with chosen common divisor at bound−1, bound, bound+1, up to about 2.6 million characters | 2,416 |
| C. Maximum distinct borders | Every border, plus 12 other families, in ascending, descending, shuffled and repeated order; up to 19,999 borders on 20,000 characters | 1,211 |
| D. Extreme-distance contradictions | Chains, and coprime periods exactly at the bound, where the only proof visits every character | 852 |
| E. Maximum free choices | Up to 5,000,000 free characters; the count function against exact arithmetic for all alphabet sizes | 412 |
| F. Interconnected constraints | Every border subset to length 9; 60,000 random brute-force puzzles; planted and corrupted puzzles to 30,000 characters | 89,132 |
| G. Boundary and malformed input | Empty text, length 1, alphabets of 1 and 94 letters, invalid input, forged proofs | 45 |

Also run: 34 command-line boundary cases, and a scaling matrix of 19 shapes
at seven sizes (10,000; 50,000; 100,000; 500,000; 1,000,000; 2,000,000;
5,000,000), one process each with a 60-second limit.

## 2. What passed

- **Unmodified solver:** 114,755 of 114,756 correctness checks at seed 1
  (section C was smaller at that point).
- **Fixed solver:** all of roughly 115,300 checks at each of seeds 1–5.
- **Existing suite after the changes:** 43/43 self-tests; `hard.rs` at seeds
  1 and 7, and seed 3 up to 5,000,000 characters, all with 0 failures.
- **Against the original archive version:** identical answers and identical
  proofs on 263,669 random puzzles.
- **Every contradiction proof** was validated link by link by the independent
  checker; proof length was compared with an independently computed shortest
  distance where that was affordable.

## 3. What failed

On the unmodified solver:

| # | Kind | Failure | Status |
|---|---|---|---|
| 1 | Performance | Grouping is quadratic when many distinct borders share a common factor: 4.9 s at 100,000 characters, over 60 s at 500,000 | Fixed |
| 2 | Performance | Proof search is quadratic with many borders: 7.3 s at 100,000, over 60 s at 500,000 | Fixed for these shapes |
| 3 | Performance | Proof search checks every border at every position: 15.8 s at 5,000,000 with 3,001 borders | Fixed (0.57 s) |
| 4 | Performance | Proof search on a dense link graph with the target two steps away: 50 s at 500,000 | **Not fixed** (31 s) |
| 5 | Logic | One-letter alphabet with more than about 4.29 billion free groups reports no count instead of 1 | Fixed |
| 6 | Input | `-h` given as the allowed letters prints help instead of solving | Fixed |
| 7 | Input | `B+2` and `+3` are accepted as numbers | Fixed |
| 8 | Crash | An impossible text length panics or aborts (exit 101/134) | Fixed for obvious cases |
| 9 | Memory | The plain report builds every proof step and pattern cell before shortening: 2.3–2.5 GB at 20 million characters | Fixed (0.65–0.8 GB) |

Notes:

- Number 5 needs a text too large for the test machine's memory, so it is a
  logic error that could not occur in practice there.
- Numbers 6 and 7 were introduced by the earlier plain-English output work,
  not by the original program.
- Partway through, the first fix for number 2 made number 4 about five times
  slower than the original (10.7 s versus 1.9 s at 100,000). The harness
  caught it and the final version is faster than the original (1.2 s).

## 4. Largest verified input

- **Fully independent check of the whole pattern:** 5,000,000 characters with
  no border or one border; about 2.6 million characters with two borders;
  20,000 characters with 19,999 borders.
- **Proofs:** 5,000,000-step proofs validated link by link.
- **Not independently verified:** solvable puzzles with millions of borders
  at 5,000,000 characters. Only the number of free groups was compared with
  the value the mathematics predicts (1, 2 or 3). The `verified=true` column
  in the perf output overstates this for those rows.

## 5. Worst runtime and memory

- **Worst completed:** 31 s (failure 4, 500,000 characters, 125,000
  borders); it times out at 1,000,000.
- **Everything else at 5,000,000 characters:** under 0.6 s. Solvable shapes
  finish in under 0.33 s, including 4,999,999 distinct borders.
- **Peak memory at 5,000,000 characters:** 160 MB typical and 469 MB with
  every border; these figures include the harness's own copy of the input.
- **Command line at 20 million characters:** 0.8–1.4 s and 0.65–0.8 GB.

## 6. Mathematical weaknesses

- **No unsound reasoning was found.** The earlier "shortest period first"
  shortcut was correct but only removed redundancy relative to one period,
  which is why failure 1 existed.
- **The new grouping method** rests on "a border of a border is a border":
  take the longest border, link the characters after it to the ones they
  copy, then repeat on the first part alone. Each character is linked at most
  once. The argument is written out in the source and the README; it is
  checked by tests, not formally proven by machine.

## 7. Weaknesses in proof construction

- The solver always searches for the shortest proof by breadth-first search
  over the original links. In the worst case that costs the total number of
  links, which grows with the square of the text length for dense border
  sets (failure 4).
- A proof can be as long as the text. That is inherent: section D contains
  puzzles where no shorter proof exists.

## 8. Are the complexity claims justified?

- **The earlier claim** (proof cost proportional to text length × distinct
  borders) was an honest upper bound, but it hid that grouping could also be
  quadratic. The "every possible border in under 1 ms" figure was flattering:
  that input happens to include the easiest border.
- **Now:** grouping and answering scale linearly in the measurements, from
  10,000 to 5,000,000 characters across all solvable shapes. Proof search is
  linear for few borders and remains quadratic in the worst case.

## 9. Reproducing the failures

Against the first pushed version (`git checkout 1b3e19d`), with the current
`adversarial.rs` copied alongside:

| Failure | Command |
|---|---|
| 1 | `sh adversarial.rs perf evenhalf 500000` and `sh adversarial.rs perf ww3 500000` |
| 2 | `sh adversarial.rs perf allborders_mid_unsat 500000` |
| 3 | `sh adversarial.rs perf chain_tiny3000_unsat 5000000` |
| 4 (still slow today) | `sh adversarial.rs perf mult4_2_unsat 500000` |
| 5 | `sh adversarial.rs check 1 E` |
| 6 | `sh prefsuf.rs 3 -h` |
| 7 | `sh prefsuf.rs 3 ab B+2` |
| 8 | `sh prefsuf.rs 9223372036854775807 ab` |
| 9 | `sh prefsuf.rs 20000000 ab B19999999 Pa Sb` (watch memory) |

The perf shapes are deterministic; the random sections print their seed and
the exact puzzle on any failure.

## 10. Recommendations

1. **Decide whether proofs must be shortest.** A non-shortest proof could
   probably be built far faster, but that has not been designed or tested.
2. **Independent verification at scale with many borders.** The independent
   grouping check costs text length × borders, so it stops at about
   20,000 × 20,000. A second, differently designed fast method would close
   that gap.
3. **Memory.** About 35 bytes per character; 32-bit indices would roughly
   halve it for texts under 4 billion characters.
4. **Machine-check the border-of-border argument** if this is to be relied
   on beyond testing.

## Limits of this testing

- Brute force only reaches length 17.
- Random testing is not proof.
- Everything ran on one machine (Intel Xeon, Linux) and one compiler
  (`rustc 1.100 -O`).
- Timings are single runs.
- Lengths above 20 million were not tried.

# Can the principle extend to an NP-complete problem?

The question asked for a general answer, so the general principles are kept
separate from what the work above actually establishes about this solver.

## General principles

**What makes a particular puzzle easy.** A puzzle can be efficiently solvable
while a similar-looking general problem is NP-complete because of structure
such as:

- **Only equalities and fixed values.** Constraints of the form "these two
  cells are equal" or "this cell is x" can be closed under transitivity in
  near-linear time. Add "these two cells differ" over three or more values
  and you have graph colouring, which is NP-complete.
- **Algebraic structure.** Systems of linear equations (including XOR
  constraints) fall to Gaussian elimination.
- **Restricted clause shapes.** 2-SAT and Horn-SAT are polynomial; 3-SAT is
  not known to be.
- **Tree-like or low-width interaction.** Dynamic programming works when
  constraints interact only locally.
- **Local consistency implying global consistency.** For some constraint
  types, if no small contradiction exists then a full solution does. This is
  rare and is exactly what general problems lack.

**What would have to be true for a technique to apply to an NP-complete
problem.** There would need to be a polynomial-time translation from every
instance of the NP-complete problem into the form the technique handles,
preserving yes/no answers. Typically the translation fails in one of two
ways: the constraints cannot be expressed in the restricted form at all, or
expressing them needs exponentially many pieces.

**Some instances versus every instance.** NP-completeness is a statement
about the worst case over all instances. It does not say typical instances
are hard. Many NP-complete problems have large easy sub-families, and
practical solvers handle huge industrial instances routinely. A technique
that solves a structured subset efficiently says nothing about the instances
outside that subset.

**What would establish a genuine polynomial-time algorithm.** You would need
a precise problem definition, a proof (or accepted citation) that it is
NP-complete, an algorithm correct on every instance, and a proof that its
running time is bounded by a polynomial in the input size. Benchmarks,
however extensive, cannot establish this. Such a result would prove P = NP,
which would be one of the most consequential results in mathematics and
would need independent verification.

## What is established about this solver

These are facts from the work above, not a verdict on generalisation:

- **Formal rules:** unknown text of fixed length over a finite alphabet;
  rules are "starts with w", "ends with w", and "first k characters equal
  last k".
- **Core idea:** every rule is either an equality between two positions or a
  fixed letter at a position. The solver groups equal positions and checks
  that no group gets two letters.
- **Runtime:** finding the answer is near-linear in the input size in the
  measurements; explaining a contradiction is at worst quadratic. Both are
  polynomial.
- **Scope:** it handles every instance of its own puzzle, not a restricted
  subset.

## What cannot be concluded

- Whether any NP-complete problem can be translated into this puzzle's rule
  types. No such reduction was attempted, and the work here gives no
  evidence for one.
- Whether this puzzle with additional rule types (for example "must differ",
  or "contains this word somewhere") stays efficiently solvable. That would
  need its own analysis for each extension.

## Information needed to assess any such claim

1. The puzzle's formal rules, including exactly which constraint types are
   allowed.
2. The solver's core idea, stated precisely enough to prove correct.
3. Its running time as a proven function of input size, not only as
   measured.
4. Whether it handles arbitrary instances or only structurally restricted
   ones.
5. For an extension claim: the explicit translation from the NP-complete
   problem, with a proof that it is correct and polynomial in size.

# Proof-construction study

This part records a later investigation into how contradiction proofs are
built. The baseline is tag `baseline-proof-study` (commit `ff5e6f1`); the
work is on branch `proof-study` (commit `185ffd1`). `main` was not changed
by this study.

In short: the slow test now runs in 0.38 s instead of 31.3 s with the same
3-position proof, by deriving the proof from the grouping step when the
shortest-proof search exceeds a fixed work budget. The derivation's
near-linear cost is measured, not proven.

Environment for all measurements: Intel Xeon (Ivy Bridge), 8 cores, 31 GB,
Linux 7.0, `rustc 1.100 -O`, single runs, wall time and peak memory from
`/usr/bin/time` on a prebuilt binary.

## 1. Why the remaining test was slow

**Baseline measurement.** `sh adversarial.rs perf mult4_2_unsat 500000` on
the untouched baseline: 31.33 s inside the harness (35.1 s wall including
compilation), 24 MB peak, exit 0, proof of 3 positions.

**Where the time goes** (instrumented copy of the search, `proof_lab.rs
trace`):

| Step | Time at 500,000 |
|---|---:|
| Input check | 0.0002 s |
| Grouping | 0.0097 s |
| Letter scan (finds the clash) | 0.0004 s |
| Preparation | 0.0100 s |
| **Shortest-proof search** | **28.13 s** |

**Workload.** 125,000 distinct borders, 31.25 billion links in total, 2
groups, 250,000 positions in the clashing group. The search expanded 125,000
positions, discovered 249,999, and tested 15.625 billion (position, border)
pairs, exactly n²/16.

**Why.** The generator uses periods 4, 8, 12, … and then 2, with the clash
between positions 0 and n−2. The first step of the search reaches every
multiple of 4. The target is only reachable from n−4, which is last in the
queue, and each of the n/4 positions before it tests all n/4 borders and
finds nothing new.

**Scaling (measured).**

| Length | Search time | Pairs tested |
|---:|---:|---:|
| 20,000 | 0.044 s | 25 million |
| 40,000 | 0.175 s | 100 million |
| 80,000 | 0.70 s | 400 million |
| 160,000 | 2.79 s | 1.6 billion |
| 500,000 | 28.1 s | 15.6 billion |

Time quadruples per doubling. At 1,000,000 the baseline exceeded the 60 s
limit; about 125 s is a projection, not a measurement.

**Inherent or avoidable?** The cost is inherent to breadth-first search on
this input, since it must finish each distance level in order. It is not
inherent to producing a proof, and not even to producing this shortest
proof: the two-link chain 0 → n−4 → n−2 is found by one table lookup
(section 3). Whether shortest proofs can always be found faster than
quadratic is open.

## 2. Are shortest proofs necessary?

No. The independent checker (`cert_ok` in `adversarial.rs`) accepts a
certificate when:

- the first position is fixed to one letter by a cited prefix or suffix
  rule;
- the last position is fixed to a different letter by a cited rule;
- every consecutive pair of positions is a link of the cited border rule.

Minimality is not part of validity. Shortness matters only for readability,
and one existing adversarial check asserts it separately (it compares proof
length with an independently computed shortest distance). That check is
unchanged and still passes.

## 3. Are faster valid proofs possible?

Yes. These were tested in `proof_lab.rs`, each judged by the independent
checker:

| Approach | Result |
|---|---|
| Derive the proof from the grouping rounds (first version) | Valid, but **slower than the search**: 362 s at 500,000. Stale borders split one phase into 125,000. |
| Same, dropping stale borders | Valid; 0.038 s at 500,000, but the proof has 250,000 positions instead of 3. |
| Worst-case hunt on that version | **Failed**: work per character grew 14 → 47 → 59 as length grew 400 → 1,600 → 6,400. |
| Early-stopping walk | Work per character 5.3–7.6 from 100 to 102,400 characters, under three usage orders. |
| Shortening pass (jump to the farthest directly linked position, at most 8 lookups per position) | `mult4_2` 1,000,000 → 3 positions; all-borders 2,000,000 → 2 (measured at 2,000,000 characters); 16 random borders 8,485 → 8,331 against a shortest of 8 (weak). |

**How the derivation works.** Consecutive grouping rounds with the same
shift form a phase. Each phase records where its border came from: an
original rule, or the end of an earlier phase. A link made in a phase
unfolds into one original link plus steps through earlier phases. A
union-find over the original links collected so far makes already-joined
claims free. Only links that join two separate groups are kept, so they form
a forest, and the proof is the path through it: no repeated positions, never
longer than the group.

**Proof lengths** (seed 1, 32,814 contradictions): derived proofs total 1.17
million links against 198,000 for shortest; 19,410 are longer; worst ratio
19,999×. After shortening: 756,000 links, 10,063 longer, worst 707×.

**Independent-checker results.** About 131,700 derived certificates across
seeds 1–4, and the solver's own fallback on about 98,700 across seeds 1–3:
all accepted, none rejected or missing. Chains between 178,424 random
position pairs per seed were also all valid.

**Why the search stays first.** Derivation is not always faster: with 16
random borders at 5,000,000 characters it takes 2.9 s against 0.4 s for the
search, and gives a far longer proof.

**What changed in the solver** (branch `proof-study` only):

- The search gets a budget of 200 million link tests plus 32 per character.
  Below it, proofs are exactly the shortest ones, as before.
- If the budget runs out, the proof is derived and then shortened.
- The search's inner loop was rewritten as two plain tests. The first
  version of the budget made it 1.8× slower; the rewrite is faster than the
  baseline and keeps the same order.

**Result on the slow test.**

| Length | Baseline | Now | Peak memory now | Proof |
|---:|---:|---:|---:|---:|
| 100,000 | 1.23 s | 0.31 s | 10 MB | 3 |
| 500,000 | 31.3 s | 0.38 s | 38 MB | 3 |
| 1,000,000 | over 60 s | 0.49 s | 76 MB | 3 |
| 5,000,000 | not attempted | 1.55 s | 362 MB | 3 |

Across all 133 rows of the scaling matrix (19 shapes × 7 sizes), no proof
length changed and no row is more than 30% slower.

## 4. Theoretical complexity

Let n be the text length, m the number of border rules, and K the total
number of links (the sum of border lengths, at most n·m).

- **Grouping (proven).** Each round writes the positions from max(p, k) to
  the end of the current text, then shrinks the text to k. These ranges are
  disjoint, so each position is written at most once in the rounds and once
  more in the final pass. Every round writes at least one position, so there
  are at most n rounds. Sorting costs m log m; the queue sees at most one
  push per phase. Total: O((n + m) log(n + m)).
- **Shortest-proof search (proven).** At most proportional to K; quadratic
  in n is reached (measured above). Now capped by the budget.
- **Derivation, correctness (proven by construction).** Every kept link is
  an original link; the kept links form a forest; the recursion only refers
  to earlier phases, so it terminates.
- **Derivation, cost (not proven).** No bound better than polynomial is
  known. The evidence is the measurements in section 3.
- **Shortening (proven).** At most 8L + 64 lookups for a chain of L
  positions, plus a table of size n.
- **Lower bound.** Some puzzles have no proof shorter than n positions
  (coprime periods exactly at the Fine–Wilf bound), so no method can beat
  linear in output size.

## 5. Can the mathematical technique be generalized?

**The argument, precisely.** If k is the longest border of a text of length
L and p = L − k, then the text has a given set of borders exactly when:

- position j equals position j − p for every j from max(p, k) to L − 1; and
- the first k characters have all the other borders, plus the border k − p
  when k > p.

Two facts make this work. Any shorter border of the whole text is a border
of its first k characters, because the last k characters are a copy of them.
And the part of border k that lies inside the first k characters says that
this prefix repeats with shift p.

**What it is, underneath.** In terms of periods, each round replaces the
period set by "every other period minus p, plus p itself" on a text
shortened by p. That is Euclid's algorithm applied to a set of periods, with
the text length tracking how far the reduction is still valid. The
Fine–Wilf threshold is where that validity runs out.

**Established relatives** (from the literature; not re-derived here):
union-find and congruence closure for equality constraints; proof-producing
congruence closure, which keeps an explanation forest much like the
derivation here; border arrays from string matching; the periodicity lemma.

**What an extension needs.** The constraints must be equalities; they must
be generated by a few shift-like maps, so the links never have to be listed;
and the instance must reduce to a strictly smaller instance of the same
kind, with the work charged to what was removed.

**Where it stops.**

- "Must differ" constraints over three or more letters contain graph
  colouring, which is NP-complete (proven). Over two letters they stay
  polynomial.
- General word equations are a much larger class, known to be NP-hard and
  decidable. This puzzle is a small fragment.
- Rules like "contains this word somewhere" are not equalities between fixed
  positions; they were not analysed.

Nothing here bears on P versus NP: the puzzle is in P and no reduction from
an NP-complete problem was attempted.

## 6. What computational possibilities this might open

These are plausible directions, not demonstrated results:

- **Constraint solving.** When equality constraints come from a small set of
  generators, the closure can be computed in time proportional to the number
  of variables. Here 31 billion links were resolved in 0.01 s of grouping.
- **Certificates from the solving trace.** The grouping rounds already
  contain a justification for every link. Recording origins costs little and
  turns a search problem into an unfolding problem.
- **Formal verification.** A solver that derives its proofs can hand a small
  independent checker a certificate without the checker trusting the solver,
  at a cost close to solving in the cases measured.
- **Shortest versus fast proofs as a setting.** The two can coexist: search
  within a budget, derive beyond it.

## 7. What remains unproven

- A bound on the derivation's cost. The measurements suggest near-linear;
  the hunt is a hill-climb over at most 14 borders and lengths up to
  102,400, so it is weak evidence.
- A bound on the number of phases. The most seen was 18; in principle it
  could be large. The recursion runs on a thread whose stack grows with the
  phase count, and that path is untested at large counts.
- Whether shortest proofs can be found in less than quadratic time for this
  problem.
- The quality of the shortening pass. It is a heuristic: worst observed
  707× the shortest.
- The fallback has not been exercised at large scale on inputs other than
  `mult4_2_unsat`; elsewhere it is tested only by forcing the budget to zero
  on small and medium puzzles (up to 20,000 characters).
- The budget constant was chosen on one machine.
- Fallback memory was measured only on the slow test (362 MB at 5,000,000
  characters).
- None of the mathematical arguments is machine-checked.

## Record of the study

**Files changed** (branch `proof-study`, commit `185ffd1`):

- `prefsuf.rs`: budgeted search, derivation, shortening, 5 new self-tests.
- `proof_lab.rs`: new; instrumentation and experiments.
- `README.md`: explanations are no longer described as always shortest.
- `adversarial.rs`, `hard.rs`, `bench.rs`: unchanged.

**Tests run on the final code:**

| Test | Result |
|---|---|
| Self-tests | 48/48 |
| `adversarial.rs`, seeds 1–5 (about 115,300 checks each) | 0 failures |
| `hard.rs`, seeds 1 and 7; seed 3 to 5,000,000 characters | 0 failures |
| Comparison with the original archive version (263,669 random puzzles) | identical answers and proofs |
| `proof_lab.rs check`, seeds 1–4 | 0 rejected |
| Scaling matrix (19 shapes × 7 sizes) | 133/133 completed and verified |

**To reproduce:**

```sh
git checkout baseline-proof-study && sh adversarial.rs perf mult4_2_unsat 500000   # about 31 s
git checkout proof-study          && sh adversarial.rs perf mult4_2_unsat 500000   # about 0.4 s
sh proof_lab.rs trace mult4_2_unsat 500000    # time breakdown and work counts (about 28 s)
sh proof_lab.rs derive mult4_2_unsat 500000   # derived proof, independent verdict
sh proof_lab.rs check 1                       # derived proofs on about 44,600 puzzles
sh proof_lab.rs hunt 1 6400 62                # search for expensive inputs
```

# Border reduction as a theorem

**Update.** A literature check made after this part was written found that
the theorem below is a known result: it is the Euclid-style reduction for
words with several periods (Holub 2013, Lemma 1, Corollary 1 and Lemma 2;
Algorithm B of Tijdeman and Zamboni). It should be read as a self-contained
statement and proof of a known theorem, not as a new one. Sources, the
generalisation to grids and higher dimensions, and the limits of that
generalisation are in `docs/research/` (notes 01–04).

This part answers the question: can the border-reduction method be stated
as a general theorem about finite sets of shift-generated equality
constraints? Yes, and it is an exact equivalence.

## The objects

Everything here is defined by the code and the earlier parts of this file.

- **Domain.** Positions 0, 1, …, n−1, and a value set A (the alphabet; any
  non-empty set works). An assignment is x = (x₀, …, xₙ₋₁) with values in A.
- **Shifts.** A finite set P of integers with 0 < q < n. Shift q is the
  partial map i ↦ i + q, defined only when i + q < n. Nothing wraps around.
- **Constraints.** E(n, P) is the set of equalities xᵢ = xᵢ₊q for every q in
  P and every i with i + q < n. Sol(n, P) is the set of assignments
  satisfying all of them.
- **Dictionary to the solver.** The rule "first k characters equal last k"
  is the full family for shift q = n − k. The longest border is the smallest
  shift.
- **Border reduction.** Let p = min P and k = n − p. Define
  P′ = { q − p : q in P, q > p }, together with p itself if p < k. The
  reduction replaces (n, P) by (k, P′).

## Theorem

**Hypotheses.** P is non-empty, p = min P, k = n − p, and P′ is as above.

**Statement.** x is in Sol(n, P) if and only if both hold:

- (a) xⱼ = xⱼ₋ₚ for every j with max(p, k) ≤ j < n;
- (b) the restriction (x₀, …, xₖ₋₁) is in Sol(k, P′).

**Corollaries.**

1. Restriction to the first k positions, together with the values at
   positions k, …, p−1 when p > k, is a bijection from Sol(n, P) onto
   Sol(k, P′) × A^max(0, 2p−n).
2. If c(n, P) is the number of groups of positions forced equal, then
   c(n, P) = c(k, P′) + max(0, 2p − n), with c(n, ∅) = n.
3. Every element of P′ is positive and less than k, and |P′| ≤ |P|. So the
   reduction can be repeated; n strictly decreases, and it stops at an empty
   shift set.

## Proof

**Forward.** Assume x is in Sol(n, P).

- (a) is a subset of the shift-p constraints, since j − p ≥ 0 and j < n.
- For (b), take q > p in P and i with i + (q − p) < k, that is, i + q < n.
  The shift-q constraint gives xᵢ = xᵢ₊q. The shift-p constraint at index
  i + q − p gives xᵢ₊q₋ₚ = xᵢ₊q. Hence xᵢ = xᵢ₊q₋ₚ.
- If p < k, the shift-p constraints with i + p < k are among the originals.

**Backward.** Assume (a) and (b).

- *Shift p.* Take i with i + p < n and put j = i + p. If j ≥ max(p, k), (a)
  gives it. Otherwise j < k, which forces p < k, so p is in P′ and (b) gives
  it.
- *Shift q > p.* Take i with i + q < n. Then i + (q − p) < k, so (b) gives
  xᵢ = xᵢ₊q₋ₚ. The shift-p constraint just proved gives xᵢ₊q₋ₚ = xᵢ₊q. ∎

**Why the removed constraints are redundant.** Every shift-q link is the
composition of one link of the reduced system and one shift-p link. The
shift-p links inside the first k positions are exactly the constraint "p is
in P′".

**Corollary 1.** By (a), every position j ≥ max(p, k) is determined by
position j − p, which lies below k. If p > k, positions k, …, p−1 appear in
no constraint: such a position is too small to be the upper end of a link
(all shifts are at least p) and too large to be the lower end
(j + q ≥ k + p = n). So they are free. Corollary 2 follows by counting.

## What kind of result it is

- **An equivalence.** The solution sets correspond one-to-one, for every
  value set A. It is not a one-way implication or a bound.
- **Exact counting.** Corollary 2 gives the number of groups exactly.
- **What it does not give.** It says nothing about paths in the original
  link graph. That is why contradiction proofs needed a separate derivation
  (see "Proof-construction study").

The statement was also checked mechanically by a throwaway script (not in
the repository): all 1,004 smallest-shift reductions for n ≤ 9 over
alphabets of 2 and 3 letters, and the counting formula for every shift set
with n ≤ 9, with no failures.

## Example

n = 8, P = {3, 5}.

| Step | n | P | p | k | Links from (a) | Free |
|---|---|---|---|---|---|---|
| 1 | 8 | {3, 5} | 3 | 5 | x₅=x₂, x₆=x₃, x₇=x₄ | 0 |
| 2 | 5 | {2, 3} | 2 | 3 | x₃=x₁, x₄=x₂ | 0 |
| 3 | 3 | {1, 2} | 1 | 2 | x₂=x₁ | 0 |
| 4 | 2 | {1} | 1 | 1 | x₁=x₀ | 0 |
| end | 1 | ∅ | | | | 1 group |

All eight positions are equal, with seven links in total. This agrees with
Fine–Wilf (3 + 5 − 1 ≤ 8, gcd 1).

## Hypotheses that cannot be dropped

- **p must be the smallest shift.** Take n = 3, P = {1, 2}, and reduce with
  p = 2. Then k = 1, P′ is empty, and (a) only says x₂ = x₀. That allows
  |A|² assignments, but the true system forces all three positions equal.
  Shift 1 was lost because 1 − 2 is not a valid shift.
- **Each shift family must be complete.** Take n = 4 with only x₁ = x₃ and
  x₀ = x₃ (shift 2 missing its i = 0 instance). Treating this as P = {2, 3}
  yields one group; the true answer is two ({0, 1, 3} and {2}). The proof
  uses the shift-p constraint at arbitrary indices.
- **No wrap-around.** On a cycle, shift 2 on 5 positions gives one group; on
  the interval it gives two. The cyclic case is a different and simpler
  theorem (the number of groups is the gcd of n and the shifts).

## General statement versus stronger claims

**Most general valid statement (nearly trivial).** Let R be any set of
equalities on a finite set X, and Y a subset. Suppose every element outside
Y is either untouched by R or joined by R to an element of Y. Then the
groups of X correspond to the groups of Y under the equalities R induces on
Y, plus the untouched elements. This holds for any equality system and needs
no shifts.

**What the shift structure adds.** The theorem above is the claim that for
complete shift families on an interval, the induced system on
Y = {0, …, k−1} is again a complete shift family, with no more shifts than
before. That closure property is the real content, and it is what the three
hypotheses buy.

**Stronger claims and what they need:**

| Claim | Status |
|---|---|
| Iterating costs at most n link assignments | Follows from the theorem: the ranges in (a) are disjoint across steps. |
| Total time O((n + m) log(n + m)) | Follows, with a priority queue for the shifts (see "Proof-construction study", section 4). |
| Fine–Wilf as a corollary | Consistent with the theorem and with the tests; the induction has not been written out. |
| Short or fast contradiction proofs | Not implied. Needs the derivation; its cost bound is unproven. |
| Higher dimensions, non-translation maps, mixed partial families | Not covered. Each would need its own closure argument; none is given here. |

**Alternative formalization.** One can instead allow each constraint to be
"the first L positions repeat with shift q", with different L per
constraint. The reduction never needs this, because the complete-family form
is closed under it. The formalization above is the one that matches the
method as implemented.
