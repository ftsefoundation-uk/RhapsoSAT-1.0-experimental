# 03 — Multidimensional shift systems

**Scope.** Does the interval theorem generalise to grids, matrices and
higher-dimensional boxes? This note gives an exact coordinate criterion for
when a reduction step is covered by Theorem 3 of note 02, proves that the
one-dimensional guarantee ("a reducing shift always exists and the chain
always ends trivially") fails from dimension 2 on, proves one family where it
survives, and records data and a conjecture about the systems where it stops.
Status labels as in note 01; "Tested" refers to `theory_lab.rs`.

## 1. Setting

A **box system** is (D, P) with D = [0, n₁) × … × [0, n_d) ⊂ Zᵈ and P a
finite set of non-zero vectors, with constraints x_v = x_{v+q} whenever v and
v + q are both in D. For d = 2 this is a matrix whose entries are tied along
a few fixed displacement vectors. A shift q has an empty family exactly when
|qᵢ| ≥ nᵢ for some i.

## 2. The coordinate criterion

**Theorem 5 (Proved here, Tested).** Let q̃ have a non-empty family in the
box D. Then p splits q̃ on D (note 02) if and only if, for every coordinate
i, pᵢ lies between 0 and q̃ᵢ: 0 ≤ pᵢ ≤ q̃ᵢ or q̃ᵢ ≤ pᵢ ≤ 0.

So p is dominated in a box system iff every other shift with a non-empty
family has an orientation whose coordinates all "contain" those of p.

*Proof.* The set of v with v and v + q̃ in D is the box
Bᵢ = [max(0, −q̃ᵢ), nᵢ − max(0, q̃ᵢ)) in each coordinate, non-empty by
assumption. Splitting asks that vᵢ + pᵢ and vᵢ + q̃ᵢ − pᵢ lie in [0, nᵢ) for
all vᵢ ∈ Bᵢ, coordinate by coordinate.
If q̃ᵢ ≥ 0, vᵢ ranges over [0, nᵢ − q̃ᵢ). Taking vᵢ = 0 gives pᵢ ≥ 0 and
q̃ᵢ − pᵢ ≥ 0; conversely if 0 ≤ pᵢ ≤ q̃ᵢ then vᵢ + pᵢ ≤ vᵢ + q̃ᵢ < nᵢ and
0 ≤ vᵢ + q̃ᵢ − pᵢ < nᵢ.
If q̃ᵢ < 0, vᵢ ranges over [−q̃ᵢ, nᵢ). Taking vᵢ = nᵢ − 1 gives pᵢ ≤ 0 and
q̃ᵢ − pᵢ ≤ 0; the converse is symmetric. ∎

The test agrees with the definition on every reduction step of every tested
box system (15,067 steps in 2-D, 2,022 in 3-D).

**Corollary (intervals).** For d = 1 the criterion reads 0 < p ≤ q for all
other q: the smallest shift is always dominated. This is why the
one-dimensional reduction never gets stuck.

## 3. The obstruction in dimension ≥ 2 (Proved here, Tested)

**Proposition 7.** For every d ≥ 2 there are box systems with no dominated
shift, and reducing by a non-dominated shift can be inexact.

*Proof by example.* D = 2 × 2, P = {(1, 0), (0, 1)}. Neither shift lies
coordinatewise between 0 and ± the other, so by Theorem 5 neither is
dominated. All four points form one group. Reducing by p = (1, 0): D′ is the
column {(0, 0), (0, 1)}, P′ = {(1, 0), (−1, 1)} has empty families on D′, and
(a) links each point of the other column to its neighbour: two groups.
Property that fails: exactness (Theorem 3); the relaxation (Theorem 2) still
holds. Padding with further coordinates of size 1 gives every d ≥ 2. ∎

So the interval theorem does **not** generalise as "pick a shift and reduce".
What generalises is the conditional statement of Theorem 3.

How common is this? Among all systems with up to three shifts on boxes up to
4 × 4 and 5 × 3, 3,290 of 5,325 have no dominated shift; with up to two
shifts on 3-D boxes up to 3 × 3 × 2, 618 of 1,047 (Tested; exhaustive on
those ranges).

## 4. A family that does reduce completely (Proved here, Tested)

**Proposition 8 (collinear shifts).** Let u ∈ Zᵈ be non-zero and let every
shift of P be a positive integer multiple of u. Then the box system (D, P)
is totally reducible: at every stage the smallest multiple with a non-empty
family is dominated, P′ again consists of positive multiples of u, and the
chain ends when all families are empty. Hence
c(D, P) = (number of points of the final box) + Σ |F| over the steps, in
O(|P|² d Σ nᵢ) operations (Proposition 6).

*Proof.* Let p = t₀u with t₀ minimal among shifts with non-empty families,
and q = tu another such shift, t > t₀. In each coordinate pᵢ = t₀uᵢ lies
between 0 and tuᵢ, so p is dominated (Theorem 5), in the orientation q
itself. q − p = (t − t₀)u is again a positive multiple of u. Shifts with
empty families stay empty, because boxes only shrink. ∎

Tested on 2,907 random collinear systems in 2-D and 3-D boxes: all reach the
trivial system with the correct count. Along each line in direction u this
is the interval theorem; the point is that the box boundary, which cuts the
lines to different lengths, is handled by the same reduction.

## 5. Where the chain stops (Observed)

On 5,986 random 2-D systems (sides 3–12, one to four shifts with entries in
[−4, 4]):

- every chain of dominated reductions preserved the number of groups
  (Tested; this is Theorem 3 applied repeatedly);
- 2,440 chains reached the trivial system;
- the rest stopped at a system with no dominated shift, with 2, 3 or 4
  shifts left (1,439 / 1,305 / 802).

So in two dimensions the Euclid-style descent typically ends not at "no
shifts" but at a small **irreducible core**: a few shifts none of which lies
coordinatewise inside another. For two linearly independent shifts u, v the
number of groups of the core on an N × N grid was measured by brute force
(`sh theory_lab.rs table`):

| u, v | \|det(u, v)\| | groups for N = 6, 8, 12, 16, 24, 32, 48 |
|---|---:|---|
| (1,0), (0,1) | 1 | 1 at every N |
| (2,0), (0,3) | 6 | 6 at every N |
| (2,1), (1,2) | 3 | 5 at every N |
| (3,1), (1,2) | 5 | 7 at every N |
| (2,−1), (1,3) | 7 | 7 at every N |
| (3,0), (1,2) | 6 | 6 at every N |
| (4,1), (1,3) | 11 | 13 at every N |

**Conjecture 9 (as first stated; now settled — see note 05).** For two
linearly independent shifts u, v and all sufficiently large boxes,
c(D, {u, v}) = |det(u, v)| + κ(u, v), where κ ≥ 0 depends only on u and v.

**Update.** Note 05 proves an exact count for two independent shifts on
every box in every dimension (Theorem 10) and, from it, the conjecture in
two dimensions with κ = |u₁v₂| + |u₂v₁| − |det(u, v)| for boxes with
n₁ ≥ |u₁| + |v₁| and n₂ ≥ |u₂| + |v₂| (Corollary 11). Two corrections to the
statement above: it is false for thin boxes — the table measured squares
only — and it does not extend to three shifts in three dimensions, where the
count can grow with the box.

## 6. Relation to the literature

Multidimensional periodicity has been studied, with a different aim: when
do several period vectors on a region force a function to be constant or
lattice-periodic?

- Simpson and Tijdeman give multidimensional versions of the Fine–Wilf
  theorem (and of Sylvester's formula): for suitable period vectors
  generating Zᵏ, functions on a suitable body with those periods are
  constant.
- Mignosi, Restivo and Silva give Fine–Wilf-type theorems for
  two-dimensional words on convex domains, complete for parallelograms.
- Amir and Benson classify the periodicity of rectangular arrays into four
  types (non-periodic, lattice, line, radiant).

These are **related results, not the statements proved here**: they
characterise when the bulk structure is forced, while Theorem 5 and
Propositions 7–8 concern when one Euclid-style reduction step is exact and
whether it can be iterated. Only abstracts and citing descriptions of these
three papers were read for this note, so the comparison is at that level.
Whether Theorem 5 or Conjecture 9 follow from them was not determined.

## 7. Open questions and next steps

1. ~~Prove or refute Conjecture 9; find κ(u, v).~~ Done in note 05.
2. Characterise the box systems that are totally reducible. Proposition 8 is
   one family; the random data show many more.
3. Is there a different reduction operator (for example on parallelogram
   domains after a unimodular change of coordinates) under which the product
   system {(1,0), (0,1)} is reducible? The present operator cannot see
   product structure.
4. Proof-producing reduction in d ≥ 2: the unfolding of note 02, section 6,
   applies to every dominated step, but no implementation or measurement
   exists beyond intervals.
5. Non-box domains with a short description (L-shapes, triangles): Theorems
   2 and 3 apply, Theorem 5 does not.
