#!/bin/sh
//bin/true 2>/dev/null; d="$(dirname "$0")"; t="${TMPDIR:-/tmp}/prefsuf_cert"; for f in prefsuf.rs certcheck.rs; do [ -f "$d/$f" ] || { echo "put $f in the same folder as $0"; exit 1; }; done; mkdir -p "$t" && tail -n +3 "$d/prefsuf.rs" | sed 's/^fn main() {/fn orig_main() {/' > "$t/core.rs" && tail -n +3 "$d/certcheck.rs" | sed 's/^fn main() {/pub fn checker_main() {/' > "$t/check.rs" && tail -n +3 "$0" > "$t/main.rs" && rustc --edition 2021 -O "$t/main.rs" -o "$t/run" && exec "$t/run" "$@"; exit $?
// Certificate laboratory: tests of `prefsuf.rs --certificate` together with
// the stand-alone checker certcheck.rs. Keep both files in the same folder.
// The checker's code is compiled into its own module; it sees nothing of
// the solver.
//
//     sh cert_lab.rs [check [SEED]]   every test below (about 30 s)
//     sh cert_lab.rs big [N]          certificates for long texts (default N = 5,000,000)
//
// The property tested throughout: the checker never says VALID to a false
// statement. "True" is decided by the solver and, for short texts, by
// listing every text.
#![allow(dead_code)]
include!("core.rs");
mod checker {
    include!("check.rs");
}
use std::time::Instant;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, m: usize) -> usize {
        (self.next() % m as u64) as usize
    }
}

struct Report {
    fails: usize,
}
impl Report {
    fn must(&mut self, ok: bool, what: &str) {
        if ok {
            println!("  ok    {what}");
        } else {
            self.fails += 1;
            println!("  FAIL  {what}");
        }
    }
}

/// The puzzle as command-line words, the way the checker is told about it.
fn words(n: usize, alphabet: &[u8], cons: &[Con]) -> Vec<String> {
    let mut out = vec![n.to_string(), text(alphabet)];
    for c in cons {
        out.push(match c {
            Con::Prefix(w) => format!("P{}", text(w)),
            Con::Suffix(w) => format!("S{}", text(w)),
            Con::Border(k) => format!("B{k}"),
        });
    }
    out
}

/// What is true about a puzzle: None = no solution, Some(free groups).
fn truth(n: usize, alphabet: &[u8], cons: &[Con]) -> Option<usize> {
    match solve(n, alphabet, cons).unwrap() {
        Outcome::Unsat(_) => None,
        Outcome::Sat { free, .. } => Some(free),
    }
}

/// Does a VALID verdict state the truth?
fn verdict_true(verdict: &str, truth: Option<usize>, groups: usize) -> bool {
    match truth {
        None => verdict.starts_with("contradiction;"),
        Some(free) => {
            let noun = if groups == 1 { "group" } else { "groups" };
            verdict.starts_with(&format!("solvable; {groups} {noun} of equal positions, {free} of them free;"))
        }
    }
}

fn group_count(n: usize, cons: &[Con]) -> usize {
    let mut r = equal_positions(n, cons).unwrap();
    r.sort_unstable();
    r.dedup();
    r.len()
}

fn random_puzzle(rng: &mut Lcg, round: usize) -> (usize, Vec<u8>, Vec<Con>) {
    let n = rng.below(match round % 12 {
        0 => 400,
        1 | 2 => 60,
        _ => 14,
    });
    let letters: &[u8] = [b"ab".as_slice(), b"abc", b"a", b"01#"][rng.below(4)];
    let mut cons = Vec::new();
    for _ in 0..rng.below(6) {
        let word = |rng: &mut Lcg| -> Vec<u8> { (0..rng.below(n.min(5) + 1)).map(|_| letters[rng.below(letters.len())]).collect() };
        cons.push(match rng.below(4) {
            0 => Con::Prefix(word(rng)),
            1 => Con::Suffix(word(rng)),
            _ => Con::Border(rng.below(n + 1)),
        });
    }
    (n, letters.to_vec(), cons)
}

/// One random change to the text of a certificate.
fn mutate(rng: &mut Lcg, cert: &str) -> String {
    let mut lines: Vec<String> = cert.lines().map(|l| l.to_string()).collect();
    if lines.is_empty() {
        return "end\n".to_string();
    }
    let pick = |rng: &mut Lcg, len: usize| rng.below(len.max(1));
    match rng.below(9) {
        // change one number
        0 | 1 | 2 => {
            let i = pick(rng, lines.len());
            let mut toks: Vec<String> = lines[i].split(' ').map(|t| t.to_string()).collect();
            let j = pick(rng, toks.len());
            if let Ok(v) = toks[j].parse::<u64>() {
                toks[j] = match rng.below(4) {
                    0 => (v + 1).to_string(),
                    1 => v.saturating_sub(1).to_string(),
                    2 => rng.below(20).to_string(),
                    _ => "99999999999999999999999".to_string(),
                };
            } else {
                toks[j] = ["solvable", "contradiction", "-1", "x", "", "B", "labels"][rng.below(7)].to_string();
            }
            lines[i] = toks.join(" ");
        }
        3 => {
            let i = pick(rng, lines.len());
            lines.remove(i);
        }
        4 => {
            let i = pick(rng, lines.len());
            let l = lines[i].clone();
            lines.insert(i, l);
        }
        5 => {
            let (i, j) = (pick(rng, lines.len()), pick(rng, lines.len()));
            lines.swap(i, j);
        }
        6 => {
            let cut = rng.below(cert.len());
            return cert[..cut].to_string();
        }
        7 => lines.push(["end", "0", "garbage", "links 1 1"][rng.below(4)].to_string()),
        _ => {
            let mut bytes = cert.as_bytes().to_vec();
            let i = rng.below(bytes.len());
            bytes[i] = b"0123456789 \nabPSB-"[rng.below(18)];
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }
    lines.join("\n") + "\n"
}

/// A certificate text written by this file from given parts.
fn write_solvable(n: usize, alphabet: &[u8], cons: &[Con], labels: &[usize], groups: usize, free: usize, links: &[(usize, Vec<usize>)]) -> String {
    let mut out = format!("prefsuf-certificate 1\nlength {n}\nletters {}\nrules {}\n", text(alphabet), cons.len());
    for w in &words(n, alphabet, cons)[2..] {
        out += &format!("{} {}\n", &w[..1], &w[1..]);
    }
    out += &format!("claim solvable\ngroups {groups}\nfree {free}\nlabels\n");
    out += &labels.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(" ");
    out += &format!("\nforest {}\n", links.iter().map(|x| x.1.len()).sum::<usize>());
    for (rule, us) in links {
        out += &format!("links {rule} {}\n{}\n", us.len(), us.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(" "));
    }
    out + "end\n"
}

/// The true labels and a spanning forest, computed here by plain
/// union-find over every link (not by the solver's method).
fn true_parts(n: usize, cons: &[Con]) -> (Vec<usize>, usize, Vec<(usize, Vec<usize>)>) {
    let mut group: Vec<usize> = (0..n).collect();
    fn top(g: &mut [usize], mut x: usize) -> usize {
        while g[x] != x {
            g[x] = g[g[x]];
            x = g[x];
        }
        x
    }
    let mut links = Vec::new();
    for (idx, c) in cons.iter().enumerate() {
        if let Con::Border(k) = c {
            if *k > 0 && *k < n {
                let mut kept = Vec::new();
                for i in 0..*k {
                    let (a, b) = (top(&mut group, i), top(&mut group, i + n - k));
                    if a != b {
                        group[a.max(b)] = a.min(b);
                        kept.push(i);
                    }
                }
                if !kept.is_empty() {
                    links.push((idx + 1, kept));
                }
            }
        }
    }
    let mut id = vec![usize::MAX; n];
    let mut groups = 0;
    let labels: Vec<usize> = (0..n)
        .map(|i| {
            let r = top(&mut group, i);
            if id[r] == usize::MAX {
                id[r] = groups;
                groups += 1;
            }
            id[r]
        })
        .collect();
    (labels, groups, links)
}

fn check(seed: u64) -> usize {
    let mut r = Report { fails: 0 };
    let mut rng = Lcg(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);

    println!("C1  certificates of the solver are accepted and say the truth (seed {seed})");
    let (mut puzzles, mut solvable, mut rejected, mut untrue, mut brute_bad, mut wrong_puzzle_ok) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut kept: Vec<(usize, Vec<u8>, Vec<Con>, String)> = Vec::new();
    for round in 0..20_000 {
        let (n, alphabet, cons) = random_puzzle(&mut rng, round);
        puzzles += 1;
        let cert = certificate(n, &alphabet, &cons).unwrap();
        let args = words(n, &alphabet, &cons);
        let t = truth(n, &alphabet, &cons);
        solvable += t.is_some() as u64;
        match checker::check_text(cert.as_bytes(), Some(&args)) {
            Ok(v) => untrue += !verdict_true(&v, t, group_count(n, &cons)) as u64,
            Err(_) => rejected += 1,
        }
        // short texts: the truth by listing every text
        if n <= 9 && alphabet.len().pow(n as u32) <= 20_000 {
            let all = brute(n, &alphabet, &cons).len();
            let said = t.map_or(0, |free| alphabet.len().pow(free as u32));
            brute_bad += (all != said) as u64;
        }
        // the same certificate offered for a different puzzle
        let mut other = args.clone();
        other.push("B1".into());
        if n > 1 && checker::check_text(cert.as_bytes(), Some(&other)).is_ok() {
            wrong_puzzle_ok += 1;
        }
        if round % 4 == 0 {
            kept.push((n, alphabet, cons, cert));
        }
    }
    r.must(rejected == 0, &format!("{puzzles} random puzzles ({solvable} solvable, {} contradictory): every certificate accepted", puzzles - solvable));
    r.must(untrue == 0, "every verdict states the true result, group count and number of free groups");
    r.must(brute_bad == 0, "for short texts the stated number of texts equals the number found by listing every text");
    r.must(wrong_puzzle_ok == 0, "a certificate offered for a different puzzle is rejected");

    println!("C2  random damage to the certificate text");
    let (mut tried, mut accepted, mut accepted_false, mut panicked) = (0u64, 0u64, 0u64, 0u64);
    for (n, alphabet, cons, cert) in &kept {
        let args = words(*n, alphabet, cons);
        let t = truth(*n, alphabet, cons);
        let groups = group_count(*n, cons);
        for _ in 0..40 {
            let mut bad = mutate(&mut rng, cert);
            if rng.below(4) == 0 {
                bad = mutate(&mut rng, &bad);
            }
            if bad == *cert {
                continue;
            }
            tried += 1;
            match std::panic::catch_unwind(|| checker::check_text(bad.as_bytes(), Some(&args))) {
                Ok(Ok(v)) => {
                    accepted += 1;
                    accepted_false += !verdict_true(&v, t, groups) as u64;
                }
                Ok(Err(_)) => {}
                Err(_) => panicked += 1,
            }
        }
    }
    r.must(accepted_false == 0, &format!("{tried} damaged certificates: none accepted with a false statement ({accepted} harmless changes accepted, {} rejected)", tried - accepted));
    r.must(panicked == 0, "the checker never crashes on damaged input");

    println!("C3  deliberate forgeries");
    let (mut forged, mut slipped, mut honest_bad) = (0u64, 0u64, 0u64);
    for round in 0..6000 {
        let (n, alphabet, cons) = random_puzzle(&mut rng, round);
        if n < 2 {
            continue;
        }
        let args = words(n, &alphabet, &cons);
        let t = truth(n, &alphabet, &cons);
        let (labels, groups, links) = true_parts(n, &cons);
        let free_of = |labels: &[usize], groups: usize| {
            let mut has = vec![false; groups];
            for c in &cons {
                if let Some((w, off)) = word_of(n, c) {
                    for j in 0..w.len() {
                        has[labels[off + j]] = true;
                    }
                }
            }
            has.iter().filter(|h| !**h).count()
        };
        let try_one = |text: String, must_reject: bool, forged: &mut u64, slipped: &mut u64| {
            *forged += 1;
            if let Ok(v) = checker::check_text(text.as_bytes(), Some(&args)) {
                if must_reject || !verdict_true(&v, t, groups) {
                    *slipped += 1;
                }
            }
        };
        // an honest certificate built here (not by the solver) is accepted when the puzzle is solvable
        let honest = write_solvable(n, &alphabet, &cons, &labels, groups, free_of(&labels, groups), &links);
        match (t, checker::check_text(honest.as_bytes(), Some(&args))) {
            (Some(_), Ok(v)) => honest_bad += !verdict_true(&v, t, groups) as u64,
            (Some(_), Err(_)) => honest_bad += 1,
            (None, Ok(_)) => honest_bad += 1, // "solvable" for a contradictory puzzle
            (None, Err(_)) => {}
        }
        // two groups merged
        if groups >= 2 {
            let (a, b) = (rng.below(groups), rng.below(groups));
            if a != b {
                let (lo, hi) = (a.min(b), a.max(b));
                let merged: Vec<usize> = labels.iter().map(|l| if *l == hi { lo } else if *l > hi { l - 1 } else { *l }).collect();
                try_one(write_solvable(n, &alphabet, &cons, &merged, groups - 1, free_of(&merged, groups - 1), &links), true, &mut forged, &mut slipped);
                // ... and with a forged link added to make the count fit
                let (pa, pb) = (labels.iter().position(|l| *l == lo).unwrap(), labels.iter().position(|l| *l == hi).unwrap());
                for (idx, c) in cons.iter().enumerate() {
                    if matches!(c, Con::Border(_)) {
                        let mut more = links.clone();
                        more.push((idx + 1, vec![pa.min(pb)]));
                        try_one(write_solvable(n, &alphabet, &cons, &merged, groups - 1, free_of(&merged, groups - 1), &more), true, &mut forged, &mut slipped);
                        break;
                    }
                }
            }
        }
        // one group split in two
        let victim = rng.below(n);
        if labels.iter().filter(|l| **l == labels[victim]).count() >= 2 {
            let mut split = labels.clone();
            split[victim] = groups;
            try_one(write_solvable(n, &alphabet, &cons, &split, groups + 1, free_of(&split, groups + 1), &links), true, &mut forged, &mut slipped);
        }
        // one link missing
        if let Some(i) = (0..links.len()).find(|i| !links[*i].1.is_empty()) {
            let mut fewer = links.clone();
            fewer[i].1.pop();
            fewer.retain(|x| !x.1.is_empty());
            try_one(write_solvable(n, &alphabet, &cons, &labels, groups, free_of(&labels, groups), &fewer), true, &mut forged, &mut slipped);
        }
        // wrong number of free groups
        try_one(write_solvable(n, &alphabet, &cons, &labels, groups, free_of(&labels, groups) + 1, &links), true, &mut forged, &mut slipped);
        // every position its own group
        let alone: Vec<usize> = (0..n).collect();
        try_one(write_solvable(n, &alphabet, &cons, &alone, n, free_of(&alone, n), &[]), groups != n || t.is_none(), &mut forged, &mut slipped);
        // a made-up contradiction for any puzzle: positions and rules at random
        let head = honest.split("claim").next().unwrap().to_string();
        let (a, b) = (rng.below(n), rng.below(n));
        let fake = format!("{head}claim contradiction\nfirst {a} {} {}\nsecond {b} {} {}\nchain 2\n{a} {b}\nvia\n{}\nend\n", alphabet[0] as char, 1 + rng.below(cons.len().max(1)), alphabet[alphabet.len() - 1] as char, 1 + rng.below(cons.len().max(1)), 1 + rng.below(cons.len().max(1)));
        try_one(fake, t.is_some(), &mut forged, &mut slipped);
    }
    r.must(honest_bad == 0, "certificates built from plain union-find (not by the solver) are accepted exactly when the puzzle is solvable");
    r.must(slipped == 0, &format!("{forged} forgeries (groups merged, merged with a forged link, group split, link missing, wrong free count, all positions separate, made-up contradiction): none accepted with a false statement"));

    println!("C4  malformed input and boundary cases");
    let bad_inputs: Vec<(&str, Vec<u8>)> = vec![
        ("empty file", vec![]),
        ("only blank lines", b"\n\n\n".to_vec()),
        ("binary noise", (0..300u32).map(|i| (i * 37 % 256) as u8).collect()),
        ("wrong version", b"prefsuf-certificate 2\nlength 1\nletters a\nrules 0\nclaim solvable\ngroups 1\nfree 1\nlabels\n0\nforest 0\nend\n".to_vec()),
        ("no end", b"prefsuf-certificate 1\nlength 1\nletters a\nrules 0\nclaim solvable\ngroups 1\nfree 1\nlabels\n0\nforest 0\n".to_vec()),
        ("text after end", b"prefsuf-certificate 1\nlength 1\nletters a\nrules 0\nclaim solvable\ngroups 1\nfree 1\nlabels\n0\nforest 0\nend\nend\n".to_vec()),
        ("negative length", b"prefsuf-certificate 1\nlength -1\nletters a\nrules 0\nclaim solvable\ngroups 0\nfree 0\nlabels\nforest 0\nend\n".to_vec()),
        ("huge length", b"prefsuf-certificate 1\nlength 999999999999999999\nletters a\nrules 0\nclaim solvable\ngroups 0\nfree 0\nlabels\nforest 0\nend\n".to_vec()),
        ("overflowing number", b"prefsuf-certificate 1\nlength 99999999999999999999999999\nletters a\nrules 0\nclaim solvable\ngroups 0\nfree 0\nlabels\nforest 0\nend\n".to_vec()),
        ("huge rule count", b"prefsuf-certificate 1\nlength 1\nletters a\nrules 999999999999\nclaim solvable\ngroups 1\nfree 1\nlabels\n0\nforest 0\nend\n".to_vec()),
        ("repeated letter", b"prefsuf-certificate 1\nlength 1\nletters aa\nrules 0\nclaim solvable\ngroups 1\nfree 1\nlabels\n0\nforest 0\nend\n".to_vec()),
        ("word not in letters", b"prefsuf-certificate 1\nlength 1\nletters a\nrules 1\nP b\nclaim solvable\ngroups 1\nfree 0\nlabels\n0\nforest 0\nend\n".to_vec()),
        ("border longer than text", b"prefsuf-certificate 1\nlength 1\nletters a\nrules 1\nB 2\nclaim solvable\ngroups 1\nfree 1\nlabels\n0\nforest 0\nend\n".to_vec()),
        ("label out of range", b"prefsuf-certificate 1\nlength 2\nletters a\nrules 0\nclaim solvable\ngroups 2\nfree 2\nlabels\n0 2\nforest 0\nend\n".to_vec()),
        ("unused group", b"prefsuf-certificate 1\nlength 2\nletters a\nrules 0\nclaim solvable\ngroups 3\nfree 3\nlabels\n0 1\nforest 0\nend\n".to_vec()),
        ("too few labels", b"prefsuf-certificate 1\nlength 3\nletters a\nrules 0\nclaim solvable\ngroups 2\nfree 2\nlabels\n0 1\nforest 0\nend\n".to_vec()),
        ("link of a P rule", b"prefsuf-certificate 1\nlength 2\nletters a\nrules 1\nP a\nclaim solvable\ngroups 1\nfree 0\nlabels\n0 0\nforest 1\nlinks 1 1\n0\nend\n".to_vec()),
        ("link beyond the border", b"prefsuf-certificate 1\nlength 3\nletters a\nrules 1\nB 1\nclaim solvable\ngroups 1\nfree 1\nlabels\n0 0 0\nforest 2\nlinks 1 2\n0 1\nend\n".to_vec()),
        ("zero links section", b"prefsuf-certificate 1\nlength 2\nletters a\nrules 1\nB 1\nclaim solvable\ngroups 1\nfree 1\nlabels\n0 0\nforest 1\nlinks 1 0\nlinks 1 1\n0\nend\n".to_vec()),
        ("contradiction with equal letters", b"prefsuf-certificate 1\nlength 2\nletters ab\nrules 2\nP a\nS a\nclaim contradiction\nfirst 0 a 1\nsecond 1 a 2\nchain 2\n0 1\nvia\n1\nend\n".to_vec()),
        ("contradiction without a link", b"prefsuf-certificate 1\nlength 2\nletters ab\nrules 2\nP a\nS b\nclaim contradiction\nfirst 0 a 1\nsecond 1 b 2\nchain 2\n0 1\nvia\n1\nend\n".to_vec()),
        ("empty chain", b"prefsuf-certificate 1\nlength 2\nletters ab\nrules 2\nP a\nS b\nclaim contradiction\nfirst 0 a 1\nsecond 1 b 2\nchain 0\nvia\nend\n".to_vec()),
    ];
    let mut wrongly_ok = Vec::new();
    for (name, bytes) in &bad_inputs {
        match std::panic::catch_unwind(|| checker::check_text(bytes, None)) {
            Ok(Err(_)) => {}
            _ => wrongly_ok.push(*name),
        }
    }
    r.must(wrongly_ok.is_empty(), &format!("{} malformed certificates rejected without a crash {wrongly_ok:?}", bad_inputs.len()));
    let boundary: Vec<(usize, &[u8], Vec<Con>)> = vec![
        (0, b"a", vec![]),
        (0, b"ab", vec![Con::Border(0), Con::Prefix(vec![]), Con::Suffix(vec![])]),
        (1, b"a", vec![Con::Border(1), Con::Border(0)]),
        (1, b"ab", vec![Con::Prefix(b"a".to_vec()), Con::Suffix(b"b".to_vec())]),
        (2, b"ab", vec![Con::Border(1), Con::Prefix(b"a".to_vec()), Con::Suffix(b"b".to_vec())]),
        (5, b"x", vec![Con::Border(4)]),
        (6, b"ab", vec![Con::Border(6), Con::Border(0), Con::Border(3), Con::Border(3)]),
        (200, b"ab", (1..200).map(Con::Border).collect()),
        (130, b"ab", vec![Con::Border(1); 3]),
    ];
    let mut bad = 0;
    for (n, alphabet, cons) in &boundary {
        let cert = certificate(*n, alphabet, cons).unwrap();
        match checker::check_text(cert.as_bytes(), Some(&words(*n, alphabet, cons))) {
            Ok(v) => bad += !verdict_true(&v, truth(*n, alphabet, cons), group_count(*n, cons)) as u64,
            Err(_) => bad += 1,
        }
    }
    let all94: Vec<u8> = (33u8..127).collect();
    let cert = certificate(3, &all94, &[Con::Prefix(b"#".to_vec())]).unwrap();
    bad += checker::check_text(cert.as_bytes(), Some(&words(3, &all94, &[Con::Prefix(b"#".to_vec())]))).is_err() as u64;
    r.must(bad == 0, &format!("{} boundary puzzles (length 0 and 1, B0, Bn, empty words, one letter, 94 letters, every border, repeated rules): accepted, verdict true", boundary.len() + 1));
    println!("certificate checks failed: {}", r.fails);
    r.fails
}

fn big(n: usize) {
    println!("Certificates for texts of {n} characters (seconds; single runs).");
    println!("{:<34} {:>9} {:>9} {:>8} {:>9} {:>9}  {}", "puzzle", "borders", "solve", "emit", "size MB", "check", "verdict");
    let h = n / 2;
    let a = || Con::Prefix(b"a".to_vec());
    let b = || Con::Suffix(b"b".to_vec());
    let mut w = vec![b'a'; h + 1];
    w[h] = b'b';
    let cases: Vec<(&str, Vec<Con>)> = vec![
        ("no rules", vec![]),
        ("one border (half)", vec![Con::Border(h)]),
        ("two coprime periods at the bound", vec![Con::Border(n - h), Con::Border(n - h - 1)]),
        ("16 random borders", { let mut r = Lcg(5); (0..16).map(|_| Con::Border(1 + r.below(n - 1))).collect() }),
        ("every border", (1..n).map(Con::Border).collect()),
        ("even periods from n/2", (h..n).filter(|q| q % 2 == 0).map(|q| Con::Border(n - q)).collect()),
        ("every border, clash at the ends", (1..n).map(Con::Border).chain([a(), b()]).collect()),
        ("neighbours linked, clash at ends", vec![Con::Border(n - 1), a(), b()]),
        ("periods 4,8,.. and 2, clash", (1..n / 4).map(|j| Con::Border(n - 4 * j)).chain([Con::Border(n - 2), a(), Con::Suffix(b"ba".to_vec())]).collect()),
        ("every border, clash in the middle", (1..n).rev().map(Con::Border).chain([Con::Prefix(w)]).collect()),
    ];
    let mut fails = 0;
    for (name, cons) in cases {
        let t = Instant::now();
        let truth = truth(n, b"ab", &cons);
        let t_solve = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let cert = certificate(n, b"ab", &cons).unwrap();
        let t_emit = t.elapsed().as_secs_f64();
        let args = words(n, b"ab", &cons);
        let t = Instant::now();
        let res = checker::check_text(cert.as_bytes(), Some(&args));
        let t_check = t.elapsed().as_secs_f64();
        let verdict = match &res {
            Ok(v) if verdict_true(v, truth, group_count(n, &cons)) => {
                let mut s = v.clone();
                s.truncate(60);
                format!("VALID: {s}")
            }
            Ok(_) => {
                fails += 1;
                "VALID BUT UNTRUE".to_string()
            }
            Err(e) => {
                fails += 1;
                format!("REJECTED: {e}")
            }
        };
        println!("{:<34} {:>9} {:>9.3} {:>8.3} {:>9.1} {:>9.3}  {}", name, cons.len(), t_solve, t_emit, cert.len() as f64 / 1048576.0, t_check, verdict);
        // damage at this size: cut the file, change one label
        let cut = &cert.as_bytes()[..cert.len() * 2 / 3];
        if checker::check_text(cut, Some(&args)).is_ok() {
            fails += 1;
            println!("    FAIL: a truncated certificate was accepted");
        }
        // one label changed: must be rejected unless the statement stays true
        let changed = cert.replacen("\nlabels\n0 ", "\nlabels\n1 ", 1);
        if changed != cert {
            if let Ok(v) = checker::check_text(changed.as_bytes(), Some(&args)) {
                if !verdict_true(&v, truth, group_count(n, &cons)) {
                    fails += 1;
                    println!("    FAIL: a certificate with a changed label was accepted");
                }
            }
        }
    }
    println!("failures: {fails}");
    std::process::exit(if fails == 0 { 0 } else { 1 });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |i: usize, default: usize| args.get(i).and_then(|s| s.replace('_', "").parse().ok()).unwrap_or(default);
    match args.first().map(|s| s.as_str()) {
        None | Some("check") => std::process::exit(if check(num(1, 1) as u64) == 0 { 0 } else { 1 }),
        Some("big") => big(num(1, 5_000_000)),
        _ => {
            eprintln!("usage: [check [SEED]] | big [N]");
            std::process::exit(2);
        }
    }
}
