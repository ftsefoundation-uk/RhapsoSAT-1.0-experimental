# Demo: how equal positions connect

`index.html` is a small interactive page for people with no mathematics or
programming background. It shows how "equality shifts" tie positions of a
short text (or a small grid) into groups, and compares two ways of finding
the groups:

- **Naive: check every link** — one operation per tie.
- **Reduction: process fewer relationships** — settle the far end from
  earlier positions, then repeat on the smaller piece.

## Opening it

It is one self-contained file: no build step, no network access, no
dependencies. Download `demo/index.html` (or clone the repository) and open
the file in any modern browser, on a computer or a phone.

## What you can do

- switch between the **Text** and **Grid** examples;
- change the text length (3–16) or grid size (2×2 to 6×6);
- add and remove equality shifts (up to 4 for text, 3 for a grid);
- step forwards and backwards through the ties, or jump to the result;
- switch between the two methods and compare **Positions**,
  **Links represented**, **Groups** and **Operations**.

## What it is and is not

- It is a teaching example. It is **not the solver** and shares no code
  with `prefsuf.rs`; its logic is about 90 lines of JavaScript written for
  this page.
- The numbers are **illustrative operation counts** for the small example on
  screen, not measured benchmark timings.
- It handles only small texts and grids with a few shifts. It does not
  handle arbitrary networks or other kinds of rule.
- In a grid the shortcut applies only while one shift "fits inside" all the
  others (note 05). When it does not, the page says so and checks the
  remaining ties one by one, so the two methods can need the same number of
  operations.

## How it was checked

No JavaScript runtime or browser is installed on the development server, so
the page itself was **not run there**. Its logic was checked through a
line-by-line Python port: on 790 text examples and 615 grid examples the
reduction gave exactly the same groups as checking every link, and never
used more operations. Please report anything that looks wrong in a real
browser.
