#!/bin/sh
//bin/true 2>/dev/null; d="$(dirname "$0")"; t="${TMPDIR:-/tmp}/prefsuf_bench"; [ -f "$d/prefsuf.rs" ] || { echo "put prefsuf.rs in the same folder as $0"; exit 1; }; mkdir -p "$t" && tail -n +3 "$d/prefsuf.rs" | sed 's/^fn main() {/fn orig_main() {/' > "$t/core.rs" && tail -n +3 "$0" > "$t/main.rs" && rustc --edition 2021 -O "$t/main.rs" -o "$t/run" && exec "$t/run" "$@"; exit $?
// Timing and memory harness for prefsuf.rs. Keep prefsuf.rs in the same folder.
//
//     sh bench.rs SHAPE N ALPHABET_SIZE REPEATS
//     sh bench.rs chain 1000000 2 3
//
// Shapes: sparse fixed chain half p2p3 period_fixed dup10 rand16 allborders
//         late_unsat early_unsat direct_unsat
#![allow(dead_code)]
include!("core.rs");
use std::time::Instant;

fn peak_rss_kb() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap();
    for l in s.lines() { if l.starts_with("VmHWM:") { return l.split_whitespace().nth(1).unwrap().parse().unwrap(); } }
    0
}
struct R2(u64);
impl R2 { fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 } 
          fn below(&mut self, m: usize) -> usize { (self.next() % m as u64) as usize } }

// independent class count: label components by scanning implicit border edges with own DSU
fn class_count(n: usize, cons: &[Con]) -> usize {
    let mut par: Vec<u32> = (0..n as u32).collect();
    fn f(p: &mut [u32], mut x: u32) -> u32 { while p[x as usize] != x { let g = p[p[x as usize] as usize]; p[x as usize] = g; x = g; } x }
    let mut classes = n;
    for c in cons { if let Con::Border(k) = c { let k=*k; if k==0||k==n {continue;} let p=n-k;
        for i in 0..k { let a=f(&mut par,i as u32); let b=f(&mut par,(i+p) as u32); if a!=b { par[a as usize]=b; classes-=1; } } } }
    classes
}

fn alphabet(k: usize) -> Vec<u8> { (33u8..127).take(k).collect() }

fn scenario(name: &str, n: usize, k: usize) -> Vec<Con> {
    let al = alphabet(k);
    let mut r = R2(0x9E3779B97F4A7C15 ^ n as u64);
    let word = |r: &mut R2, len: usize| -> Vec<u8> { (0..len).map(|_| al[r.below(k)]).collect() };
    match name {
        "sparse"      => vec![Con::Prefix(word(&mut r, n/10)), Con::Suffix(word(&mut r, n/10))],
        "fixed"       => vec![Con::Prefix(word(&mut r, n))],
        "chain"       => vec![Con::Border(n-1), Con::Prefix(vec![al[0]])],
        "half"        => vec![Con::Border(n/2)],
        "p2p3"        => vec![Con::Border(n-2), Con::Border(n-3)],
        "rand16"      => (0..16).map(|_| Con::Border(1 + r.below(n-1))).collect(),
        "dup10"       => vec![Con::Border(n-1); 10],
        "allborders"  => (1..n).map(Con::Border).collect(),
        "late_unsat"  => vec![Con::Border(n-1), Con::Prefix(vec![al[0]]), Con::Suffix(vec![al[1]])],
        "early_unsat" => vec![Con::Prefix(vec![al[0]]), Con::Suffix({let mut w = vec![al[1]; n]; w[n-1]=al[0]; w}), Con::Border(n-1)],
        "direct_unsat"=> vec![Con::Prefix(vec![al[0]; n]), Con::Suffix(vec![al[1]])],
        "period_fixed"=> { let p = 1000.min(n/2); vec![Con::Border(n-p), Con::Prefix(word(&mut r, p))] }
        _ => panic!("unknown scenario"),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 {
        println!("usage: sh bench.rs SHAPE N ALPHABET_SIZE REPEATS   e.g.  sh bench.rs chain 1000000 2 3");
        println!("shapes: sparse fixed chain half p2p3 period_fixed dup10 rand16 allborders late_unsat early_unsat direct_unsat");
        return;
    }
    let (name, n, k, reps): (&str, usize, usize, usize) = (&a[1], a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
    let al = alphabet(k);
    let cons = scenario(name, n, k);
    let ksum: usize = cons.iter().map(|c| if let Con::Border(b)=c { if *b<n {*b} else {0} } else {0}).sum();
    let lsum: usize = cons.iter().map(|c| match c { Con::Prefix(w)|Con::Suffix(w)=>w.len(), _=>0 }).sum();
    let mut times = Vec::new();
    let mut last = None;
    for _ in 0..reps { let t = Instant::now(); let out = solve(n, &al, &cons).unwrap(); times.push(t.elapsed().as_secs_f64()); last = Some(out); }
    let rss = peak_rss_kb();
    times.sort_by(|x,y| x.partial_cmp(y).unwrap());
    let out = last.unwrap();
    let classes = if a.len() > 5 { 0 } else { class_count(n, &cons) };
    let (status, free, cnt, extra) = match &out {
        Outcome::Unsat(cf) => ("UNSAT".to_string(), 0, "0".to_string(), format!("cert_path={} valid={}", cf.path.len(), check_conflict(cf, n, &cons))),
        Outcome::Sat{pattern, free} => {
            let w = witness(pattern, *free, &al);
            let c = match count(k, *free) { Some(c) => c.to_string(), None => format!("{}^{}", k, free) };
            ((if is_unique(k,*free) {"UNIQUE"} else {"MULTIPLE"}).to_string(), *free, c, format!("witness_ok={}", satisfies(&w, n, &al, &cons)))
        }
    };
    println!("{name}\tn={n}\tk={k}\tm={}\tK={ksum}\tL={lsum}\tmin={:.6}s\tmed={:.6}s\trssMB={:.0}\tclasses={classes}\tfree={free}\t{status}\tcount={cnt}\t{extra}",
        cons.len(), times[0], times[times.len()/2], rss as f64/1024.0);
}
