# 02 — A general framework for form-preserving reduction

**Scope.** Lift Theorem 1 of note 01 from intervals to arbitrary finite
point sets of Zᵈ, isolate exactly which hypothesis makes the reduction
exact, and test three extensions: a mirror (palindrome) constraint, weighted
shifts, and costs. Status labels are those of note 01. Every "Tested" claim
refers to `theory_lab.rs` (run `sh theory_lab.rs`).

The central question is: *can efficiently reducible equality-constraint
systems be characterised by families of transformations that preserve their
own constraint structure?* Section 7 answers as far as the results allow.

## 1. The family of systems

- **Domain.** D, a finite subset of Zᵈ (d ≥ 1).
- **Shifts.** P, a finite set of non-zero vectors of Zᵈ. The vectors q and
  −q describe the same constraints; an *orientation* of q is a choice of one
  of them.
- **Constraints.** E(D, P): x_v = x_{v+q} for every q ∈ P and every v with
  v ∈ D and v + q ∈ D (complete families, no wrap-around).
- **Sol(D, P)**, **groups**, **c(D, P)** as in note 01, with A any non-empty
  set.
- **𝔉** — the family of all such pairs (D, P). Sub-families: *intervals*
  (d = 1, D = {0, …, n−1}; this is note 01) and *boxes*
  (D = [0, n₁) × … × [0, n_d)). Constraints are translation-invariant, so a
  translated box is the same system.

## 2. The reduction operator

Fix p ∈ P and an orientation q̃ of every other q ∈ P.

- D′ = { v ∈ D : v + p ∈ D }.
- T⁺ = { t ∈ D ∖ D′ : t − p ∈ D } and F = { t ∈ D ∖ D′ : t − p ∉ D }.
- P′ = { q̃ − p : q ∈ P, q ≠ ±p } ∪ { p }, with zero vectors dropped.
- **Reduced description** Red(D, P; p): the assignments x on D with
  (a) x_t = x_{t−p} for every t ∈ T⁺, and
  (b) the restriction of x to D′ lies in Sol(D′, P′).

R_p : (D, P) ↦ (D′, P′) is the **reduction operator**. For an interval with
p = min P and all shifts oriented positively this is exactly note 01.

## 3. Results

**Theorem 2 (relaxation; Proved here, Tested).** For every (D, P), every
p ∈ P and every orientation, Sol(D, P) ⊆ Red(D, P; p).

*Proof.* (a) consists of shift-p constraints. For (b): shift-p constraints
inside D′ are original constraints. For q̃ − p: if v and w = v + q̃ − p are in
D′, then w + p = v + q̃ ∈ D by definition of D′, so x_v = x_{v+q̃} is an
original shift-q constraint and x_w = x_{w+p} is an original shift-p
constraint; hence x_v = x_w. ∎

So the reduced description never forces an equality that the original
system does not force. It may force fewer.

**Definition (splitting).** p *splits* q̃ on D if for every v:
v ∈ D and v + q̃ ∈ D imply v + p ∈ D and v + q̃ − p ∈ D. The shift p is
**dominated** in (D, P) if every other shift has an orientation that p
splits. (If the family of q is empty, both orientations are split.)

**Theorem 3 (exactness; Proved here, Tested).** If p is dominated and every
other shift is taken in an orientation that p splits, then
Sol(D, P) = Red(D, P; p). Consequently

> Sol(D, P) ≅ Sol(D′, P′) × A^F and c(D, P) = c(D′, P′) + |F|,

the bijection being restriction to D′ ∪ F.

*Proof.* By Theorem 2 only Red ⊆ Sol is needed. Let x ∈ Red.
Shift p: a link (v, v + p) has v ∈ D′. If v + p ∈ D′, (b) gives it. Otherwise
v + p ∈ T⁺ (because (v + p) − p = v ∈ D), and (a) gives it.
Shift q ≠ ±p: every link of the family can be written (v, v + q̃). By
splitting, v + p ∈ D, so v ∈ D′; and w = v + q̃ − p ∈ D with w + p = v + q̃ ∈ D,
so w ∈ D′. Then (b) gives x_v = x_w and the shift-p link (w, w + p), just
proved, gives x_w = x_{v+q̃}.
Bijection: by (a) the values on T⁺ are determined by values on D′ (note
t − p ∈ D′ for t ∈ T⁺); the points of F occur in no clause of (a) or (b), so
they are free. ∎

**The orientation matters.** A first draft of Theorem 3 took q − p for an
arbitrary orientation of q. The machine check refuted it (307 failures among
2,035 dominated reductions on small 2-D boxes); with the split orientation
all 2,035 are exact. In one dimension all shifts are positive and the issue
does not arise.

**Proposition 4 (closure; Proved here).** R_p maps 𝔉 into 𝔉, intervals to
intervals, and boxes to boxes (D′ is the box with sides nᵢ − |pᵢ|, up to
translation). |P′| ≤ |P|.

**Proposition 5 (termination; Proved here).** If D is non-empty then
D′ ⊊ D: a point of D maximising the inner product with p is not in D′. So
|D| is a strictly decreasing measure and any chain of reductions is finite.

**Proposition 6 (complexity for boxes; Proved here).** Call a box system
*totally reducible* if repeated reduction by dominated shifts reaches a
system all of whose families are empty. For such a system the number of
groups can be computed with at most n₁ + … + n_d reduction steps, each
costing O(|P|² d) integer operations (Theorem 5 of note 03 makes "dominated"
a coordinate test), that is **O(|P|² · d · Σ nᵢ)** in total — independent
of the number of points Π nᵢ and of the number of constraints. Explicit
group labels cost an additional O(|D|).

*Proof.* Each step lowers some side by at least 1 and |P′| ≤ |P|; the free
count |F| = |D| − 2|D′| + |D″|, with D″ the box of sides nᵢ − 2|pᵢ| (empty if
any is ≤ 0), is a product formula. ∎ (No attempt was made to optimise this;
for intervals the queue-based bound of note 01 is better.)

**Splitting is sufficient, not necessary (Tested).** Interval n = 8,
P = {2, 4}, reduce by p = 4: 4 is not dominated (it does not split 2), yet
the reduced description is exact. The exact criterion is the tautology
"every link of every other family has its endpoints joined in the reduced
description"; splitting is the special case where that join is always the
two-link path through v + q̃ − p. Small-case counts of exact-but-not-dominated
reductions are in note 04.

**What is and is not general.**

| Statement | Needs |
|---|---|
| Reduced description is a relaxation (Thm 2) | nothing |
| Reduced description is exact (Thm 3) | p dominated, split orientations |
| A dominated shift exists | intervals: always (the smallest shift). Boxes with d ≥ 2: not always (note 03) |
| The chain ends at a trivial system | intervals: always. Boxes: collinear shift sets always; in general no (note 03) |

## 4. Extension 1: a mirror constraint (Proved here, Tested)

Add to an interval system the constraint **mirror**: xᵢ = x_{n−1−i} for all
i (the text is a palindrome). Write Sol(n, P, mirror).

**Theorem M.** Let P be non-empty, p = min P, k = n − p, and P′ as in note
01. Then x ∈ Sol(n, P, mirror) if and only if

- (a) xⱼ = xⱼ₋ₚ for max(p, k) ≤ j < n,
- (b) (x₀, …, xₖ₋₁) ∈ Sol(k, P′, mirror) — the mirror of the *shorter* text,
- (c) xⱼ = x_{n−1−j} for k ≤ j < p (the free strip is itself a palindrome).

Hence Sol(n, P, mirror) ≅ Sol(k, P′, mirror) × A^⌈max(0, 2p−n)/2⌉, and the
chain ends at Sol(n, ∅, mirror) ≅ A^⌈n/2⌉.

*Proof.* Forward: for v < k, x_v = x_{n−1−v} (mirror) and
n − 1 − v = (k − 1 − v) + p with k − 1 − v ≥ 0, so x_{k−1−v} = x_{n−1−v} by
shift p; hence x_v = x_{k−1−v}, the mirror of length k. (c) is part of the
mirror; the shift clauses are Theorem 1.
Backward: shifts as in Theorem 1. For a mirror pair {v, n−1−v}: if v < k,
(b) gives x_v = x_{k−1−v} and shift p gives x_{k−1−v} = x_{n−1−v}. If
n − 1 − v < k, the same with the roles exchanged. Otherwise both are ≥ k,
so k ≤ v ≤ p − 1 and (c) applies.
Counting: j ↦ n − 1 − j maps the strip [k, p) onto itself and strip
positions are in no shift constraint. ∎

The reduced generator is "shift by −p, then mirror". The generators here do
not commute (they generate a dihedral group), so closure under reduction is
not limited to commuting shifts. Tested on all 4,095 systems with n ≤ 12 and
3,000 random systems up to n = 3,000. Whether Theorem M appears in the
literature was not determined; the palindromic structure of extremal
Fine–Wilf words is known (Holub 2013 mentions it; Tijdeman–Zamboni).

*Not covered:* mirrors of a prefix only, or several mirrors. Two different
mirrors compose to a shift whose family need not be complete, which breaks
the hypothesis of every theorem here.

## 5. Extension 2: weights

"Weighted shift" can mean two different things.

**(i) Weights with equality semantics (Proved here, Tested).** Let A be an
abelian group and replace each constraint by x_{i+q} = xᵢ + w for a weight
w ∈ A attached to the shift; P is now a finite multiset of pairs (q, w).
Choose (p, w_p) with p minimal and put

P′ = { (q − p, w − w_p) : (q, w) ∈ P ∖ {(p, w_p)} } ∪ ({(p, w_p)} if p < k),

where a pair (0, c) is read as "xᵢ = xᵢ + c for all i < k" (a contradiction
when c ≠ 0 and k > 0). Then x ∈ Sol(n, P) iff (a′) xⱼ = xⱼ₋ₚ + w_p for
max(p, k) ≤ j < n and (b) (x₀, …, xₖ₋₁) ∈ Sol(k, P′); the bijection of
Corollary 1 holds verbatim, with both sides empty when the system is
inconsistent. The proof is that of Theorem 1 with the offsets carried along:
x_{i+q} = xᵢ + w and x_{i+q} = x_{i+q−p} + w_p give x_{i+q−p} = xᵢ + w − w_p.
Tested exhaustively for A = Z₃, n ≤ 6, up to three weighted shifts (1,275
systems, 640 of them inconsistent). The same proof works when each shift
carries an arbitrary bijection of A; that version was not tested.

**(ii) Weights as inequalities (counterexample; Tested).** With
x_{i+q} ≤ xᵢ + w the forward step fails, because an inequality cannot be
inverted. n = 3, constraints x₁ ≤ x₀ + 5, x₂ ≤ x₁ + 5, x₂ ≤ x₀: the recipe
of (i) yields on {0, 1} the constraint x₁ ≤ x₀ − 5, which rejects the
solution (0, 0, 0). The property that fails is Theorem 2 (the "reduced"
system is no longer a relaxation).

**(iii) Weights as costs (counterexample; Tested).** If each link has a cost
and one asks for cheapest chains (for example shortest contradiction
proofs), reduction preserves the *groups* but not *distances*. n = 5,
P = {1, 4}: positions 0 and 4 are one link apart, but the chain of
reduction links between them has four links. This is the small-scale version
of the gap between shortest and derived proofs in `resultssummary.md`.

## 6. Proof-producing reduction

A reduction step justifies each derived link by two links one level up
(Theorem 3's proof), so every derived equality unfolds into original links.
`prefsuf.rs` does this for intervals (`Derivation`), memoising with a
union-find so that only a spanning forest of original links is kept. Two
certificate types result; both are checked without the solver:

| Claim certified | Certificate | What the checker verifies | Checker cost |
|---|---|---|---|
| two positions are forced equal | a chain of original links | each link is a link of the cited rule | linear in the chain |
| the groups are *exactly* these | labels + spanning forest of original links | (1) each forest link is original; (2) the forest joins exactly the label classes; (3) the label word has every required border, by the classical border table | O((n + m) α(n)) |

(3) is the completeness half: the labels, read as a text, are themselves a
solution in which different classes hold different letters, so no two
classes are forced equal. Details and measurements: note 04.

## 7. Assessment of the central question

- **Closure alone characterises nothing.** The family of *all* shift systems
  on finite point sets is closed under every R_p (Proposition 4). Closure
  becomes meaningful only together with exactness.
- **A sufficient framework exists and is proved:** family 𝔉 (or a sub-family
  with a constant-size description of D), operator R_p, closure
  (Prop. 4), exact bijection under domination (Thm 3), decreasing measure
  |D| (Prop. 5), and a complexity bound independent of the number of
  constraints when the chain reaches a trivial system (Prop. 6).
- **It is not a characterisation of efficient solvability.** The grid with
  the two unit shifts (1, 0) and (0, 1) has no dominated shift, yet it is a
  product of two interval systems and trivially easy. So "admits a
  form-preserving exact reduction of this kind" is sufficient for an
  algorithm sublinear in the number of constraints, not necessary.
- **Nor is domination necessary for exactness** (section 3).
- **What the reduction buys.** Any finite equality system can be closed in
  time near-linear in the number of *constraints* by union-find. The
  reduction's gain is to be near-linear in the number of *variables* (or,
  for counting, polynomial in the description size) when the constraints are
  generated by few shifts. That is a statement about succinctly described
  instances of a polynomial-time problem. No connection to NP-complete
  problems has been established.

## 8. Open

- Necessary and sufficient conditions for *existence* of an exact reducing
  shift in a box system, and for total reducibility (note 03 has partial
  results and data).
- A reduction for domains that are not boxes but have a short description.
- Whether Theorem M and Theorem 3 are in the literature under other names.
