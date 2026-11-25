// Measures the fairness of Zed's priority queue against the real weights.
//
// This binary does not reimplement the scheduler. It reads Priority::weight()
// out of crates/scheduler and replays the exact selection loop used by
// TestScheduler::step (walk a target drawn from [0, total_weight) down the
// weight vector), so the published shares come from Zed's own constants.
//
// Run: cargo run --release --bin priority-draw

use scheduler::Priority;

/// Deterministic xorshift64*, so the published numbers can be reproduced
/// exactly on any machine without depending on rand's version or seeding.
struct Rng(u64);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as u32
    }
}

/// The selection loop from TestScheduler::step, over the real weights.
/// Returns the index of the chosen candidate.
fn pick(weights: &[u32], rng: &mut Rng) -> usize {
    let total: u32 = weights.iter().sum();
    assert!(total > 0);
    let mut target = rng.next_u32() % total;
    for (i, &w) in weights.iter().enumerate() {
        if target < w {
            return i;
        }
        target -= w;
    }
    // Unreachable while total is the sum of weights, but returned rather
    // than panicked so a future weight change surfaces as skew, not a crash.
    weights.len() - 1
}

fn main() {
    let candidates = [
        ("High", Priority::High),
        ("Medium", Priority::Medium),
        ("Low", Priority::Low),
        ("RealtimeAudio", Priority::RealtimeAudio),
    ];

    println!("priority weights, read from crates/scheduler");
    println!("{:<15} {:>8} {:>12}", "priority", "weight", "nominal %");
    let total: u32 = candidates.iter().map(|(_, p)| p.weight()).sum();
    for (name, p) in &candidates {
        let w = p.weight();
        let pct = if total == 0 { 0.0 } else { 100.0 * w as f64 / total as f64 };
        println!("{:<15} {:>8} {:>11.2}%", name, w, pct);
    }
    println!("{:<15} {:>8}", "sum", total);
    println!();
    println!(
        "note: RealtimeAudio carries weight 0 and is excluded from the draw;"
    );
    println!("its nominal share is therefore 0, not the 100/4 split.");
    println!();

    const DRAWS: u32 = 1_000_000;
    println!("measured shares over {DRAWS} draws (xorshift64*, fixed seed)");

    // Drawn exactly as the scheduler does: only the three weighted priorities.
    let drawn = [
        ("High", Priority::High),
        ("Medium", Priority::Medium),
        ("Low", Priority::Low),
    ];
    let weights: Vec<u32> = drawn.iter().map(|(_, p)| p.weight()).collect();
    let mut rng = Rng(0x2545F4914F6CDD1D);
    let mut counts = vec![0u32; drawn.len()];
    for _ in 0..DRAWS {
        counts[pick(&weights, &mut rng)] += 1;
    }

    println!(
        "{:<15} {:>12} {:>10} {:>12}",
        "priority", "count", "measured %", "nominal %"
    );
    let mut max_dev = 0.0f64;
    for (i, (name, p)) in drawn.iter().enumerate() {
        let measured = 100.0 * counts[i] as f64 / DRAWS as f64;
        let nominal = 100.0 * p.weight() as f64 / 100.0;
        let dev = (measured - nominal).abs();
        if dev > max_dev {
            max_dev = dev;
        }
        println!(
            "{:<15} {:>12} {:>9.3}% {:>11.0}%",
            name, counts[i], measured, nominal
        );
    }
    println!("\nmax absolute deviation from nominal: {:.3} points", max_dev);

    // RealtimeAudio: prove it never wins even when it is a candidate.
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mixed = [Priority::RealtimeAudio, Priority::High, Priority::Low];
    let mixed_weights: Vec<u32> = mixed.iter().map(|p| p.weight()).collect();
    let mut audio_hits = 0u32;
    const MIXED_DRAWS: u32 = 200_000;
    for _ in 0..MIXED_DRAWS {
        if pick(&mixed_weights, &mut rng) == 0 {
            audio_hits += 1;
        }
    }
    println!(
        "\nRealtimeAudio selected {audio_hits} times in {MIXED_DRAWS} draws \
         while competing against High and Low"
    );
}
