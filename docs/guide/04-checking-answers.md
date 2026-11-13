# 4. Checking answers

[Guide index](README.md) · previous: [How it works](03-how-it-works.md) · next: [Words used](05-words-used.md)

A fast answer is only useful if it is right. This project is built so that
you do not have to take the solver's word for it.

## Built-in double-checks

Every report ends with a line such as:

```text
Double-check: the example was tested directly against every rule: confirmed.
```

For a contradiction, each step of the explanation is re-tested against the
rule it cites.

## Certificates and a separate checker

The solver can write a **certificate**: a plain text file containing the
puzzle, the claimed answer and the evidence for it. A second, separate
program reads the file and either accepts or rejects it.

```sh
sh prefsuf.rs --certificate 8 ab B5 Pab Sb > answer.txt
sh certcheck.rs answer.txt 8 ab B5 Pab Sb
```

```text
VALID: solvable; 3 groups of equal positions, 1 of them free; number of texts: 2
```

The checker does **not** solve the puzzle and shares no code with the
solver. It only tests the evidence:

- **For "there are solutions":** every listed tie really is a tie of some
  rule; the ties produce exactly the claimed groups; and the groups are not
  too coarse, because giving each group its own letter breaks no rule.
- **For "no solution":** the two clashing letters really are demanded by the
  cited rules, and each step of the chain really is a tie.

## What the checker does not tell you

- It does not check that a chain is the *shortest* one.
- It does not compare the certificate with the report printed on screen.
- The checker is itself a program. It is short and heavily tested (including
  with hundreds of thousands of deliberately damaged certificates), but it
  has not been verified by a proof assistant.

The exact file format is described in
[certificate-format.md](../certificate-format.md).
