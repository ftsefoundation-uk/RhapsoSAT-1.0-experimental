# RhapsoSAT 1.0 experimental

RhapsoSAT 1.0 experimental is a small, fast solver for one kind of text
puzzle, together with tools that let you check its answers independently.

**Try the demo:** <https://ftsefoundation-uk.github.io/RhapsoSAT-1.0-experimental/>
(an interactive page that shows the main idea; nothing to install).

## What it does

You describe an unknown text with three kinds of rule:

- the text **starts with** a given word,
- the text **ends with** a given word,
- the **first k characters are the same as the last k** characters.

The solver then tells you one of three things:

1. there is **exactly one** text that fits, and what it is;
2. there are **several**, with a pattern that describes all of them and
   their exact number;
3. there is **none**, with a short chain of reasons you can check by hand.

For example (abridged: the full report also restates the puzzle first and
ends with a double-check line):

```text
$ sh prefsuf.rs 7 ab B4 Pa Sb

Result: NO SOLUTION - no text can fit all the rules; they contradict each other.

  Why
    Rule 2 needs character 1 to be "a".
    Rule 3 needs character 7 to be "b".
    But those two characters must hold the same letter, because:
      character 1 = character 4   (rule 1)
      character 4 = character 7   (rule 1)
```

It handles texts of millions of characters in well under a second, because
it never looks at the individual "must be equal" relationships one by one.

## Try it yourself

You need a Unix-like system (Linux, macOS, …) with the Rust compiler
`rustc`. There is nothing to build or install: each file compiles itself.

```sh
sh prefsuf.rs 8 ab B5 Pab Sb     # length 8, letters a and b, three rules
sh prefsuf.rs --help             # all options
sh prefsuf.rs                    # run the built-in self-tests and two examples
```

`B5` means "the first 5 characters equal the last 5", `Pab` "starts with
ab", `Sb` "ends with b".

## Read more

| If you want… | Read |
|---|---|
| A plain-language tour | [docs/guide](docs/guide/README.md) |
| Every option and output format | [docs/reference/user-manual.md](docs/reference/user-manual.md) |
| How an answer can be checked without trusting the solver | [docs/guide/04-checking-answers.md](docs/guide/04-checking-answers.md) |
| The mathematics, proofs and measurements | [docs/research](docs/research) |
| The demo's own notes | [demo/README.md](demo/README.md) |

## What "experimental" means here

- **It solves this puzzle only.** Despite the name, it is not a general SAT
  solver: it does not read Boolean formulas, and it does not handle "must be
  different" rules or other kinds of constraint.
- **No claim about hard problems.** The puzzle can be solved quickly by
  ordinary methods. Nothing here bears on NP-complete problems or on P
  versus NP.
- **Proofs are written, not machine-verified.** The research notes label
  every statement as proved, tested, measured, conjectured or open.
- **The demo is a teaching page**, not the solver. It shares no code with
  it and shows small examples only.

## Tests

```sh
tools/run_tests.sh run      # every test suite, in the foreground
```

Nine suites: the solver's self-tests, stress and adversarial tests, the
checks behind the research notes, and command-line checks of both programs
(`tools/test_cli.sh`: exit statuses and verdict lines). See the user manual
for details.

## Licence

Released under the [MIT License](LICENSE).
