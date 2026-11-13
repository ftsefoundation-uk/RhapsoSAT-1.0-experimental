# 2. Trying it

[Guide index](README.md) · previous: [What it does](01-what-it-does.md) · next: [How it works](03-how-it-works.md)

## The demo page (nothing to install)

Open <https://ftsefoundation-uk.github.io/RhapsoSAT-1.0-experimental/>.

- Choose **Text** or **Grid**.
- Add or remove **equality shifts** (a shift of 3 ties each position to the
  one 3 places later).
- Press **Next step** to watch positions being tied into groups.
- Switch between **Naive** (look at every tie) and **Reduction** (the
  shortcut the solver is built on) and compare the number of operations.

The page is a teaching example. It is not the solver, shares no code with
it, and its numbers are counts for the small example on screen, not speed
measurements.

## The solver (needs the Rust compiler)

On Linux or macOS with `rustc` installed, download this repository and run:

```sh
sh prefsuf.rs 8 ab B5 Pab Sb
```

That asks: *which 8-character texts over the letters a and b start with
"ab", end with "b", and have their first 5 characters equal to their last
5?*

The first run takes a few seconds, because the file compiles itself.

| Command | What it does |
|---|---|
| `sh prefsuf.rs LENGTH LETTERS RULE…` | Solve one puzzle |
| `sh prefsuf.rs --help` | Show all options |
| `sh prefsuf.rs` | Run the self-tests and two examples |
| `sh prefsuf.rs --full …` | Do not shorten long answers |
| `sh prefsuf.rs --certificate …` | Print a certificate instead of a report (see [Checking answers](04-checking-answers.md)) |

More detail: the [user manual](../reference/user-manual.md).
