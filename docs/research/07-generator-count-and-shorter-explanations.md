# 07 — How many generators, and shorter explanations

**Scope.** Two follow-ups to note 06: the number of generator borders, and
the length of contradiction explanations. Also a ranked list of the open
research questions and an honest record of what this round did *not* reach
(multidimensional theory). Status labels as in note 01, plus
**Established literature**. "Tested" refers to `reduce_lab.rs` (R3, R4) and
the self-tests. Nothing is machine-checked.

Definitions as in note 06: text length n ≥ 2, Border rules with borders
0 < b < n, phases of the reduction, *generators* = the Border rules that
start a phase. G(n) denotes the largest number of generators over all sets
of borders for a text of length n. Logarithms are base 2; "Θ(log n)" means
bounded above and below by constant multiples of log n for all n ≥ 2.

## 1. The number of generators is Θ(log n)

**Theorem 19 (Proved here, Tested).** For every n ≥ 2,

> ⌊log₂ n⌋ ≤ G(n) ≤ 2 log₂ n + 2.

So no constant bounds the number of generators, and the logarithmic bound of
note 06 is tight up to a factor of about 2.

*Proof.* The upper bound is Theorem 16 (every generator starts a phase).
Lower bound: let j = ⌊log₂ n⌋ − 1, so 2^(j+1) ≤ n, and take the borders
1, 2, 4, …, 2^j. The longest, 2^j, is used first, at length n, with shift
p = n − 2^j ≥ 2^j. After one round the text looked at has length 2^j ≤ p, so
the phase ends and leaves no derived border behind. The same now repeats at
length 2^i with border 2^(i−1) and shift 2^(i−1), for i = j, j−1, …, 1:
each phase has one round, ends with length equal to its shift, and leaves
nothing behind. So every phase is started by a rule: j + 1 = ⌊log₂ n⌋
generators. ∎

**The generators of that family are all necessary (Tested for k ≤ 40).** For
n = 2^k with borders n/2, n/4, …, 1 there is one group, and leaving out any
single border leaves more than one group. This was computed exactly for
k = 1 … 40; a proof for all k was not written. Where it holds, *no* method
can certify the grouping with fewer than log₂ n of the given borders, so the
logarithmic number is a property of the problem, not of this algorithm.

**Generators are not always necessary (Counterexample).** n = 11, borders
3, 4, 6: three generators, but the third starts a phase when the remaining
text already forms a single group. So "generator" is sufficient for
forcing all equalities (Theorem 17), not a minimal set.

**Conjecture 20.** G(n) = ⌊log₂ n⌋ for every n ≥ 2.

*Evidence (Experimental observation).* Exhaustive over every set of borders
for n ≤ 25 (33.5 million sets): the maximum is exactly ⌊log₂ n⌋ for each n.
No larger count in 3 million random sets up to n = 10⁶ and 400,000 up to
n = 2⁴⁰.

*What was tried and failed.* Three candidate invariants would each prove the
conjecture if they halved at every generator: the shift, min(shift, border),
and the number of groups of the remaining text. All three are false
(hundreds of thousands of counterexamples for n ≤ 22; the groups invariant
fails on n = 11, borders 3, 4, 6). A proof needs a different potential.
The gap between ⌊log₂ n⌋ and 2 log₂ n + 2 is therefore **open**.

For the number of *phases* the picture differs: up to 5 phases occur at
n = 20 (borders with periods 13 and 8, Fibonacci numbers), above
⌊log₂ 20⌋ = 4. Phases can exceed log₂ n; generators, as far as observed,
cannot.

## 2. Shorter explanations

### 2.1 What is and is not known

| Statement | Status |
|---|---|
| A shortest chain can be found by breadth-first search in O(n·m) link tests | Established (textbook) |
| A shortest chain can be found in time o(n·m) in the worst case | Open |
| A valid chain can be found with O(n log n) link tests | Proved (Theorem 18, note 06) |
| The chain produced after shortening is near-shortest | Not claimed; measured below |

The solver's explanation pipeline, with proved worst-case look-up counts
(g ≤ 2 log₂ n + 2 generators, L = length of the generator chain ≤ n):

| Stage | Look-ups (worst case, proved) | Guarantee |
|---|---|---|
| 1. Breadth-first search, all borders | ≤ 2·10⁸ + 32n, then abandoned | shortest chain if it finishes |
| 2. Breadth-first search, generators only | ≤ 2gn | shortest chain through generator links |
| 3. Farthest-reach shortening (new) | ≤ 64n + 10⁷ + 2m | valid, never longer |
| 4. Cheap shortening (as before) | ≤ 8L + 64 | valid, never longer |

Memory: stages 1–2 use three arrays of n entries; stage 3 one array of n
32-bit entries; stage 4 one array of n entries.

### 2.2 Farthest-reach shortening (new; Proved valid, not optimal)

`reach_shorten` in `prefsuf.rs`. Given a chain, record the index of every
position on it. From the current position try every border period in both
directions and jump to the reachable position that lies farthest along the
chain; repeat from there. Only positions landed on are examined, so the
cost is 2m look-ups per position of the *result*; once 64n + 10⁷ look-ups
are spent the rest of the chain is kept unchanged.

- **Proved:** every step of the result is a link of a Border rule (the jump
  distance is that rule's period), the result starts and ends where the
  input does, and it is never longer than the input (each jump moves at
  least one index forward).
- **Not proved, and false in general:** that the result is a shortest chain.
  Greedy farthest-jump is optimal only when reachability is monotone along
  the chain.

### 2.3 Measurements

`sh reduce_lab.rs proofs 1000000` (machine as in note 04; single runs).
Chain between position 0 and the last position of its group; lengths in
positions. "Before" is note 06 (cheap shortening only).

| Shape (n = 10⁶) | Borders | Shortest (full search) | Generator chain | Shortened before | Shortened now |
|---|---:|---:|---:|---:|---:|
| periods 2 and 3 | 2 | 333,334 | 333,334 | 333,334 | 333,334 |
| coprime at the bound | 2 | 999,998 | 999,998 | 999,998 | 999,998 |
| 16 random | 16 | 8 | 431 | 431 | 431 |
| 1,000 random | 1,000 | 3 | 1,510 | 43 | 43 |
| every border | 999,999 | 2 | 1,000,000 | 2 | 2 |
| block twice, block period 3 | 166,666 | unknown (search gave up) | 333,332 | 4 | 4 |
| periods 4, 8, … and 2 | 250,000 | unknown | 500,000 | 3 | 3 |
| Fibonacci periods | 28 | 8 | 333,334 | 12 | 12 |
| 1,670 small periods | 1,670 | unknown | 1,000,000 | 960,016 | **202** |

For the last shape every period is below 5,000, so no chain between
positions 0 and 999,999 has fewer than 201 positions: 202 is within one of
optimal there. On 4,375 random pairs (`reduce_lab.rs check 1`, R3) the
shortened chain equals the shortest for 2,811 pairs; in total 50,151
positions against 29,503 for shortest chains. The "16 random" and Fibonacci
rows show the remaining gap: with few borders the shortest chain leaves the
generator chain entirely, and no shortening of that chain finds it.

Runtime of stage 2 at n = 10⁶ is 0.001–0.13 s (table in note 06); stage 3
added no measurable time on these shapes (it examines only the positions it
lands on). Operations were not instrumented separately from time in this
round; the look-up bounds above are the proved figures.

## 3. Ranked research questions

| Rank | Question | Importance | Feasibility | Why this rank |
|---:|---|---|---|---|
| 1 | Conjecture 20: G(n) = ⌊log₂ n⌋ | Closes the generator theory | Medium: exhaustive data is clean, three natural invariants already refuted | Small, sharp, decidable by the right potential function |
| 2 | Near-shortest chains with a proved ratio in near-linear time | Directly improves what users read | Medium | The "16 random" case shows the current method can be 50× too long |
| 3 | Three or more shifts in 2-D: closed corner constant and threshold | Completes the 2-D picture | Medium: proof sketch exists (note 05) | Next natural step after Theorem 10 |
| 4 | Three independent shifts in 3-D (Conjecture 14) | Decides whether any determinant-style count survives | Low to medium | Counterexamples exist; no structure theorem yet |
| 5 | Shortest chains in o(n·m) | Theoretical | Low | No approach known here |
| 6 | Whether Theorems 16–17 and 19 are in the literature | Needed before any novelty claim | High with library access | See section 5 |
| 7 | Machine-checked proofs of Theorems 1, 10, 16, 17 | Removes trust in prose | Low here (no proof assistant installed) | Valuable, but tooling is missing |

## 4. Not done in this round

- **Multidimensional theory (questions 3 and 4).** No new result. The state
  is that of note 05: Theorem 10 and Corollary 11 proved; three or more
  shifts in 2-D a proof sketch with tests; 3-D counterexamples and
  Conjecture 14.
- **End-to-end performance review.** One redundancy was noted and left:
  `certificate` solves the puzzle and then groups the positions a second
  time. No change was made to the grouping or counting code, and no
  benchmark suggests one is needed (note 06, section 4).
- **Operation counters** separate from timings for the explanation stages.

## 5. Literature

- **E. Rivals, S. Rahmann, *Combinatorics of periods in strings*,
  J. Combin. Theory Ser. A 104 (2003) 95–113** (Established literature;
  located, **not readable here**: the PDF's text could not be extracted and
  no PDF tool is installed). Its abstract and section headings confirm that
  it introduces the *irreducible period set*, a smaller set of periods that
  determines the whole period set of a word, with conversions between the
  two. That is the closest published relative of the generators of note 06.
  Whether it bounds the size of an irreducible period set, and how that
  relates to Theorem 19, was **not verified**. From memory (unverified), a
  logarithmic size is discussed there; if so, Theorem 19's upper bound may
  be a restatement in a different setting. The settings differ: an
  irreducible period set describes the periods a word *has*; generators
  describe which given border *constraints* are used by the reduction.
- E. Rivals, M. Sweering, P. Wang, ICALP 2023 (bound on the number of
  period sets, using irreducible period sets): found by search, title and
  venue not checked further.
- No source was found for Theorem 17 or for the lower-bound family of
  Theorem 19; the search was two web queries. **No novelty is claimed.**

## 6. Status table

| Claim | Status |
|---|---|
| ⌊log₂ n⌋ ≤ G(n) ≤ 2 log₂ n + 2 | Independently proved |
| The family's generators are all necessary | Tested for k ≤ 40; no general proof written |
| G(n) = ⌊log₂ n⌋ | Conjecture (exhaustive for n ≤ 25) |
| Farthest-reach shortening is valid and never lengthens | Independently proved; Tested |
| Shortened chains are near-shortest | Experimental observation only; false in general |
| Shortest chains in o(n·m) | Open question |
| Any connection to NP-complete problems | None established |
