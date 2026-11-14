# 04 — Computational evidence, certificates, sources and open questions

**Scope.** What was actually run and what it showed; the certificate
formats and what an independent checker must verify; the literature that was
checked and how far; the status of every claim; open questions. Status
labels as in note 01.

Machine used for all measurements: Intel Xeon (Ivy Bridge), 8 cores, 31 GB,
Ubuntu Linux (kernel 7.0), `rustc 1.100 -O`. Single runs. Times will differ
elsewhere.

## 1. Commands

| Command | What it does |
|---|---|
| `sh prefsuf.rs` | 56 self-tests (49 when this note was written), then two demos |
| `sh hard.rs 1` | planted-solution stress test |
| `sh adversarial.rs check 1` | ~115,000 adversarial checks |
| `sh proof_lab.rs check 1` | derived contradiction certificates, independent checker |
| `sh theory_lab.rs` | experiments E1–E5 below |
| `sh theory_lab.rs big N` | partition certificates at length N |
| `sh theory_lab.rs table` | group counts of irreducible two-shift grids |
| `tools/run_tests.sh start` | the first five, detached; about 80 s in total here, compilation included |

Checks not applied: `rustfmt --check` fails on every `.rs` file (the
project has never been formatted with it and has no formatting
configuration), so formatting was left as it is. There is no build system;
each file compiles itself with `rustc`. The compiler reports no warnings for
`prefsuf.rs`.

## 2. Results of `theory_lab.rs`

All "must hold" checks passed (`theory checks failed: 0`).

**E1 — reduction on point sets (Theorems 2, 3, 5).**

| Systems | Steps | Dominated (all exact) | Not dominated: exact / inexact | Systems with no dominated shift |
|---|---:|---:|---:|---:|
| intervals, n ≤ 9, every shift set (502) | 1,793 | 502 | 466 / 825 | 0 |
| 2-D boxes ≤ 4×4 and 5×3, ≤ 3 shifts (5,325) | 15,067 | 2,035 | 652 / 12,380 | 3,290 |
| 3-D boxes ≤ 3×3×2, ≤ 2 shifts (1,047) | 2,022 | 429 | 18 / 1,575 | 618 |
| random subsets of a 5×5 grid (3,981) | 7,552 | 1,531 | 48 / 5,973 | 2,423 |

- In no step did the reduced description force an equality that is not true
  (Theorem 2).
- Every dominated step was exact and satisfied the counting formula
  (Theorem 3).
- On boxes the definition of "dominated" and the coordinate test agreed on
  every step (Theorem 5).
- Exactness without domination does occur (the "exact" entries of column 4),
  so domination is not necessary.

A failed experiment worth recording: the first statement of Theorem 3
ignored orientations and was refuted here (2,035 dominated steps, only 1,728
exact). The theorem in note 02 is the corrected one.

**E2 — mirror constraint (Theorem M).** One step exact on all 4,095 systems
with n ≤ 12; repeated reduction gives the right count there and on 3,000
random systems up to n = 3,000.

**E3 — weights.** Offsets in Z₃ with equality semantics: solution sets
rebuilt exactly for 1,275 systems (640 inconsistent). Inequality and cost
counterexamples of note 02 confirmed.

**E4 — repeated reduction on grids.** Note 03, sections 4–5.

**E5 — partition certificates.** Section 3 below.

## 3. Certificates

### 3.1 Contradiction certificate (existing)

*Claim:* the puzzle has no solution. *Certificate:* two positions fixed to
different letters by cited prefix/suffix rules, and a chain of links between
them, each citing a border rule. *Checker* (`cert_ok` in `adversarial.rs`,
`check_conflict` in `prefsuf.rs`): each end is fixed as claimed; each
consecutive pair is a link of the cited rule. Linear in the chain. The
checker does not need the chain to be shortest.

### 3.2 Partition certificate (new, `theory_lab.rs`)

*Claim:* the groups of positions forced equal by the border rules are
exactly the classes of a given labelling. Together with the letters of the
prefix/suffix words this determines the whole answer of a solvable puzzle.

*Certificate:* `labels` (one number per position) and `forest` (a list of
original links, each with its rule).

*Checker* (`check_certificate`), which never calls the solver:

1. every forest link (u, v, rule) satisfies: the rule is `Border(k)`, and
   v = u + (n − k) with u < k;
2. a union-find over the forest links yields exactly the classes of
   `labels` — so positions with equal labels are provably forced equal;
3. the word `labels` has every border the rules ask for, decided with the
   classical border table (failure function) of the word — so `labels` is
   itself a solution over a large alphabet in which different classes hold
   different letters, hence no two classes are forced equal.

Steps 1–2 give soundness of the merging, step 3 gives completeness. Cost:
O((n + m) α(n)) for n positions and m rules; step 3 is linear and uses only
the definition of a border.

`check_pattern` then verifies a solver answer cell by cell from the
certified groups: one letter per group from the words, free groups numbered
in order of first appearance.

*Evidence (Tested).*

- 20,000 random puzzles (n ≤ 1,540): every emitted certificate accepted.
- 46,174 tampered certificates (two classes merged; one class split; a
  forest link removed; a forged link between two classes): every one
  rejected.
- For n ≤ 12, 2^(number of labels) equals the number of two-letter texts
  with all the borders, by enumeration.
- 6,667 solver answers for solvable puzzles confirmed from certified groups.

*At scale (measured).* n = 5,000,000:

| Border set | Borders | Groups | Emit | Check |
|---|---:|---:|---:|---:|
| none | 0 | 5,000,000 | 0.15 s | 1.68 s |
| one (half) | 1 | 2,500,000 | 0.19 s | 0.72 s |
| 16 random | 16 | 1 | 0.52 s | 0.28 s |
| 1,000 random | 1,000 | 1 | 0.52 s | 0.21 s |
| every border | 4,999,999 | 1 | 0.59 s | 0.22 s |
| even periods ≥ n/2 | 1,250,000 | 2 | 0.36 s | 0.18 s |
| block twice, block period 3 | 833,334 | 3 | 0.37 s | 0.17 s |
| two coprime periods at the Fine–Wilf bound | 2 | 1 | 0.32 s | 0.16 s |

This closes a gap recorded in `resultssummary.md`: solvable puzzles with
millions of borders had only had their number of free groups compared with a
predicted value. Their groups are now confirmed by an independent checker.

*Caveat.* The checker's cost is proved. The **emitter's** cost is not: the
forest comes from the same derivation as the fallback contradiction proofs,
whose cost is only measured (section 5).

## 4. Status of every claim

| Claim | Status |
|---|---|
| Interval reduction theorem and bijection (note 01, Thm 1) | Proved here; Literature (Holub 2013; Tijdeman–Zamboni Algorithm B) |
| Grouping runs in O((n + m) log(n + m)) | Proved here |
| Reduction is always a relaxation (Thm 2) | Proved here; Tested |
| Reduction is exact under domination, with oriented shifts (Thm 3) | Proved here; Tested |
| Coordinate criterion for boxes (Thm 5) | Proved here; Tested |
| No dominated shift need exist for d ≥ 2 (Prop. 7) | Proved here (explicit example); Tested |
| Collinear shift sets reduce completely (Prop. 8) | Proved here; Tested |
| Mirror extension (Thm M) | Proved here; Tested |
| Weighted shifts with equality semantics | Proved here; Tested for Z₃ |
| Reduction fails for inequality weights and for costs | Counterexamples; Tested |
| Group count of irreducible two-shift grids is \|det\| + κ | **Conjecture** (Observed on 7 pairs, N ≤ 48) |
| Fallback contradiction proofs are valid | Tested per instance by an independent checker |
| Fallback / forest derivation is near-linear | **Observed only** (5–8 work units per character up to n = 102,400 in a hill-climbing search); no proof |
| Shortest contradiction proofs need quadratic time | Open |
| Any connection to NP-complete problems | **None established.** The puzzle is in P; no reduction from an NP-complete problem was attempted. |

## 5. Proof-construction cost: what is and is not known

**Update.** Note 06 proves a bound: there are at most 2 log₂ n + 2 phases,
and the fallback explanation is now a search over at most that many
"generator" borders, costing O(n log n) link tests. The text below describes
the earlier derivation, which is kept only as a further fallback.

- The shortest-chain search is bounded by the total number of links, which
  can grow with n²; it is cut off by a fixed budget.
- The derivation used beyond the budget is correct by construction
  (every kept link is original; the kept links form a forest) and its output
  is independently checked, but **its running time has no proved bound
  better than polynomial**. The number of phases and the recursion depth are
  not bounded by anything proved either (largest seen: 18).
- Nothing in notes 01–03 changes that. Theorem 3's proof shows each derived
  link costs two links one level up, which is the source of the potential
  blow-up; the union-find memoisation is what keeps the measured cost low.

## 6. Sources

How far each source was read is stated; "metadata" means title, authors,
venue and identifiers were confirmed from publisher or index pages, not the
text.

**Exact matches for the interval theorem**

- Š. Holub, *On an algorithm for multiperiodic words*, Acta Polytechnica
  53(4) (2013) 344–346; arXiv:1212.6609. **Read in full.** Lemma 1,
  Corollary 1, Lemma 2 are Theorem 1 of note 01.
- R. Tijdeman, L. Zamboni, *Fine and Wilf words for any periods*, Indag.
  Math. (N.S.) 14(1) (2003) 135–147; and *Fine and Wilf words for any
  periods II*, Theoret. Comput. Sci. 410(30–32) (2009) 3027–3034. Metadata
  and Holub's description only ("Algorithm B", Reduction/Extension).
- Š. Holub, *On multiperiodic words*, RAIRO Theor. Inform. Appl. 40(4)
  (2006) 583–591, doi:10.1051/ita:2006042; corrigendum 45(4) (2011)
  467–469. Metadata only.

**Periodicity with several periods**

- N. J. Fine, H. S. Wilf, *Uniqueness theorems for periodic functions*,
  Proc. Amer. Math. Soc. 16 (1965) 109–114,
  doi:10.1090/S0002-9939-1965-0174934-9. Metadata.
- M. G. Castelli, F. Mignosi, A. Restivo, *Fine and Wilf's theorem for three
  periods and a generalization of Sturmian words*, Theoret. Comput. Sci.
  218(1) (1999) 83–94, doi:10.1016/S0304-3975(98)00251-5. Abstract.
- J. Justin, *On a paper by Castelli, Mignosi, Restivo*, RAIRO Theor.
  Inform. Appl. 34(5) (2000) 373–377, doi:10.1051/ita:2000122. Metadata.
- S. Constantinescu, L. Ilie, *Generalised Fine and Wilf's theorem for
  arbitrary number of periods*, Theoret. Comput. Sci. 339(1) (2005) 49–60,
  doi:10.1016/j.tcs.2005.01.007. Abstract.
- S. A. Rankin, *Fine–Wilf graphs and the generalized Fine–Wilf theorem*,
  arXiv:0906.1780 (2009). Abstract.
- L. J. Guibas, A. M. Odlyzko, *Periods in strings*, J. Combin. Theory
  Ser. A 30(1) (1981) 19–42, doi:10.1016/0097-3165(81)90038-8. Metadata.
- E. Rivals, S. Rahmann, *Combinatorics of periods in strings*, J. Combin.
  Theory Ser. A 104(1) (2003) 95–113. Metadata (no DOI confirmed).

**Multidimensional periodicity (related, not exact matches)**

- R. J. Simpson, R. Tijdeman, *Multi-dimensional versions of a theorem of
  Fine and Wilf and a formula of Sylvester*, Proc. Amer. Math. Soc. 131
  (2003) 1661–1671, https://www.ams.org/proc/2003-131-06/S0002-9939-03-06970-3/.
  Abstract.
- F. Mignosi, A. Restivo, P. V. Silva, *On Fine and Wilf's theorem for
  bidimensional words*, Theoret. Comput. Sci. 292 (2003) 245–262,
  doi:10.1016/S0304-3975(01)00226-2. Abstract. (Author order differs between
  index records.)
- A. Amir, G. Benson, *Two-dimensional periodicity in rectangular arrays*,
  SIAM J. Comput. 27(1) (1998) 90–106, doi:10.1137/S0097539795298321.
  Abstract.

**Congruence closure and proof production (analogy)**

- G. Nelson, D. C. Oppen, *Fast decision procedures based on congruence
  closure*, J. ACM 27(2) (1980) 356–364, doi:10.1145/322186.322198.
  Metadata.
- P. J. Downey, R. Sethi, R. E. Tarjan, *Variations on the common
  subexpression problem*, J. ACM 27(4) (1980) 758–771,
  doi:10.1145/322217.322228. Metadata.
- R. Nieuwenhuis, A. Oliveras, *Proof-producing congruence closure*, RTA
  2005, LNCS 3467, 453–468, doi:10.1007/978-3-540-32033-3_33 (DOI from one
  index only); and *Fast congruence closure and extensions*, Inform. and
  Comput. 205(4) (2007) 557–580, doi:10.1016/j.ic.2006.08.009. Metadata.
  The explanation forest kept by the derivation in `prefsuf.rs` is in the
  spirit of this work; no claim of equivalence is made.

**Cited from memory, not re-checked in this work**

- D. E. Knuth, J. H. Morris, V. R. Pratt, *Fast pattern matching in
  strings*, SIAM J. Comput. 6(2) (1977) 323–350 (the border table used by
  the certificate checker).
- R. E. Tarjan, *Efficiency of a good but not linear set union algorithm*,
  J. ACM 22(2) (1975) 215–225 (union-find bound).
- Graph 3-colouring is NP-complete (Karp 1972; Garey and Johnson 1979):
  the reason "must differ" constraints over three or more letters leave the
  scope of this technique.

## 7. Open questions

1. A bound on the derivation's cost and on the number of phases.
2. Conjecture 9 of note 03 and a description of irreducible cores with more
   than two shifts.
3. Necessary and sufficient conditions for total reducibility of box
   systems.
4. Whether shortest contradiction proofs can be found in subquadratic time.
5. Whether the n log n term of the grouping bound is attained.
6. Whether Theorems 3, 5 and M are known under other names.
7. A machine-checked proof of Theorems 1 and 3.

## 8. Next steps (concrete)

- Implement the dominated-reduction counter for boxes as a small library
  function with the Proposition 6 bound and test it at sizes where brute
  force is impossible, using collinear families where the answer is known.
- Attack Conjecture 9 for (2,1), (1,2) by hand, then generalise.
- Emit partition certificates from `prefsuf.rs` itself (a `--certificate`
  option) and move `check_certificate` into a stand-alone checker program.
- Search adversarially for border sets with many phases, to test the
  recursion-depth handling of the derivation.
