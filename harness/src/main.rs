// Benchmark harness against the real zed crates, pinned at
// zed-industries/zed @ 2664a0aa1eae5dabf8f8273c03f88d53061ca7dc
//
// Every number printed here comes from executing the actual zed code in this
// process. Nothing is estimated or copied from documentation.
use std::time::Instant;

use clock::ReplicaId;
use text::{Buffer, BufferId};

fn buf(t: &str) -> Buffer {
    Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), t)
}

fn make_doc(lines: usize) -> String {
    let mut s = String::with_capacity(lines * 41);
    for i in 0..lines {
        s.push_str(&format!("fn line_{i:06}() {{ let x = {i}; }}\n"));
    }
    s
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// A. Does single-insert latency grow with document size?
fn bench_insert_scaling() {
    println!("== A. edit() at offset 0 vs document size (25 reps, fresh buffer) ==");
    println!("{:>8}  {:>12}  {:>12}  {:>14}", "lines", "median_us", "min_us", "ns_MB_edit");
    for &lines in &[1_000usize, 10_000, 50_000, 100_000, 200_000] {
        let doc = make_doc(lines);
        let mb = doc.len() as f64 / 1_048_576.0;
        let mut s = Vec::new();
        for _ in 0..25 {
            let mut b = buf(&doc);
            let t = Instant::now();
            b.edit([(0..0, "x")]);
            s.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let med = median(s.clone());
        let min = s.iter().cloned().fold(f64::INFINITY, f64::min);
        println!("{:>8}  {:>12.2}  {:>12.2}  {:>14.3}", lines, med, min, med * 1000.0 / mb);
    }
}

/// A2. Isolate the edit cost from Buffer::new by building once.
fn bench_insert_pure() {
    println!();
    println!("== A2. edit() alone on a pre-built buffer (100 reps) ==");
    println!("{:>8}  {:>12}  {:>16}", "lines", "median_us", "ns_per_doc_byte");
    for &lines in &[1_000usize, 50_000, 200_000] {
        let doc = make_doc(lines);
        let mut b = buf(&doc);
        let mut s = Vec::new();
        for i in 0..100 {
            let t = Instant::now();
            b.edit([(0..0, format!("{i}"))]);
            s.push(t.elapsed().as_secs_f64() * 1e6);
        }
        let med = median(s);
        println!("{:>8}  {:>12.3}  {:>16.4}", lines, med, med * 1000.0 / doc.len() as f64);
    }
}

/// B. Sequential typing at the end of a large document.
fn bench_typing() {
    println!();
    println!("== B. 1000 append edits at end of a 100k-line document ==");
    let doc = make_doc(100_000);
    let mut b = buf(&doc);
    let mut s = Vec::new();
    for i in 0..1000 {
        let end = b.snapshot().len();
        let t = Instant::now();
        b.edit([(end..end, if i % 10 == 0 { "\n" } else { "a" })]);
        s.push(t.elapsed().as_secs_f64() * 1e6);
    }
    let total: f64 = s.iter().sum();
    println!("  total   : {:.2} ms", total / 1000.0);
    println!("  median  : {:.2} us/edit", median(s.clone()));
    println!("  max     : {:.2} us/edit", s.iter().cloned().fold(0.0, f64::max));
    println!("  doc grew: {} -> {} bytes", doc.len(), b.snapshot().len());
}

/// C. Random-access point lookup.
fn bench_offset_to_point() {
    println!();
    println!("== C. offset_to_point() on 100k-line document (10k lookups) ==");
    let doc = make_doc(100_000);
    let b = buf(&doc);
    let snap = b.snapshot();
    let len = snap.len();
    let mut s = Vec::new();
    let mut acc = 0usize;
    for i in 0..10_000usize {
        let off = (i * 7919) % len;
        let t = Instant::now();
        let p = snap.offset_to_point(off);
        s.push(t.elapsed().as_secs_f64() * 1e9);
        acc = acc.wrapping_add(p.row as usize);
    }
    let mut sorted = s.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("  median  : {:.0} ns/lookup", median(s));
    println!("  p99     : {:.0} ns/lookup", sorted[sorted.len() * 99 / 100]);
    println!("  checksum: {acc}");
}

/// D. Undo grouping.
///
/// The compiled-in default depends on build features: prod is 300ms, but
/// `test-support` forces 0ns, and cargo unifies that feature when this crate's
/// tests are built. So the default is reported, then set explicitly to make the
/// grouping mechanism observable instead of measuring a build artifact.
fn bench_undo_grouping() {
    println!();
    println!("== D. undo grouping ==");
    let probe = buf("hello");
    println!(
        "  compiled-in default group_interval : {:?}",
        probe.transaction_group_interval()
    );

    for (label, interval) in [("300ms (production)", Some(300u64)), ("0ns (test)", Some(0))] {
        let mut b = buf("hello");
        if let Some(ms) = interval {
            b.set_group_interval(std::time::Duration::from_millis(ms));
        }
        b.edit([(5..5, " world")]);
        b.edit([(11..11, "!")]);
        let mut n = 0;
        while b.undo().is_some() {
            n += 1;
        }
        println!(
            "  {label:<20} -> {n} undo step(s) for 2 rapid edits, text back to {:?}",
            b.snapshot().text()
        );
    }
}

/// E. Rope chunk geometry.
fn bench_rope_slices() {
    println!();
    println!("== E. rope chunk geometry, 100k-line document ==");
    let doc = make_doc(100_000);
    let b = buf(&doc);
    let snap = b.snapshot();
    let rope = snap.as_rope();
    let mut chunks = 0usize;
    let mut max_chunk = 0usize;
    for c in rope.chunks() {
        chunks += 1;
        max_chunk = max_chunk.max(c.len());
    }
    println!("  bytes     : {}", snap.len());
    println!("  chunks    : {chunks}");
    println!("  avg chunk : {:.1} bytes", snap.len() as f64 / chunks as f64);
    println!("  max chunk : {max_chunk} bytes");

    let mut s = Vec::new();
    for i in 0..1000usize {
        let start = (i * 4099) % (snap.len() - 512);
        let t = Instant::now();
        let sl = rope.slice(start..start + 512);
        std::hint::black_box(sl.len());
        s.push(t.elapsed().as_secs_f64() * 1e9);
    }
    println!("  slice 512B: median {:.0} ns", median(s));
}

fn main() {
    println!("zed text engine harness");
    println!("source: crates persistently checked out at 2664a0aa1e");
    println!();
    bench_insert_scaling();
    bench_insert_pure();
    bench_typing();
    bench_offset_to_point();
    bench_undo_grouping();
    bench_rope_slices();
}
