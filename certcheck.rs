#!/bin/sh
//bin/true 2>/dev/null; out="${TMPDIR:-/tmp}/prefsuf_certcheck"; rustc --edition 2021 -O "$0" -o "$out" || exit 2; exec "$out" "$@"
// certcheck.rs -- stand-alone checker for the certificates that
// `sh prefsuf.rs --certificate ...` prints. It does NOT solve the puzzle and
// shares no code with prefsuf.rs: it only reads the certificate and tests
// it against the rules of the puzzle. Format: docs/certificate-format.md.
//
//     sh certcheck.rs FILE                       check a certificate ("-" reads standard input)
//     sh certcheck.rs FILE LENGTH LETTERS RULE.. also require that it is about this puzzle
//
// Prints one line starting with VALID or REJECTED. Exit status: 0 valid,
// 1 rejected, 2 the file could not be read, the command line is wrong, the
// checker could not be compiled, or the verdict could not be written.
// Without LENGTH LETTERS RULE..., VALID is about the puzzle written in the
// certificate's own header, whatever puzzle you had in mind.
//
// What VALID means
//   claim solvable:  the groups of positions in the certificate are exactly
//     the positions that the "first k = last k" rules force to be equal; no
//     group is given two different letters; so the puzzle has solutions and
//     their number is (number of letters) ^ (number of free groups).
//       1. every listed link joins positions i and i + (length - k) for a
//          rule Bk of the puzzle, and both ends carry the same label;
//       2. the links join the positions into exactly as many groups as
//          there are labels (so equal labels ARE forced equal);
//       3. the sequence of labels, read as a text, has every border the
//          rules ask for (so different labels are NOT forced equal). This
//          uses the classical border table of the label sequence;
//       4. the words of the P and S rules give each label at most one
//          letter, and the number of labels without a letter is as claimed.
//   claim contradiction:  two positions are fixed to different letters by
//     the cited P/S rules and are joined by a chain in which every step is
//     a link of the cited B rule. So no text satisfies the rules.
//
// What is NOT checked: that the chain of a contradiction is the shortest
// one, that the report prefsuf.rs prints matches the certificate, and
// anything about how the certificate was produced.

use std::io::Read;

// ---------------------------------------------------------------- reading

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// The rest of the current line, without the line end.
    fn line(&mut self) -> Result<&'a [u8], String> {
        if self.at >= self.bytes.len() {
            return Err("the certificate ends too early".into());
        }
        let start = self.at;
        while self.at < self.bytes.len() && self.bytes[self.at] != b'\n' {
            self.at += 1;
        }
        let mut end = self.at;
        if self.at < self.bytes.len() {
            self.at += 1;
        }
        if end > start && self.bytes[end - 1] == b'\r' {
            end -= 1;
        }
        Ok(&self.bytes[start..end])
    }
    /// The next run of non-blank characters, across line ends.
    fn token(&mut self) -> Result<&'a [u8], String> {
        while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
        if self.at >= self.bytes.len() {
            return Err("the certificate ends too early".into());
        }
        let start = self.at;
        while self.at < self.bytes.len() && !self.bytes[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
        Ok(&self.bytes[start..self.at])
    }
    fn number(&mut self, what: &str) -> Result<usize, String> {
        let t = self.token()?;
        number(t).ok_or_else(|| format!("{what}: '{}' is not a whole number", show(t)))
    }
    fn word(&mut self, expected: &str) -> Result<(), String> {
        let t = self.token()?;
        if t == expected.as_bytes() {
            Ok(())
        } else {
            Err(format!("expected '{expected}', found '{}'", show(t)))
        }
    }
    fn only_blank_left(&self) -> bool {
        self.bytes[self.at..].iter().all(|b| b.is_ascii_whitespace())
    }
}

/// Digits only, at most 18 of them (so no overflow anywhere below).
fn number(t: &[u8]) -> Option<usize> {
    if t.is_empty() || t.len() > 18 || !t.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut v: u64 = 0;
    for b in t {
        v = v * 10 + (b - b'0') as u64;
    }
    usize::try_from(v).ok()
}

fn show(t: &[u8]) -> String {
    let s: String = t.iter().take(40).map(|b| if b.is_ascii_graphic() { *b as char } else { '?' }).collect();
    if t.len() > 40 { format!("{s}...") } else { s }
}

fn fields(line: &[u8]) -> Vec<&[u8]> {
    line.split(|b| *b == b' ' || *b == b'\t').filter(|f| !f.is_empty()).collect()
}

// ----------------------------------------------------------------- puzzle

enum Rule {
    Starts(Vec<u8>),
    Ends(Vec<u8>),
    Border(usize),
}

struct Puzzle {
    n: usize,
    letters: Vec<u8>,
    rules: Vec<Rule>,
}

fn one_rule(kind: &[u8], rest: Option<&[u8]>) -> Result<Rule, String> {
    match kind {
        b"P" => Ok(Rule::Starts(rest.unwrap_or(b"").to_vec())),
        b"S" => Ok(Rule::Ends(rest.unwrap_or(b"").to_vec())),
        b"B" => rest.and_then(number).map(Rule::Border).ok_or_else(|| "a B rule needs a whole number".to_string()),
        _ => Err(format!("'{}' is not a rule kind (P, S or B)", show(kind))),
    }
}

/// The puzzle must itself be well formed, by the rules of the puzzle.
fn validate(p: &Puzzle) -> Result<(), String> {
    if p.letters.is_empty() {
        return Err("the list of letters is empty".into());
    }
    let mut allowed = [false; 256];
    for a in &p.letters {
        if !a.is_ascii_graphic() {
            return Err("letters must be visible ASCII characters".into());
        }
        if allowed[*a as usize] {
            return Err(format!("the letter '{}' is listed twice", *a as char));
        }
        allowed[*a as usize] = true;
    }
    for (i, r) in p.rules.iter().enumerate() {
        match r {
            Rule::Starts(w) | Rule::Ends(w) => {
                if w.len() > p.n {
                    return Err(format!("rule {}: the word is longer than the text", i + 1));
                }
                if w.iter().any(|c| !allowed[*c as usize]) {
                    return Err(format!("rule {}: the word uses a letter that is not allowed", i + 1));
                }
            }
            Rule::Border(k) => {
                if *k > p.n {
                    return Err(format!("rule {}: B{k} is longer than the text", i + 1));
                }
            }
        }
    }
    Ok(())
}

/// The puzzle as given on the command line: LENGTH LETTERS RULE...
fn puzzle_from_args(args: &[String]) -> Result<Puzzle, String> {
    if args.len() < 2 {
        return Err("give LENGTH and LETTERS (and the rules) after the file name, or nothing".into());
    }
    let n = number(args[0].as_bytes()).ok_or("LENGTH must be a whole number")?;
    let mut rules = Vec::new();
    for a in &args[2..] {
        let b = a.as_bytes();
        if b.is_empty() {
            return Err("an empty rule".into());
        }
        rules.push(one_rule(&b[..1], Some(&b[1..]))?);
    }
    Ok(Puzzle { n, letters: args[1].as_bytes().to_vec(), rules })
}

fn same_puzzle(a: &Puzzle, b: &Puzzle) -> bool {
    a.n == b.n
        && a.letters == b.letters
        && a.rules.len() == b.rules.len()
        && a.rules.iter().zip(&b.rules).all(|(x, y)| match (x, y) {
            (Rule::Starts(v), Rule::Starts(w)) | (Rule::Ends(v), Rule::Ends(w)) => v == w,
            (Rule::Border(j), Rule::Border(k)) => j == k,
            _ => false,
        })
}

/// The letter that a P or S rule fixes at a position, if it fixes one.
fn fixed_letter(p: &Puzzle, rule: &Rule, pos: usize) -> Option<u8> {
    match rule {
        Rule::Starts(w) => w.get(pos).copied(),
        Rule::Ends(w) => pos.checked_sub(p.n - w.len()).and_then(|i| w.get(i).copied()),
        Rule::Border(_) => None,
    }
}

/// The shift of rule number `r` (counted from 1) if it is a B rule that
/// links anything: positions i and i + shift for 0 <= i < k.
fn shift_of(p: &Puzzle, r: usize) -> Result<usize, String> {
    match r.checked_sub(1).and_then(|i| p.rules.get(i)) {
        Some(Rule::Border(k)) if *k > 0 && *k < p.n => Ok(p.n - k),
        Some(_) => Err(format!("rule {r} links no positions")),
        None => Err(format!("there is no rule {r}")),
    }
}

// ---------------------------------------------------------------- checker

/// Check a certificate. `expected` is the puzzle it must be about, if the
/// caller wants that tested. Ok(text) describes what was verified.
pub fn check_text(bytes: &[u8], expected: Option<&[String]>) -> Result<String, String> {
    let mut r = Reader { bytes, at: 0 };
    if fields(r.line()?) != [b"prefsuf-certificate".as_slice(), b"1".as_slice()] {
        return Err("the first line is not 'prefsuf-certificate 1'".into());
    }
    let head = |r: &mut Reader, name: &str| -> Result<Vec<u8>, String> {
        let f = fields(r.line()?);
        if f.len() == 2 && f[0] == name.as_bytes() {
            Ok(f[1].to_vec())
        } else {
            Err(format!("expected a line '{name} ...'"))
        }
    };
    let n = number(&head(&mut r, "length")?).ok_or("the length is not a whole number")?;
    let letters = head(&mut r, "letters")?;
    let m = number(&head(&mut r, "rules")?).ok_or("the number of rules is not a whole number")?;
    if m > bytes.len() {
        return Err("more rules announced than the certificate has lines".into());
    }
    let mut rules = Vec::new();
    for i in 0..m {
        let f = fields(r.line()?);
        if f.is_empty() || f.len() > 2 {
            return Err(format!("rule {}: expected 'P word', 'S word' or 'B number'", i + 1));
        }
        rules.push(one_rule(f[0], f.get(1).copied()).map_err(|e| format!("rule {}: {e}", i + 1))?);
    }
    let puzzle = Puzzle { n, letters, rules };
    validate(&puzzle)?;
    if let Some(args) = expected {
        let wanted = puzzle_from_args(args)?;
        if !same_puzzle(&puzzle, &wanted) {
            return Err("the certificate is about a different puzzle than the one given".into());
        }
    }
    // Numbers below are positions (< n) or counts (<= n); n positions need
    // n tokens, so a length beyond the size of the file cannot be honest.
    r.word("claim")?;
    let verdict = match r.token()? {
        b"solvable" => {
            if n > bytes.len() {
                return Err("the certificate is shorter than the text it describes".into());
            }
            check_solvable(&puzzle, &mut r)?
        }
        b"contradiction" => check_contradiction(&puzzle, &mut r)?,
        other => return Err(format!("unknown claim '{}'", show(other))),
    };
    r.word("end")?;
    if !r.only_blank_left() {
        return Err("there is text after 'end'".into());
    }
    Ok(verdict)
}

fn root(group: &mut [u32], mut x: u32) -> u32 {
    while group[x as usize] != x {
        group[x as usize] = group[group[x as usize] as usize];
        x = group[x as usize];
    }
    x
}

fn check_solvable(p: &Puzzle, r: &mut Reader) -> Result<String, String> {
    let n = p.n;
    if n >= u32::MAX as usize {
        return Err("the text is too long for this checker".into());
    }
    r.word("groups")?;
    let groups = r.number("groups")?;
    r.word("free")?;
    let free = r.number("free")?;
    if groups > n {
        return Err("more groups than positions".into());
    }
    r.word("labels")?;
    let mut label: Vec<u32> = Vec::with_capacity(n);
    let mut used = vec![false; groups];
    for i in 0..n {
        let v = r.number("label")?;
        if v >= groups {
            return Err(format!("position {i} has label {v}, but there are only {groups} groups"));
        }
        used[v] = true;
        label.push(v as u32);
    }
    if used.iter().any(|u| !u) {
        return Err("some group number is not used by any position".into());
    }

    // 1 and 2: the links are links of the rules, stay inside a label, and
    // join the positions into exactly `groups` groups.
    r.word("forest")?;
    let total = r.number("forest")?;
    if total > n {
        return Err("more links than positions".into());
    }
    let mut group: Vec<u32> = (0..n as u32).collect();
    let (mut seen, mut joined) = (0usize, 0usize);
    while seen < total {
        r.word("links")?;
        let rule = r.number("rule number")?;
        let count = r.number("number of links")?;
        let shift = shift_of(p, rule)?;
        if count == 0 || count > total - seen {
            return Err("the numbers of links do not add up".into());
        }
        for _ in 0..count {
            let u = r.number("link")?;
            if u >= n - shift {
                return Err(format!("rule {rule} has no link at position {u}"));
            }
            let v = u + shift;
            if label[u] != label[v] {
                return Err(format!("a link of rule {rule} joins positions {u} and {v}, which carry different labels"));
            }
            let (a, b) = (root(&mut group, u as u32), root(&mut group, v as u32));
            if a != b {
                group[a as usize] = b;
                joined += 1;
            }
        }
        seen += count;
    }
    if n - joined != groups {
        return Err(format!("the links leave {} groups, the labels say {groups}: positions with equal labels are not shown to be forced equal", n - joined));
    }

    // 3: the label sequence has every border the rules ask for. fail[i] is
    // the length of the longest proper border of the first i labels.
    let mut fail = vec![0u32; n + 1];
    let mut k = 0usize;
    for i in 1..n {
        while k > 0 && label[i] != label[k] {
            k = fail[k] as usize;
        }
        if label[i] == label[k] {
            k += 1;
        }
        fail[i + 1] = k as u32;
    }
    let mut is_border = vec![false; n + 1];
    let mut b = if n > 0 { fail[n] as usize } else { 0 };
    while b > 0 {
        is_border[b] = true;
        b = fail[b] as usize;
    }
    for (i, rule) in p.rules.iter().enumerate() {
        if let Rule::Border(k) = rule {
            if *k > 0 && *k < n && !is_border[*k] {
                return Err(format!("rule {} (B{k}) does not hold for the labels: two positions it forces equal carry different labels", i + 1));
            }
        }
    }

    // 4: letters.
    let mut letter = vec![0u8; groups];
    for (i, rule) in p.rules.iter().enumerate() {
        let (word, offset) = match rule {
            Rule::Starts(w) => (w, 0),
            Rule::Ends(w) => (w, n - w.len()),
            Rule::Border(_) => continue,
        };
        for (j, ch) in word.iter().enumerate() {
            let g = label[offset + j] as usize;
            if letter[g] == 0 {
                letter[g] = *ch;
            } else if letter[g] != *ch {
                return Err(format!("rule {} needs '{}' at position {}, but its group already has '{}': the puzzle has no solution", i + 1, *ch as char, offset + j, letter[g] as char));
            }
        }
    }
    let really_free = letter.iter().filter(|l| **l == 0).count();
    if really_free != free {
        return Err(format!("{really_free} groups have no letter, the certificate says {free}"));
    }
    let count = match (p.letters.len() as u128).checked_pow(free as u32) {
        _ if p.letters.len() == 1 => "1".to_string(),
        Some(c) if free <= u32::MAX as usize => c.to_string(),
        _ => format!("{}^{free}", p.letters.len()),
    };
    Ok(format!("solvable; {groups} groups of equal positions, {free} of them free; number of texts: {count}"))
}

fn check_contradiction(p: &Puzzle, r: &mut Reader) -> Result<String, String> {
    let n = p.n;
    let mut ends = Vec::new();
    for name in ["first", "second"] {
        r.word(name)?;
        let pos = r.number("position")?;
        let letter = r.token()?;
        let rule = r.number("rule number")?;
        if pos >= n || letter.len() != 1 {
            return Err(format!("'{name}' does not name a position and a letter"));
        }
        let fixed = rule.checked_sub(1).and_then(|i| p.rules.get(i)).and_then(|x| fixed_letter(p, x, pos));
        if fixed != Some(letter[0]) {
            return Err(format!("rule {rule} does not put '{}' at position {pos}", show(letter)));
        }
        ends.push((pos, letter[0]));
    }
    if ends[0].1 == ends[1].1 {
        return Err("the two letters are the same: no contradiction".into());
    }
    r.word("chain")?;
    let len = r.number("chain length")?;
    if len == 0 || len > r.bytes.len() {
        return Err("the chain is empty or longer than the certificate".into());
    }
    let mut chain = Vec::with_capacity(len);
    for _ in 0..len {
        let pos = r.number("chain position")?;
        if pos >= n {
            return Err(format!("the chain leaves the text at position {pos}"));
        }
        chain.push(pos);
    }
    if chain[0] != ends[0].0 || chain[len - 1] != ends[1].0 {
        return Err("the chain does not run from the first position to the second".into());
    }
    r.word("via")?;
    for i in 0..len - 1 {
        let rule = r.number("rule number")?;
        let shift = shift_of(p, rule)?;
        if chain[i].abs_diff(chain[i + 1]) != shift {
            return Err(format!("positions {} and {} are not linked by rule {rule}", chain[i], chain[i + 1]));
        }
    }
    Ok(format!("contradiction; position {} must be '{}' and position {} must be '{}', and a chain of {} links forces them equal", ends[0].0, ends[0].1 as char, ends[1].0, ends[1].1 as char, len - 1))
}

/// Print the one-line verdict. A verdict that cannot be written (closed
/// pipe, full disk) must not look like a successful check: exit 2.
fn say(line: &str) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{line}").and_then(|_| out.flush()).is_err() {
        std::process::exit(2);
    }
}

fn main() {
    const USAGE: &str = "usage: sh certcheck.rs FILE [LENGTH LETTERS RULE...]     (FILE may be - for standard input)";
    // Arguments that are not valid UTF-8 cannot name a puzzle: a usage error.
    let mut args: Vec<String> = Vec::new();
    for a in std::env::args_os().skip(1) {
        match a.into_string() {
            Ok(s) => args.push(s),
            Err(bad) => {
                eprintln!("argument {bad:?} is not valid text\n{USAGE}");
                std::process::exit(2);
            }
        }
    }
    if args.is_empty() || args[0] == "--help" {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    // A puzzle given on the command line is read before the certificate, so
    // that a mistake in it is reported as a bad command line (exit 2), not
    // as a rejected certificate (exit 1).
    if args.len() > 1 {
        if let Err(e) = puzzle_from_args(&args[1..]) {
            eprintln!("bad puzzle on the command line: {e}\n{USAGE}");
            std::process::exit(2);
        }
    }
    let mut bytes = Vec::new();
    let read = if args[0] == "-" {
        std::io::stdin().read_to_end(&mut bytes).map(|_| ())
    } else {
        std::fs::File::open(&args[0]).and_then(|mut f| f.read_to_end(&mut bytes)).map(|_| ())
    };
    if let Err(e) = read {
        eprintln!("cannot read {}: {e}", args[0]);
        std::process::exit(2);
    }
    let expected = if args.len() > 1 { Some(&args[1..]) } else { None };
    match check_text(&bytes, expected) {
        Ok(what) => {
            say(&format!("VALID: {what}"));
            if expected.is_none() {
                eprintln!("note: checked against the puzzle written in the certificate itself; add LENGTH LETTERS RULE... to require that it is your puzzle");
            }
        }
        Err(why) => {
            say(&format!("REJECTED: {why}"));
            std::process::exit(1);
        }
    }
}
