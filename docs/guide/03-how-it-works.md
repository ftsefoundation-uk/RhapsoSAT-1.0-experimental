# 3. How it works

[Guide index](README.md) · previous: [Trying it](02-trying-it.md) · next: [Checking answers](04-checking-answers.md)

The solver works in three steps. Only the first needs a clever idea.

## Step 1: find which positions must be equal

A rule "first k = last k" ties each position to the one a fixed distance
later. Follow the ties and the positions fall into **groups**: everything in
a group must hold the same letter.

The obvious way is to look at every tie. That is slow: a long text with many
such rules can contain billions of ties.

The solver uses a shortcut instead.

1. Take the **longest** "first k = last k" rule.
2. The end of the text is then a copy of an earlier part. Note, for each
   position in that end part, which earlier position it copies.
3. **Forget the end part** and continue with the first k characters only.
   The other rules still apply to this shorter text, and one extra fact
   about it is remembered.
4. Repeat until no rule is left.

Each position is dealt with once, however many rules there are. This is the
same idea as Euclid's ancient method for finding the greatest common divisor
of two numbers: replace a problem by a smaller one of the same kind.

> **Optional detail.** If the longest rule has k characters on a text of
> length n, the distance between tied positions is p = n − k. The extra fact
> remembered in step 3 is that the first k characters also satisfy "first
> k − p = last k − p" (when k is larger than p). The method and its proof are
> in [research note 01](../research/01-existing-solver-and-theorem.md); it
> is a known result about words with several periods.

## Step 2: place the letters

The "starts with" and "ends with" rules give letters to particular
positions. Each letter is handed to that position's whole group.

If a group is given two different letters, the rules **contradict** each
other. The solver then builds the explanation: a chain of ties leading from
one of the two positions to the other.

## Step 3: read off the answer

- A group with a letter is filled in.
- A group without a letter is an **open choice**.
- The number of texts is (number of allowed letters) multiplied by itself
  once per open choice. With 2 letters and 3 open choices: 2 × 2 × 2 = 8.

## Why the explanation is kept short

For a contradiction, the solver first searches for the shortest chain. If
that search would take too long, it falls back to a faster method that is
guaranteed to finish quickly and then shortens the chain it found. The
result is always a valid chain; it is not always the shortest possible one.

> **Optional detail.** Only a handful of the "first k = last k" rules really
> matter: at most about 2 × log₂(n) of them force everything the others
> force. The fallback searches through those only. See
> [research note 06](../research/06-reduction-first-generators-and-certificates.md).
