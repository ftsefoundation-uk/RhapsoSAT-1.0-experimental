# Certificate format (version 1)

`sh prefsuf.rs --certificate LENGTH LETTERS [RULE]...` prints a certificate
of the answer instead of the report. `certcheck.rs` reads it and confirms or
rejects it **without solving the puzzle**; it shares no code with
`prefsuf.rs`.

```sh
sh prefsuf.rs --certificate 8 ab B5 Pab Sb > c.txt
sh certcheck.rs c.txt                     # is the certificate valid?
sh certcheck.rs c.txt 8 ab B5 Pab Sb      # ... and is it about this puzzle?
sh prefsuf.rs --certificate 7 ab B4 Pa Sb | sh certcheck.rs -
```

The checker prints one line starting with `VALID` or `REJECTED` and exits
with status 0 (valid), 1 (rejected) or 2 (file unreadable, bad command
line).

## Layout

Plain ASCII text. Positions are counted from 0, rules from 1.

```
prefsuf-certificate 1
length N
letters LETTERS
rules M
<M lines, one rule each:  P word | S word | B k   (an empty word: just P or S)>
claim solvable | claim contradiction
<body, see below>
end
```

The header lines must appear exactly in this order, one item per line.
After `claim`, items are separated by any blanks or line ends.

### Body of `claim solvable`

```
groups G
free F
labels
<N numbers: the group of each position, each below G, every group used>
forest T
links R C          (repeated; the C values add up to T)
<C numbers u: rule R links position u to position u + (N - k), R being "B k">
```

### Body of `claim contradiction`

```
first  POSITION LETTER RULE     (rule RULE, a P or S rule, puts LETTER at POSITION)
second POSITION LETTER RULE     (a different letter)
chain K
<K positions, from the first position to the second>
via
<K - 1 rule numbers: rule i links chain position i to chain position i + 1>
```

## What the checker verifies

For every certificate: the puzzle in the header is well formed (letters
distinct visible ASCII, words no longer than the text and made of allowed
letters, borders no longer than the text), the layout is exact, and nothing
follows `end`. If a puzzle is given on the command line, the header must
describe exactly that puzzle.

**Solvable.**

1. Every link joins positions u and u + (N − k) with u < k for a rule
   `B k` with 0 < k < N, and both ends carry the same label.
2. Joining the positions along the links leaves exactly G groups. With 1,
   this shows positions with equal labels are forced equal.
3. The label sequence, read as a text, has every border the rules ask for
   (decided with the border table of the sequence). So giving different
   letters to different labels breaks no B rule: positions with different
   labels are not forced equal.
4. The words of the P and S rules give each group at most one letter, and
   exactly F groups get none.

Then the texts satisfying the rules are exactly the ways of choosing a
letter for each free group: (number of letters)^F of them.

**Contradiction.** The two cited rules fix the two positions to the two
letters, the letters differ, the chain starts and ends at those positions,
and every step is a link of the cited B rule. Positions joined by links
must hold the same letter, so no text satisfies the rules.

## What the checker does not verify

- That a contradiction's chain is the shortest possible.
- That the report `prefsuf.rs` prints agrees with the certificate (the
  certificate is produced by a separate call).
- Anything about how the certificate was made. A certificate with a
  different but still correct forest, or with groups numbered differently,
  is accepted.

The checker's own correctness rests on about 300 lines of code and the
argument above; it is tested (`sh cert_lab.rs`) but not machine-verified.

## Size and cost

The certificate contains the puzzle, so it is at least as long as the list
of rules. A solvable certificate adds N labels and N − G links. Checking
takes time proportional to the size of the certificate (times the
inverse-Ackermann factor of union-find). Producing it looks at no more than
N × (number of generator borders) ≤ N × (2 log₂ N + 2) links
(`docs/research/06`).

Measured at N = 5,000,000 (`sh cert_lab.rs big`; note 06 has the table):
37–93 MB, 0.4–1.3 s to produce, 0.15–1.2 s to check.

Limits: texts of 2³² − 1 characters or more are refused by both programs.
Numbers in a certificate may have at most 18 digits.
