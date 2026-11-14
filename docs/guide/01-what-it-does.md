# 1. What it does

[Guide index](README.md) · next: [Trying it](02-trying-it.md)

## The puzzle

Someone is thinking of a piece of text. You are told:

- **how long** it is, for example 8 characters;
- **which letters** it may use, for example `a` and `b`;
- some **rules** about it.

There are three kinds of rule.

| Rule | Written as | Meaning |
|---|---|---|
| Starts with | `Pab` | The text begins with "ab". |
| Ends with | `Sb` | The text ends with "b". |
| First k = last k | `B5` | The first 5 characters, read as a word, are the same as the last 5. |

The third kind is the interesting one. It quietly ties characters together:
in an 8-character text, "first 5 = last 5" means character 1 equals
character 4, character 2 equals character 5, and so on.

## The three possible answers

**Exactly one text fits.** The solver prints it.

**Several texts fit.** The solver prints a pattern and the exact number:

```text
Result: SEVERAL SOLUTIONS - 2 different texts fit all the rules.

  Pattern  : a b ?1 a b ?1 a b
  Example  : abaabaab
```

A plain letter is forced. `?1` is an open choice: any allowed letter, but
the same one wherever `?1` appears.

**No text fits.** The solver shows why, as a short chain of steps that each
follow from one rule (see the example in the main [README](../../README.md)).

## What it is for

It is an illustrative problem: small enough to explain completely, yet large
inputs (millions of characters; millions of rules in the test programs;
on a command line the system's limit on argument length allows about
100,000) are still answered
quickly and with evidence. It is a place to study how such answers can be
found fast and checked independently.

## What it is not

- Not a general puzzle or logic solver. It handles these three rules only.
- Not a SAT solver in the usual sense, despite the project name.
- Not a result about hard problems: this puzzle is easy for computers, and
  nothing here says anything about NP-complete problems.
