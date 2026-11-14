> This is the full user manual carried over from development. For a gentler
> start, read the [guide](../guide/README.md).

# prefsuf — a "what could this text be?" puzzle solver

`prefsuf` solves one specific kind of word puzzle, quickly and with an
explanation you can check.

You describe an unknown piece of text by giving:

- how long it is,
- which letters it may use, and
- a few **rules** about how it starts, how it ends, and whether its beginning
  repeats at its end.

The program then tells you one of three things:

| Result | Meaning |
| --- | --- |
| **ONE SOLUTION** | The rules pin the text down completely. The program prints it. |
| **SEVERAL SOLUTIONS** | More than one text fits. The program says exactly how many, and shows a pattern that describes all of them. |
| **NO SOLUTION** | The rules contradict each other. The program shows a short step-by-step reason why. |

It is a small, self-contained demonstration of a technique (explained
[below](#how-it-works)), not a solution to any particular real-world task.

## The three kinds of rule

| You type | Meaning | Example |
| --- | --- | --- |
| `Pword` | The text **starts** with `word` (a *prefix*). | `Pab` – starts with "ab" |
| `Sword` | The text **ends** with `word` (a *suffix*). | `Sb` – ends with "b" |
| `Bk` | The first `k` characters are **the same as** the last `k` characters (a *border*). | `B5` – the first 5 characters equal the last 5 |

The border rule is the interesting one. In the 8-letter text `abcabcab`, the
first 5 characters are `abcab` and the last 5 characters are also `abcab`, so
it satisfies `B5`. Notice that the two halves overlap in the middle; that
overlap is what forces the text to repeat itself (here, every 3 characters).

## Running it

You need a Unix-like system (Linux, macOS, Termux on Android, …) with the Rust
compiler `rustc` installed. Nothing else: no build tool, no downloads, no
extra libraries.

```sh
sh prefsuf.rs LENGTH LETTERS [RULE]...
```

- `LENGTH` – how many characters the unknown text has, e.g. `8`
- `LETTERS` – the letters it may use, written together, e.g. `ab`
- `RULE` – any number of `Pword`, `Sword`, `Bk` rules, in any order

The file is a Rust program that is also a tiny shell script: `sh prefsuf.rs`
compiles it (this takes a few seconds, each time) and then runs it. If you
prefer, compile it once yourself with
`rustc --edition 2021 -O prefsuf.rs -o prefsuf` and run the resulting
`./prefsuf` with the same arguments.

Options:

| Option | Effect |
| --- | --- |
| `--full` | Do not shorten long answers. |
| `--compact` | Short technical output in the program's original format (see [Compact output](#compact-output)). |
| `--certificate` | Print a certificate of the answer instead of the report. `sh certcheck.rs FILE` confirms it without solving the puzzle (see `../certificate-format.md`). |
| `--help` | Show a usage summary. |

Running `sh prefsuf.rs` with nothing after it runs the built-in self-tests and
then two demonstration puzzles.

## Reading the output

### Example 1 – several solutions

"An 8-character text made of `a` and `b`. Its first 5 characters equal its
last 5, it starts with `ab`, and it ends with `b`."

```sh
sh prefsuf.rs 8 ab B5 Pab Sb
```

```
The puzzle
  Length of the text : 8 characters
  Allowed letters    : a b
  Rules
    1. The first 5 characters are the same as the last 5 characters.
    2. The text starts with "ab".
    3. The text ends with "b".

Result: SEVERAL SOLUTIONS - 2 different texts fit all the rules.

  Pattern  : a b ?1 a b ?1 a b
  Example  : abaabaab

  How to read the pattern (one entry per character, left to right)
    A plain letter is forced by the rules.
    ?1, ?2, ... are open choices. Each can be any allowed letter, but
    every place with the same number must get the same letter.

  Open choices: 1 (each has 2 possible letters)
    ?1 = characters 3, 6

  Double-check: the example was tested directly against every rule: confirmed.
```

What this says: characters 1, 2, 4, 5, 7 and 8 are fully decided. Characters 3
and 6 are not decided, but they must be equal to each other. Filling `?1` with
`a` gives `abaabaab`; filling it with `b` gives `abbabbab`. Those are the only
two texts that fit, and both do.

- **Pattern** – one entry per character. A letter is certain; a `?number` is
  an open choice.
- **Example** – one text that fits (every open choice filled with the first
  allowed letter).
- **Open choices** – how many independent decisions are left, and which
  characters each one controls. The number of solutions is always
  *(number of allowed letters)* multiplied by itself once per open choice.
  When that number is too big to print, it is shown as, for example, `3^250`.
- **Double-check** – the example text was tested against your rules by a
  separate piece of code that does not rely on the solver (it compares the
  words directly, and checks the border rules by direct comparison or, when
  their total length is large, with the classical border table of the
  example). It confirms that the example fits; it does not re-derive the
  pattern or the count. For an independent check of the groups and the
  count, use `--certificate` and `certcheck.rs`.

### Example 2 – exactly one solution

```sh
sh prefsuf.rs 6 ab B4 Pab
```

Abridged (the report starts by restating the puzzle, as in Example 1, and
ends with the double-check line):

```
Result: ONE SOLUTION - exactly one text fits all the rules.

  The text : ababab
```

### Example 3 – no solution

```sh
sh prefsuf.rs 7 ab B4 Pa Sb
```

Abridged (the report starts by restating the puzzle):

```
Result: NO SOLUTION - no text can fit all the rules; they contradict each other.

  Why
    Rule 2 needs character 1 to be "a".
    Rule 3 needs character 7 to be "b".
    But those two characters must hold the same letter, because:
      character 1 = character 4   (rule 1)
      character 4 = character 7   (rule 1)

  Double-check: each step of this explanation was re-checked against the rules: confirmed.
```

The "Why" section is a complete argument, not a hint: every line can be
checked by hand against the rules. The program gives the shortest such chain
of "this character = that character" steps, except for puzzles with so many
different border rules that finding the shortest chain would take too long.
In that case it gives a chain that is still a complete, checkable argument
but may be longer than necessary (see "How fast it is" below).

In these reports characters are numbered from 1 (the first character is
character 1), and rules are numbered in the order you typed them.

Very long answers are shortened (the start and end are shown, with a note of
how much was left out). Add `--full` to print everything.

### Compact output

`--compact` prints the program's original terse format, which is handy for
scripts. It uses technical terms, and it counts positions **from 0**, not 1:

```
n = 8, alphabet = ab, constraints = [Border(5), Prefix(ab), Suffix(b)]
status   MULTIPLE
pattern  a b ?0 a b ?0 a b
free     1 class(es)
count    2
witness  abaabaab
checked  witness satisfies definition: true
```

`status` is `UNIQUE`, `MULTIPLE` or `UNSAT` (short for "unsatisfiable": no
solution). `free … class(es)` is the number of open choices, `witness` is the
example, and for `UNSAT` the `reason` lines are the explanation (the
"certificate").

## How it works

The program never tries candidate texts one by one. With 2 letters and 100
characters there are already about a million million million million million
candidates, so that approach could not work. Instead it reasons about the *positions* in the
text. There are four steps.

**1. Find the positions that must be equal.**
A border rule says the start of the text is repeated at the end. If the first
5 of 8 characters equal the last 5, then character 1 = character 4,
character 2 = character 5, and so on: the text repeats itself every 3
characters (3 = 8 − 5). The program gathers positions that are forced to hold
the same letter into **groups**. If character 1 = character 4 and
character 4 = character 7, all three land in one group.

With several border rules this is done from the outside in, using a simple
fact: *a border of a border is a border*. Take the longest border. The
characters after it are copies of earlier characters, so each is linked to
the earlier character it copies. Every shorter border of the whole text is
then just a border of that first part, so the program repeats the step on the
first part alone, and so on inward. Each character is linked at most once, no
matter how many border rules there are, which is why even hundreds of
thousands of border rules cost little more than one.

**2. Write in the known letters.**
"Starts with `ab`" fixes characters 1 and 2. Because of the groups, fixing one
character fixes every character in its group.

**3. Look for a clash.**
If one group is asked to be two different letters, no text can fit. The
program then finds a chain of "this = that" links between the two clashing
characters - the shortest one whenever that can be found quickly - and prints
it as the explanation (see Example 3). A
separate checker confirms each link really does follow from one of your
rules.

**4. Read off the answer.**
Groups that received a letter are decided. Every group that did not is one
open choice. If there are none, there is exactly one solution. Otherwise each
open choice can independently be any allowed letter, which is where the exact
count comes from.

The answer is complete: every way of filling the open choices is a solution,
and there are no other solutions.

### How fast it is, and where it stops being fast

Measured on one machine (Intel Xeon, Linux, `rustc 1.100 -O`). Your numbers
will differ. "Before" is the original program this project started from.

Text length 2,000,000, fastest of 5 runs, scenarios from `bench.rs`
(peak memory is for the whole benchmark process):

| Scenario (`bench.rs` name) | Time before | Time now | Peak memory before | now |
| --- | ---: | ---: | ---: | ---: |
| two long words, no borders (`sparse`) | 0.329 s | 0.023 s | 269 MB | 96 MB |
| one word covering everything, 94 letters (`fixed`) | 0.161 s | 0.031 s | 202 MB | 98 MB |
| one border, repeat distance 1 (`chain`) | 0.178 s | 0.032 s | 353 MB | 96 MB |
| one border covering half (`half`) | 0.502 s | 0.030 s | 435 MB | 96 MB |
| two borders (`p2p3`) | 0.265 s | 0.033 s | 384 MB | 96 MB |
| border + word (`period_fixed`) | 0.181 s | 0.029 s | 353 MB | 96 MB |
| the same border 10 times (`dup10`) | 1.929 s | 0.032 s | 1,217 MB | 111 MB |
| 16 random borders (`rand16`) | 2.740 s | 0.032 s | 1,059 MB | 111 MB |
| contradiction, long explanation (`late_unsat`) | 1.309 s | 0.082 s | 567 MB | 143 MB |
| contradiction (`early_unsat`) | 0.240 s | 0.020 s | 294 MB | 21 MB |
| contradiction, no borders (`direct_unsat`) | 0.100 s | 0.013 s | 142 MB | 21 MB |
| every possible border, length 20,000 (`allborders`) | 27.06 s | 0.0004 s | 8,297 MB | 4 MB |

**Finding the answer** (steps 1, 2 and 4) takes time proportional to the text
length plus the total length of the words, plus a small extra for sorting the
border rules. In `adversarial.rs perf` runs, every solvable shape tried —
including a 5,000,000-character text with 4,999,999 different border rules —
finished in under 0.4 seconds.

**Double-checking the example** (the last line of a report) used to compare
every border rule character by character, which costs the total length of
all the borders: the whole report took 12.1 s for a 2,000,000-character text
with the 100,000 border rules `B1900000` … `B1999999` and `Pa`. It now uses
the border table of the example text when that is cheaper, so it too takes
time proportional to the text length: the same report takes 0.17 s.
`--certificate` never ran this check.

**Explaining a contradiction** (step 3) is done in one of two ways.

*Search.* The program first searches for the *shortest* chain of links. With
a few border rules this is quick (a 5,000,000-step explanation takes about
0.15 s). With very many different border rules the search can have to look at
a number of links that grows with the square of the text length, so it is
given a fixed allowance of work: 200 million link tests plus 32 per character
of the text (well under a second on the test machine).

*Derivation.* If the allowance runs out, the program stops searching and
instead works out the chain from the same reasoning it used to group the
positions in step 1: every link made there is a consequence of the border
rules, and the program unfolds those consequences back into original links.
The chain it gets this way is a complete argument that can be checked like
any other, but it is usually longer than the shortest one. A quick pass then
shortens it wherever two positions further apart are directly linked.

The deliberately nasty puzzle that used to take 31 seconds to explain
(125,000 border rules on a 500,000-character text; `adversarial.rs` shape
`mult4_2_unsat`) now takes about 0.4 s, and 1.6 s at 5,000,000 characters,
with the same 3-position explanation as before. In the measurements made so
far the derivation's work grows in step with the text length; that has been
observed, not proved. `proof_lab.rs` contains the experiments.

## Files

| File | What it is |
| --- | --- |
| `prefsuf.rs` | The solver, the report, and the built-in self-tests. This is the only file you need. |
| `hard.rs` | A stress test. It builds large puzzles whose correct answer is known in advance (by a method that shares no code with the solver) and checks the solver against them. Run: `sh hard.rs` (optionally `sh hard.rs SEED MAX_LENGTH`). |
| `bench.rs` | A timing and memory measurement tool. Run: `sh bench.rs SHAPE LENGTH NUMBER_OF_LETTERS REPEATS`, e.g. `sh bench.rs chain 1000000 2 3`. Reads `/proc`, so Linux only. |
| `adversarial.rs` | Deliberately hostile tests. It checks the solver against answers worked out independently (trying every text on small puzzles; a separate grouping method on large ones) across awkward cases: borders that nearly clash, thousands of borders at once, contradictions millions of steps long, malformed input. Run: `sh adversarial.rs` (optionally `sh adversarial.rs check SEED`), or `sh adversarial.rs perf SHAPE LENGTH` for one timing. |
| `proof_lab.rs` | Experiments on how explanations are built: where the time of a contradictory puzzle goes (`trace`), explanations built by derivation and judged by the independent checker of `adversarial.rs` (`derive`, `check`), and searches for inputs that make the derivation expensive (`cost`, `hunt`). Run: `sh proof_lab.rs check`, or `sh proof_lab.rs trace mult4_2_unsat 100000`. |
| `theory_lab.rs` | Machine checks of the mathematics in `../research/`: the reduction theorem on intervals, grids and higher-dimensional boxes, its extensions and counterexamples, and "partition certificates" that let a separate checker confirm the solver's groups without trusting it. Run: `sh theory_lab.rs`. |
| `grid_lab.rs` | Shift systems on grids and higher-dimensional boxes (`../research/05`): an exact group count for two independent shifts, reduce-then-count, counterexamples in three dimensions, checked against two independent brute-force counters. Stand-alone. Run: `sh grid_lab.rs`; `sh grid_lab.rs bench`; `sh grid_lab.rs count 3,3 2,1 1,2`. |
| `reduce_lab.rs` | Reduction before grouping (`../research/06`): grouping strategies and explanation strategies compared, with correctness checks. Run: `sh reduce_lab.rs`; `sh reduce_lab.rs bench`; `sh reduce_lab.rs proofs`. |
| `certcheck.rs` | Stand-alone checker for certificates printed by `sh prefsuf.rs --certificate ...`. Shares no code with the solver. Run: `sh certcheck.rs FILE [LENGTH LETTERS RULE...]`. |
| `cert_lab.rs` | Tests of certificates and checker: valid, damaged, forged, malformed, boundary cases, long texts. Run: `sh cert_lab.rs`; `sh cert_lab.rs big`. |
| `tools/test_cli.sh` | Command-line checks of `prefsuf.rs` and `certcheck.rs`: exit statuses, the verdict line, and failures (bad arguments, output that cannot be written) that must not crash. Run: `sh tools/test_cli.sh` (about a minute: each call recompiles). |
| `tools/run_tests.sh` | Runs every test suite in the background so that it keeps going if your terminal disconnects (see "Testing"). |
| `../research/` | Research notes: 01 the solver and its theorem, 02 a general framework, 03 grids and higher dimensions, 04 evidence, sources and open questions, 05 exact counts on grids and boxes, 06 reduction first, generator borders, explanation cost, certificates, 07 number of generators and shorter explanations. `../certificate-format.md` describes the certificate. |
| `demo/` | A small interactive web page (`demo/index.html`, open it in a browser) that explains shifts, groups and the reduction to non-specialists. A teaching example; it shares no code with the solver. |
| `resultssummary.md` | Earlier reports: adversarial testing, the proof-construction study, the theorem. |

`hard.rs`, `bench.rs`, `adversarial.rs`, `proof_lab.rs`, `theory_lab.rs`,
`reduce_lab.rs` and `cert_lab.rs` must sit in the same folder as
`prefsuf.rs`; `cert_lab.rs` also needs `certcheck.rs` and `proof_lab.rs`
also needs `adversarial.rs`. `grid_lab.rs` and `certcheck.rs` stand alone.

## Testing

`sh prefsuf.rs` (no arguments) runs 56 self-tests. They include comparing the
solver against plain trial-and-error on several thousand tiny puzzles — small
enough that trying every possible text is feasible — and checks of the report
wording. The command exits with status 0 only if all pass.

`sh hard.rs` runs roughly 25,000 larger randomly generated puzzles (up to
500,000 characters by default) and prints `failures: 0` on success.

`sh adversarial.rs` runs about 115,000 checks in seven groups (A–G) and prints
`total failures: 0` on success. It takes under a minute and needs about 200 MB
of memory, because some of its puzzles are 5,000,000 characters long.

`sh tools/test_cli.sh` runs the solver and the checker as a user would and
checks their exit statuses (0 answer or VALID, 1 REJECTED, 2 unreadable
input, bad command line, or output that cannot be written, 3 a failed
internal double-check) and that neither crashes. The double-check case uses
a copy of the solver whose double-checks are made to fail (fault injection
inside the test; the real file is not changed).

### Running everything in the background

```sh
tools/run_tests.sh start     # start all suites, detached (about three minutes)
tools/run_tests.sh status    # running? finished? result and exit status
tools/run_tests.sh log -f    # watch the log
tools/run_tests.sh wait      # block until done; exits with the run's status
```

The run is started in a detached `tmux` session (or with `nohup` if `tmux` is
not installed), so it continues if you close the terminal or lose the
connection. It does not survive a reboot or the process being killed; in that
case `status` says the run was interrupted and `start` begins a new one. A
second `start` while a run is active is refused. Logs, the status line and the
exit status are kept in `test-logs/`.

## What the mathematics says, in one paragraph

The grouping step rests on one theorem: shortening the text by its smallest
repeat distance p, and replacing every other repeat distance q by q − p, gives
a smaller puzzle of exactly the same kind with exactly the same solutions
(plus max(0, 2p − n) completely free characters). This is Euclid's algorithm
applied to a set of repeat distances, and it is a known result in the theory
of words with several periods (see `../research/01-…`). The research notes
ask how far it extends: it carries over to grids only under an extra
condition, which the notes state and prove, and from two dimensions on that
condition can fail. Nothing here has any bearing on NP-complete problems.

## Limitations

- **Only these three kinds of rule.** There is no "contains this word
  somewhere in the middle", no wildcards, and no "must not" rules.
- **Letters must be plain visible ASCII characters** (English letters,
  digits, common punctuation). No spaces, accented letters or emoji, and each
  allowed letter can be listed only once. Because rules are typed as
  `Pword`/`Sword`, letters that your shell treats specially (such as `*` or
  `?`) need quoting.
- **The length must be known exactly.** The program cannot search over
  different lengths.
- **Memory grows with the text length**, roughly 35 bytes per character
  while solving (more while explaining a contradiction), so 20 million
  characters need under 1 GB and hundreds of millions need many gigabytes. A
  length that is plainly too large is refused with a message; a length that
  is only slightly too large for your machine may still be stopped by the
  operating system.
- **Explanations are not always the shortest possible.** With very many
  different border rules the program may give a longer chain than necessary
  (see "How fast it is" above). It is still a valid, checkable argument.
- **Printing is the slow part for huge answers.** `--full` on a
  million-character text prints millions of entries.
- `sh prefsuf.rs` recompiles on every run and writes the compiled program to
  your temporary folder (`$TMPDIR`, or `/tmp`).
- Input mistakes (for example a word longer than the text, or a letter that
  is not in the allowed list) are reported as errors, with exit status 2.
  So is an answer that cannot be written (a full disk, or a pipe closed
  early, as in `sh prefsuf.rs ... | head`).
- If the report's own double-check fails (it never has in testing), the
  report is still printed, with "FAILED - please report this" (or
  `checked ... false` in `--compact`), followed by a diagnostic on stderr,
  and the exit status is 3. Treat such an answer as wrong.

## Changes from the first release (compatibility)

Answers, reports on standard output, certificates, and what `certcheck.rs`
accepts or rejects are unchanged. What changed, all on purpose:

| Situation | First release | Now |
| --- | --- | --- |
| `prefsuf.rs`: an argument that is not valid UTF-8 | crash, exit 101 | message, exit 2 |
| `prefsuf.rs`: the answer cannot be written (full disk, closed pipe) | crash, exit 101 | message, exit 2 |
| `prefsuf.rs`: the report's double-check fails | exit 0 | diagnostic on stderr, exit 3 |
| `certcheck.rs`: bad puzzle after the file name | `REJECTED`, exit 1 | message on stderr, exit 2 |
| `certcheck.rs`: an argument that is not valid UTF-8, or the verdict cannot be written | crash, exit 101 | exit 2 |
| `certcheck.rs`: it cannot be compiled | exit 1 | exit 2 |
| `certcheck.rs`: verdict for one group | `1 groups` | `1 group` |
| `certcheck.rs` without a puzzle after the file name | `VALID` only | `VALID` plus a reminder on stderr |
| `prefsuf.rs` messages for a too-large length, a non-ASCII letter, an unknown `--option` | generic | specific (exit status unchanged, 2) |

Measured with `tools/test_cli.sh` run against the first release: 15 of its
cases pass and 11 fail. The 11 are exactly the changes in this table (the
fault-injection block counts once there, because the first release has no
failure path to inject into); none is a regression. The 15 that pass are
the behaviour that did not change.
