# 01 — The existing solver and its reduction theorem

**Scope.** What `prefsuf.rs` computes, the exact statement and proof of the
reduction theorem it rests on, where that theorem lives in the code, and what
evidence supports each claim. Nothing new is claimed in this note; it fixes
definitions for notes 02–04.

**Status labels used in all four notes.**

| Label | Meaning |
|---|---|
| **Proved here** | A complete argument is written in these notes. Not machine-checked by a proof assistant. |
| **Literature** | Stated in a published source that was located; see note 04 for how far each source was read. |
| **Tested** | Checked by a program in this repository, on the stated range only. |
| **Observed** | A measurement or pattern with no proof. |
| **Conjecture / open** | Not established. |

## 1. Definitions

- **n** — a non-negative integer, the length of the unknown text. Positions
  are 0, 1, …, n−1.
- **A** — the alphabet: any non-empty set. (The program restricts A to 1–94
  visible ASCII characters; the mathematics does not need that.)
- **Assignment** — x = (x₀, …, xₙ₋₁) with every xᵢ in A.
- **Shift** — an integer q with 0 < q < n, read as the partial map i ↦ i + q,
  defined only when i + q < n. There is no wrap-around.
- **P** — a finite set of shifts. The **constraints** E(n, P) are the
  equalities xᵢ = xᵢ₊q for every q in P and every i with 0 ≤ i < n − q. A
  shift always comes with *all* of these equalities (a *complete family*).
- **Sol(n, P)** — the set of assignments satisfying E(n, P).
- **Group** — a class of the smallest equivalence relation on positions
  containing every pair (i, i + q) of E(n, P). c(n, P) is the number of
  groups. Sol(n, P) is exactly the set of assignments constant on each
  group, so |Sol(n, P)| = |A|^c(n,P) when A is finite.
- **Dictionary to the program.** The rule `Bk` ("the first k characters equal
  the last k", a *border* of length k) is the complete family for the shift
  q = n − k (the text has *period* q). `B0` and `Bn` are empty families.
  Prefix and suffix rules fix letters and are handled after grouping; they
  are not part of the theorem.
- **Reduction.** Let P be non-empty, **p = min P** and **k = n − p**. Put
  **P′ = { q − p : q ∈ P, q > p } ∪ ({p} if p < k)**. The reduction replaces
  (n, P) by (k, P′). In border language: p belongs to the longest border k.

## 2. The theorem (Proved here; Literature)

**Theorem 1.** With the notation above, an assignment x is in Sol(n, P) if
and only if

- (a) xⱼ = xⱼ₋ₚ for every j with max(p, k) ≤ j < n, and
- (b) (x₀, …, xₖ₋₁) is in Sol(k, P′).

**Corollary 1 (bijection).** x ↦ ((x₀, …, xₖ₋₁), (xₖ, …, xₚ₋₁)) is a
bijection

> Sol(n, P) ≅ Sol(k, P′) × A^max(0, 2p−n).

The second factor is present only when p > k; its positions k, …, p−1 occur
in no constraint.

**Corollary 2 (counting).** c(n, P) = c(k, P′) + max(0, 2p − n), and
c(n, ∅) = n.

**Corollary 3 (closure and termination).** Every element of P′ is a shift
for length k (positive and less than k), |P′| ≤ |P|, and k < n. Repeating
the reduction therefore ends, after at most n steps, at a system with no
shifts, whose solution set is A^(its length).

**Proof of Theorem 1.**

*Forward.* Let x ∈ Sol(n, P). (a) consists of shift-p constraints (j − p ≥ 0,
j < n). For (b): if q > p and i + (q − p) < k, then i + q < n, so xᵢ = xᵢ₊q
(shift q), and xᵢ₊q₋ₚ = xᵢ₊q (shift p at index i + q − p ≥ 0). Hence
xᵢ = xᵢ₊q₋ₚ. If p < k, the shift-p constraints with i + p < k are among the
original ones.

*Backward.* Assume (a) and (b).
Shift p: take i with i + p < n, j = i + p. If j ≥ max(p, k), this is (a).
Otherwise j < k, hence p < k, so p ∈ P′ and (b) gives xᵢ = xⱼ.
Shift q > p: take i with i + q < n. Then i + (q − p) < k, so (b) gives
xᵢ = xᵢ₊q₋ₚ, and the shift-p constraint just proved gives xᵢ₊q₋ₚ = xᵢ₊q. ∎

*Corollary 1.* By (a) each position j ≥ max(p, k) is determined by position
j − p < k. If p > k, a position j with k ≤ j < p is in no constraint: it
cannot be the upper end of a link (every shift is ≥ p > j) nor the lower end
(j + q ≥ k + p = n). Corollary 2 is Corollary 1 counted. ∎

**The three hypotheses are needed.** The first counterexample is checked by
`theory_lab.rs` (experiment E1); the other two are small enough to check by
hand:

| Hypothesis | Counterexample |
|---|---|
| p is the *smallest* shift | n = 3, P = {1, 2}, reduce by p = 2: the reduced description has 2 groups, the truth is 1. |
| every shift family is *complete* | n = 4 with only x₁ = x₃ and x₀ = x₃: treating it as P = {2, 3} gives 1 group, the truth is 2. |
| *no wrap-around* | On a cycle of 5 positions shift 2 gives 1 group; on the interval it gives 2. |

## 3. Relation to the literature (important)

Theorem 1 is **not new**. It is the standard Euclid-style reduction for
multiperiodic words:

- Š. Holub, *On an algorithm for multiperiodic words* (2013), read in full
  for this note. With m = min P and Q = {p − m : p ∈ P, p ≠ m} ∪ {m}, his
  **Lemma 1** says two positions below k are equivalent under (Q, k) iff
  they are equivalent under (P, k + m); **Corollary 1** says the canonical
  word for (Q, k) is a prefix of the one for (P, k + m); **Lemma 2** says
  positions i with n − m ≤ i ≤ m − 1 are singleton classes. Together these
  are Theorem 1 and its corollaries (his Q keeps m even when m ≥ k, where it
  is vacuous). He writes that the reduction "is, in fact, one step in the
  Euclidean algorithm, and is well known in the literature on multiperiodic
  words", and that the resulting procedure is identical to **Algorithm B of
  Tijdeman and Zamboni**.
- The same reduction underlies the bounds of Constantinescu–Ilie and
  Tijdeman–Zamboni for an arbitrary number of periods (abstracts and
  citing papers only were checked here).

What the repository adds to that is engineering and checking, not the
theorem: letter constraints and contradiction certificates on top of the
grouping, a priority-queue implementation with a stated bound, independent
test oracles, and (notes 02–03) a test of how far the reduction extends.

Full references are in note 04.

## 4. Where the theorem is in the code

`equal_positions` in `prefsuf.rs` is Theorem 1 iterated, in border language:

| Theorem | Code |
|---|---|
| current length n | `len` |
| longest border k, p = n − k | `k` (largest of `given[next]` and `derived.peek()`), `p = len - k` |
| (a): link j to j − p for max(p, k) ≤ j < n | `for j in p.max(k)..len { root[j] = j - p; }` |
| P′ ∋ q − p for the other shifts | nothing to do: a border k′ < k of the text is the border k′ of its first k characters, and its shift there is (n − k′) − p |
| P′ ∋ p when p < k | `if k > p` … the border `c = k - p` is pushed to `derived` (or handled at once if it is the longest waiting) |
| continue with (k, P′) | `len = k` |
| final labels | one left-to-right pass `root[i] = root[root[i]]` |

Prefix and suffix letters are then attached to groups (`solve_with`, steps
2–4), which is ordinary and not part of the theorem.

## 5. Termination and complexity of the grouping (Proved here)

Let m be the number of border rules.

- **Measure.** `len` strictly decreases in every round (k < len).
- **Each position is written at most once in the rounds.** A round writes
  the positions in [max(p, k), len) ⊆ [k, len) and then sets len = k, so the
  ranges of different rounds are disjoint. The final pass writes each
  position once more.
- **Every round writes at least one position** (len − p = k ≥ 1 and
  len − k = p ≥ 1), so there are at most n rounds.
- **Queue work.** Sorting the given borders costs O(m log m). At most one
  border is pushed per run of rounds, so at most n pushes; every element is
  removed at most once. Each queue operation costs O(log(n + m)).

**Total: O((n + m) log(n + m))** time and O(n + m) space, in the unit-cost
model for index arithmetic. This is an upper bound; whether the n log n term
can actually occur was not investigated.

This bound is for the grouping only. Building contradiction certificates is
separate: the shortest-proof search is capped by a fixed budget and the
derivation that replaces it beyond the budget has **no proved bound better
than polynomial** (note 04).

## 6. Evidence in the repository

| Claim | Status | Where |
|---|---|---|
| Theorem 1 and corollaries | Proved here; Literature | this note; `resultssummary.md`, part "Border reduction as a theorem"; Holub 2013 |
| Theorem 1 on all shift sets, n ≤ 9 | Tested | `theory_lab.rs` E1 (502 systems) |
| Counting corollary vs the solver's grouping | Tested | `prefsuf.rs` self-test "number of groups equals the reduction theorem's count" (3,000 random border sets, n ≤ 300) |
| Grouping agrees with brute force / independent search | Tested | `prefsuf.rs` self-tests, `hard.rs`, `adversarial.rs` |
| Grouping is O((n + m) log(n + m)) | Proved here | section 5 |
| Groups of a given puzzle are exactly the reported ones | Tested per instance by an independent checker | `theory_lab.rs` E5 (partition certificates; note 04) |
| Any link to NP-complete problems | **None established** | — |

A premise check: the project brief says the equivalence "is mathematically
proven". In the repository that means a written proof (here and in
`resultssummary.md`) plus exhaustive small-case tests. There is no
machine-checked (proof-assistant) proof.

## 7. Unresolved, and next

- Is the n log n term in section 5 attained, or is the true bound
  O(n + m log m)?
- A machine-checked proof of Theorem 1 would remove the remaining trust in
  the prose.
- Notes 02 and 03 ask how far Theorem 1 extends.
