# 05 — Counting groups in box systems: two shifts, classification, limits

**Scope.** Results obtained after notes 01–04 on box systems in Zᵈ: an exact
count for two independent shifts in every dimension and every box size,
which settles Conjecture 9 of note 03; uniqueness of the dominated shift;
which systems reduce completely; and where the picture stops (three shifts
in three dimensions). Definitions are those of notes 02 and 03 and are not
changed. All "Tested" claims refer to `grid_lab.rs` (`sh grid_lab.rs`),
which shares no code with the solver.

**Status labels.** As in note 01, with one addition: **Proof sketch** means
the argument is outlined and believed correct but not written in full.
Nothing here is machine-checked by a proof assistant.

**Terms not defined by the project.** Notes 01–04 define *borders* only for
intervals (border k is the shift n − k). There is no multidimensional border
in the documents or the code, so this note speaks of shifts only.

## 1. Two independent shifts: an exact count

Let D = [0, n₁) × … × [0, n_d) and let u, v ∈ Zᵈ be linearly independent.
For a list of shifts T write

> N(T) = ∏ᵢ max(0, nᵢ − Σ_{q∈T} |qᵢ|)

(read as 0 if any factor is 0). N(∅) = |D| is the number of points, N({q})
the number of links of the family of q, and N({u, v}) the number of
*squares*: points x with x, x + u, x + v, x + u + v all in D. (The four
points span |uᵢ| + |vᵢ| in coordinate i whatever the signs.)

**Theorem 10 (Proved here, Tested).** For every d ≥ 2, every box D and
every pair of linearly independent shifts u, v,

> c(D, {u, v}) = N(∅) − N({u}) − N({v}) + N({u, v}).

*Proof.* Let L = Zu + Zv. Two points joined by a link differ by ±u or ±v, so
every group lies in one coset of L. Fix a coset x₀ + L. Since u, v are
independent, φ(i, j) = x₀ + iu + jv is a bijection from Z² onto the coset.
Let K ⊂ R² be the set of real (s, t) with x₀ + su + tv in the real box
[0, n₁−1] × … × [0, n_d−1]. K is convex and closed (the preimage of a convex
closed set under an affine map), and φ maps K ∩ Z² onto D ∩ (x₀ + L).

Under φ, the links inside the coset are exactly the unit segments of the
square grid with both ends in K, and the squares are exactly the unit grid
squares with all four corners in K. So the links of the coset form a finite
plane graph G drawn on the grid. For a plane graph with C components,
V − E + (number of bounded faces) = C (Euler's formula). It remains to show
that the bounded faces of G are exactly the squares.

A square has its four sides in G and no vertex or edge of G in its interior
(grid edges do not cross a unit square), so it is a bounded face. Conversely
let f be a bounded face. It is enclosed by a cycle Z of G. Z consists of grid
edges, so the region it encloses is a union of unit grid squares; f contains
a point of the open interior of one of them, Q. The corners of Q lie in the
closed region enclosed by Z, hence in the convex hull of the vertices of Z,
hence in K, because the vertices of Z are in K and K is convex. So Q is a
square of the system, its sides are edges of G, and f = Q.

Summing V − E + S over the cosets gives the formula, with V, E, S the
numbers of points, links and squares of D; these are the products N. ∎

The same proof gives c = V − E + S for *any* domain of the form
(convex set) ∩ Zᵈ, with V, E, S counted in that domain. Boxes are the case
where the three numbers are products.

**Corollary 11 (two dimensions, Proved here, Tested).** Let d = 2 and
n₁ ≥ |u₁| + |v₁|, n₂ ≥ |u₂| + |v₂|. Then

> c(D, {u, v}) = |u₁v₂| + |u₂v₁| = |det(u, v)| + κ(u, v),

where det(u, v) = u₁v₂ − u₂v₁ and

> κ(u, v) = 2·min(|u₁v₂|, |u₂v₁|) if u₁v₂ and u₂v₁ have the same sign and
> are non-zero, and κ(u, v) = 0 otherwise.

*Proof.* Expand n₁n₂ − (n₁−|u₁|)(n₂−|u₂|) − (n₁−|v₁|)(n₂−|v₂|)
+ (n₁−|u₁|−|v₁|)(n₂−|u₂|−|v₂|); all terms containing n₁ or n₂ cancel and
|u₁||v₂| + |u₂||v₁| remains. The second form is |a| + |b| − |a − b| with
a = u₁v₂, b = u₂v₁. ∎

**This settles Conjecture 9 of note 03**, with corrections:

- It is true for d = 2 with the explicit constant κ above (always even),
  and the threshold "sufficiently large" is n₁ ≥ |u₁| + |v₁| and
  n₂ ≥ |u₂| + |v₂|.
- The threshold is sufficient, not necessary: the closed form also happens
  to hold on some smaller boxes.
- It is **false for thin boxes**: with u = (2, 1), v = (1, 2) a 2 × N box
  has N + 2 groups (Theorem 10 gives 2N − (N − 2)). Note 03 measured only
  squares.
- "|det|" has no meaning for two shifts in d ≥ 3; there Theorem 10 gives a
  polynomial in the sides of degree d − 2.

**Interpretation of κ (Tested).** κ counts groups that are trapped at the
four corners of the box and never reach the bulk: for (2, 1), (1, 2) the
points (0, N−1) and (N−1, 0) have no link at all. `grid_lab.rs` measures
these trapped groups directly in corner windows and finds κ for every pair
tried.

## 2. The dominated shift is unique

Write p ⊑ q when every coordinate of p lies between 0 and the same
coordinate of q (Theorem 5 of note 03: p splits q on a box iff p ⊑ q, for q
with a non-empty family). Call a shift *active* if its family is non-empty.

**Proposition 12 (Proved here, Tested).** In a box system at most one active
shift, up to sign, is dominated; and for that shift p each other active
shift q has exactly one orientation with p ⊑ q̃.

*Proof.* If p and p′ are both dominated then p ⊑ εp′ and p′ ⊑ ε′p for signs
ε, ε′. The first gives |pᵢ| ≤ |p′ᵢ| for all i, the second the converse, so
|pᵢ| = |p′ᵢ|; with p ⊑ εp′ the signs agree wherever pᵢ ≠ 0, so p = εp′. If
p ⊑ q and p ⊑ −q, every pᵢ lies between 0 and qᵢ and between 0 and −qᵢ, so
p = 0. ∎

**Consequence for ordering.** The chain of dominated reductions is
deterministic: there is no choice of shift and no choice of orientation.
"Which dominated shift first" is therefore not a question. (The existence
test — the active shifts, up to sign, have a least element under ⊑ — is
Theorem 5 restated. ⊑ is the *conformal* order of Graver-basis theory; see
section 6.)

## 3. Which systems reduce completely

**Proposition 13 (Proof sketch, Tested on small boxes).** Let the active
shifts of (D, P) span a lattice of rank at least 2 and let every side of D
exceed 2·Σ_{q∈P} Σᵢ |qᵢ|. Then the dominated chain does not reach a system
without links: it stops at a core with at least two active shifts and no
dominated shift.

*Sketch.* A step replaces P by {p} ∪ {q̃ − p}, which generates the same
lattice, and lowers Σ_q Σᵢ|qᵢ| by at least Σᵢ|pᵢ| while at least two shifts
remain, since |q̃ᵢ − pᵢ| = |q̃ᵢ| − |pᵢ|. So the box shrinks by at most
Σ_q Σᵢ|qᵢ| in every side during that time, no shift becomes inactive, and a
set of rank ≥ 2 never shrinks to a single shift.

Together with Proposition 8 of note 03: **in large boxes a system is totally
reducible exactly when its shifts are collinear.** In small boxes shifts can
become inactive and the chain can finish; there the only criterion known is
to run the chain (at most n₁ + … + n_d steps).

**Reduce, then count the core (Proved here).** By Theorem 3 of note 02 every
dominated step is exact, so c(D, P) = Σ|F| + c(core). The core is counted

- as a product if it has no active shift,
- by Theorem 10 if it has one or two independent active shifts,
- otherwise by union-find over the core box only.

`count_hybrid` in `grid_lab.rs` does this and agrees with brute force on
every tested system (5,517 exhaustive small systems and 6,000 random ones in
1–4 dimensions).

**Three or more shifts of rank 2 in 2-D (Proof sketch, Tested).** For large
boxes c = [Z² : L] + κ′, where L is the lattice generated by the shifts and
κ′ counts the groups trapped at the corners (it depends only on the shifts).
Sketch: inside one coset any two points far from the boundary are joined by
a path that stays near the segment between them; a point near an edge but
far from the corners can step inward; what is left is a bounded
neighbourhood of each corner, which looks the same as in an infinite
quadrant. Tested on 1,199 random systems with three or four shifts. No
closed formula for κ′ and no explicit threshold are known.

## 4. Reduction when no shift is dominated

Reducing by a non-dominated shift is always a relaxation (Theorem 2) and
sometimes exact. Chains using such steps exist:

- **3 × 2 grid, shifts (1,0), (0,1) (Proved by direct check).** No shift is
  dominated. Reducing by (1,0) with (0,1) oriented as (0,1) gives the 2 × 2
  box with shifts (1,0), (−1,1), which is connected, so the step is exact;
  there (1,0) ⊑ (1,−1) is dominated, and the chain finishes.
- **2 × 2 grid, same shifts (Tested exhaustively).** No sequence of
  single-shift reductions, with any orientations, is exact.
- **Unit shifts on a × b, 2 ≤ a, b ≤ 6 (Tested).** An exact chain exists
  exactly when a or b is odd. The two steps above turn an a × b unit grid
  into an (a − 2) × b unit grid whenever a ≥ 3, which proves "odd ⇒ chain";
  the converse is only tested.

These steps are **not covered by any theorem here**. Using them is a
heuristic that has to be checked instance by instance, which costs as much
as solving the instance. On 1,522 small systems: the dominated chain
completes for 407, only a non-dominated exact chain exists for 273, none for
842.

## 5. Three independent shifts in three dimensions (counterexamples)

- **The count can grow with the box (lower bound Proved here, value
  Tested).** P = {(1,1,0), (1,1,1), (2,1,0)} has determinant 1. The shift
  (1,1,0) is dominated, and its reduction releases
  |F| = N³ − 2(N−1)²N + (N−2)²N = 2N points on an N × N × N cube, so
  c ≥ 2N. Measured: c = 2N + 1 for N = 6, 8, 10, 12, 16. So "|det| +
  constant" is false in dimension 3.
- **Theorem 10 does not extend to three shifts (Tested).** The alternating
  count points − links + squares − cubes gives 3 for that system at N = 10;
  the true count is 21. The planar-graph step of the proof has no
  three-dimensional analogue.
- **Observation.** Of 150 random independent triples with entries in
  [−2, 2], on cubes N = 10 … 16: 121 have a constant count, 29 an exactly
  linear one, none grows faster. The alternating count was right for exactly
  the 121 constant ones.

**Conjecture 14.** For d independent shifts in Zᵈ and cubes of side N, the
group count is, for large N, a polynomial in N of degree at most d − 2.
(True for d = 2 by Corollary 11; consistent with the d = 3 data; otherwise
open.)

## 6. A sufficient condition with no threshold, and the literature

**Proposition 15 (Proved here from a known property).** If P contains, up to
sign, the Graver basis of the lattice L it generates, then for every box the
groups are exactly the non-empty sets D ∩ (coset of L).

*Proof.* For x, y ∈ D in one coset, y − x ∈ L is a sum of Graver basis
elements g₁, …, g_r each ⊑ y − x (the conformal-sum property of Graver
bases). The partial sums x + g₁ + … + g_j have every coordinate between
those of x and y, so they stay in D. ∎

**Sources and how they relate.**

- *Graver bases.* The order ⊑ and the conformal-sum property are standard
  (J. E. Graver, *On the foundations of linear and integer linear
  programming I*, Math. Programming 9 (1975) 207–226 — cited from memory,
  not re-checked; the property was confirmed only from secondary
  descriptions found by web search). Proposition 15 is an immediate
  consequence, not a new result.
- *Bounded fibres.* F. Rapallo, R. Yoshida, *Markov bases and subbases for
  bounded contingency tables*, Ann. Inst. Statist. Math. 62 (2010) 785–805,
  arXiv:0905.4841 (abstract only): connectivity of lattice points under
  moves when cells are bounded, for the lattice of two-way tables. Related
  question, different lattice.
- *Two-dimensional Fine–Wilf theorems.* R. Giancarlo, F. Mignosi,
  *Generalizations of the periodicity theorem of Fine and Wilf*, CAAP '94,
  LNCS 787, doi:10.1007/BFb0017478 (abstract only; pages not confirmed);
  Simpson–Tijdeman and Mignosi–Restivo–Silva (note 04; abstracts only).
  These give conditions under which a function with given period vectors is
  constant, i.e. the case of one group.

**Not found:** Theorem 10, Corollary 11 and Proposition 12 as stated. The
search was a small number of web queries reading abstracts and summaries, so
it is **not exhaustive**; Theorem 10 is a short Euler-characteristic
argument and may well be known. No novelty is claimed.

## 7. Classification summary

| System | Group count | Status |
|---|---|---|
| no active shift | \|D\| | trivial |
| one shift | N(∅) − N({q}) | Proved (a forest of paths) |
| collinear shifts, any d | dominated chain to the end (Prop. 8) | Proved |
| two independent shifts, any d, any box | Theorem 10 | Proved |
| rank 2 in 2-D, ≥ 3 shifts, large box | index + corner constant | Proof sketch, Tested |
| anything else | dominated chain, then union-find on the core | Proved exact; cost depends on the core |
| three independent shifts in 3-D | no formula; can grow linearly | Counterexample, Conjecture 14 |

## 8. Measurements (`sh grid_lab.rs bench`)

Machine as in note 04; single runs.

| Shifts | Box | Groups | Links | Union-find | Reduce-then-count |
|---|---|---:|---:|---:|---:|
| (2,1), (1,2) | 3000² | 5 | 17,982,004 | 0.68 s | 7 µs (formula) |
| (40,3), (7,55) | 3000² | 2,221 | 17,685,505 | 0.68 s | 6 µs (formula) |
| (1,1), (2,2), (5,5) | 3000² | 5,999 | 26,952,030 | 0.43 s | 0.3 ms (chain, 2,999 steps) |
| (1,0), (3,1), (2,5) | 3000² | 1 | 26,964,013 | 0.93 s | 0.62 s (core by union-find) |
| (1,1,0), (2,3,1) | 200³ | 1,394 | 15,682,394 | 0.72 s | 12 µs (formula) |
| (1,1,0), (1,1,1), (2,1,0) | 200³ | 401 | 23,681,199 | 1.04 s | 0.86 s (core by union-find) |
| (1,0,2,0), (0,3,1,1) | 60⁴ | 45,549 | 24,224,220 | 0.94 s | 10 µs (formula) |
| 3, 5 (interval) | 8,000,000 | 1 | 15,999,992 | 0.22 s | 13 µs (4 divisions; 0.89 s when done as 7,999,996 single steps) |

The last row was a weakness of the first version, which took one step per
unit of length when the smallest shift is small. Runs of equal steps are now
done by one division (note 06, section 4).

Formula-only examples beyond brute force: (40,3), (7,55) on a 10⁹ × 10⁹ box
has 2,221 groups; two shifts on a 5-dimensional box of side 10⁶ are counted
exactly (about 2.5 × 10¹⁹ groups).

## 9. Open

1. Conjecture 14, and any formula for three independent shifts in 3-D.
2. A closed formula and threshold for the corner constant with three or
   more shifts in 2-D; a full proof of that statement.
3. Whether the unit-grid parity rule of section 4 holds for all sizes.
4. A full write-up of Proposition 13.
5. Whether Theorem 10 is in the literature.
