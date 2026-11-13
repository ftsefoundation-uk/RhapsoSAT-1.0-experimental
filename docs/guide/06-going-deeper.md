# 6. Going deeper (optional)

[Guide index](README.md) · previous: [Words used](05-words-used.md)

Everything below is optional. It is written for readers comfortable with
mathematics or programming.

## The files

| File | Role |
|---|---|
| `prefsuf.rs` | The solver, with its own self-tests |
| `certcheck.rs` | The separate certificate checker |
| `demo/index.html` | The teaching demo |
| `hard.rs`, `adversarial.rs` | Stress tests and adversarial tests, with their own brute-force reference |
| `proof_lab.rs`, `theory_lab.rs`, `grid_lab.rs`, `reduce_lab.rs`, `cert_lab.rs` | Experiments and checks behind the research notes |
| `bench.rs` | A small timing program |
| `tools/run_tests.sh` | Runs all test suites |

Each `.rs` file compiles itself when run with `sh`.

## The research notes

In [docs/research](../research), in reading order:

| Note | Subject |
|---|---|
| 01 | The solver and the reduction theorem it rests on (a known result) |
| 02 | A general framework: when a reduction step is exact |
| 03 | Grids and higher dimensions |
| 04 | Evidence, sources, open questions |
| 05 | An exact count for two shifts on grids and boxes |
| 06 | Reduction before grouping, generators, cost of explanations, certificates |
| 07 | How many generators there can be; shorter explanations |

[resultssummary.md](../../resultssummary.md) is an earlier report on
adversarial testing.

## How to read the claims

Every statement in the notes carries a label: proved (a written argument),
literature (found in a published source), tested (checked by a program on a
stated range), observed or measured, conjecture, or open. Proofs are prose,
not machine-checked. Timings are single runs on one machine.

## Known limits

- Texts of 2³² − 1 characters or more are refused by the certificate tools.
- Explanations of contradictions are valid but not always shortest.
- Several questions are open; note 07 lists them in order of importance.
