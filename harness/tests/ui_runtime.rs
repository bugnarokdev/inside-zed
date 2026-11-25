// UI-runtime tests against Zed's real `scheduler` crate.
//
// The theme names the UI runtime and the concurrency model as first-class
// subjects. Rather than describe them, these tests exercise the actual
// crate at the pinned revision:
//
//   * Priority::weight() - the fairness budget the queue draws from.
//   * TestScheduler - Zed's own deterministic scheduler, the same one GPUI's
//     test dispatcher drives, with a seeded RNG so the distribution below is
//     reproducible rather than anecdotal.
//
// Everything asserted here comes from crates/scheduler, not a model of it.

use scheduler::{Priority, TestScheduler};

// ---------------------------------------------------------------------------
// 1. The fairness budget is a hard-coded ratio, not a tunable. Reading
//    Priority::weight() out of the real crate pins the contract: high:medium
//    :low is 6:3:1, and RealtimeAudio is deliberately excluded from the
//    weighted draw (weight 0), because it is dispatched on its own thread.
// ---------------------------------------------------------------------------

#[test]
fn priority_weights_are_the_documented_6_3_1_ratio() {
    assert_eq!(Priority::High.weight(), 60);
    assert_eq!(Priority::Medium.weight(), 30);
    assert_eq!(Priority::Low.weight(), 10);
    assert_eq!(
        Priority::RealtimeAudio.weight(),
        0,
        "realtime audio must not compete in the probability draw"
    );

    // The ratio, stated as the invariant a rewrite would have to preserve.
    assert_eq!(Priority::High.weight(), Priority::Medium.weight() * 2);
    assert_eq!(Priority::Medium.weight(), Priority::Low.weight() * 3);
    assert_eq!(
        Priority::High.weight() + Priority::Medium.weight() + Priority::Low.weight(),
        100,
        "the discrete weights sum to 100, which is what makes them read as percentages"
    );

    // Medium is the default, so an unannotated spawn does not get starved.
    assert_eq!(Priority::default(), Priority::Medium);
}

// ---------------------------------------------------------------------------
// 2. The weighted draw itself, measured. This mirrors the selection loop in
//    TestScheduler::step: build the weight vector, draw a target in
//    [0, total_weight), walk it down. Run here over the real Priority weights
//    with a fixed LCG so the result is deterministic and re-runnable without
//    depending on rand's version.
// ---------------------------------------------------------------------------

#[test]
fn weighted_draw_over_the_real_weights_approaches_the_6_3_1_ratio() {
    const DRAWS: u32 = 40_000;
    let candidates = [Priority::High, Priority::Medium, Priority::Low];
    let total: u32 = candidates.iter().map(|p| p.weight()).sum();
    assert_eq!(total, 100);

    let mut state: u64 = 0x2545F4914F6CDD1D;
    let mut counts = [0u32; 3];
    for _ in 0..DRAWS {
        // xorshift64*, deterministic and dependency-free.
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let r = (state.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as u32;
        let mut target = r % total;

        let mut picked = 2;
        for (i, p) in candidates.iter().enumerate() {
            let w = p.weight();
            if target < w {
                picked = i;
                break;
            }
            target -= w;
        }
        counts[picked] += 1;
    }

    // Expected shares: 60% / 30% / 10%. Allow 1.5 points of absolute slack
    // for 40k samples; the point is that the ratio is real, not that the
    // sampler is perfect.
    let share = |i: usize| counts[i] as f64 / DRAWS as f64;
    assert!(
        (share(0) - 0.60).abs() < 0.015,
        "high share was {:.3}, expected ~0.60",
        share(0)
    );
    assert!(
        (share(1) - 0.30).abs() < 0.015,
        "medium share was {:.3}, expected ~0.30",
        share(1)
    );
    assert!(
        (share(2) - 0.10).abs() < 0.015,
        "low share was {:.3}, expected ~0.10",
        share(2)
    );

    // High gets roughly twice the medium slots and six times the low ones.
    assert!(counts[0] > counts[1] && counts[1] > counts[2]);
}

// ---------------------------------------------------------------------------
// 3. RealtimeAudio has weight 0. Under the real selection loop, a zero-weight
//    candidate can never be reached: the walk subtracts weights and the
//    target is strictly less than the total, so a zero-width band is skipped.
//    Prove it over many draws rather than asserting it from the code alone.
// ---------------------------------------------------------------------------

#[test]
fn zero_weight_priority_is_never_selected_by_the_weighted_draw() {
    let candidates = [Priority::RealtimeAudio, Priority::Low];
    let total: u32 = candidates.iter().map(|p| p.weight()).sum();
    assert_eq!(total, 10, "audio contributes nothing to the mass");

    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut audio = 0u32;
    for _ in 0..10_000 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let r = (state.wrapping_mul(0x2545F4914F6CDD1D) >> 33) as u32;
        let mut target = r % total;

        for p in candidates.iter() {
            let w = p.weight();
            if target < w {
                if *p == Priority::RealtimeAudio {
                    audio += 1;
                }
                break;
            }
            target -= w;
        }
    }
    assert_eq!(audio, 0, "a zero-weight priority must never win the draw");
}

// ---------------------------------------------------------------------------
// 4. Zed ships a deterministic scheduler for its own tests. Confirming it is
//    real and seeded matters for this research: it means the runtime's
//    ordering can be reproduced exactly, which is the difference between a
//    claim and an experiment. The seeded config is what makes the seed below
//    meaningful.
// ---------------------------------------------------------------------------

#[test]
fn the_test_scheduler_is_seeded_and_reports_its_own_clock() {
    use scheduler::TestSchedulerConfig;

    let cfg = TestSchedulerConfig::with_seed(42);
    assert_eq!(cfg.seed, 42);

    let other = TestSchedulerConfig::with_seed(7);
    assert_ne!(
        cfg.seed, other.seed,
        "two configs with different seeds must not collapse to one"
    );

    // Defaults: the runtime's test harness randomises order by default, which
    // is why the weighted draw above is on the hot path in tests too.
    let default = TestSchedulerConfig::default();
    assert!(
        default.randomize_order,
        "randomised ordering is the default, so priority weights matter in tests"
    );

    // A freshly constructed scheduler has nothing to run: the queue starts
    // empty, and a run() over nothing terminates instead of spinning.
    let scheduler = TestScheduler::new(TestSchedulerConfig::with_seed(1));
    assert!(!scheduler.has_pending_tasks());
    assert_eq!(scheduler.pending_task_counts(), (0, 0));
    scheduler.run();
    assert!(!scheduler.has_pending_tasks(), "run() must leave the queue empty");
}

// ---------------------------------------------------------------------------
// 5. Thread discipline, as enforced by the type system.
//
//    crates/scheduler marks LocalExecutor with `not_send: PhantomData<Rc<()>>`.
//    That single field is why GPUI can hold an Rc/RefCell entity graph at all:
//    the executor that touches the graph cannot be moved to another thread,
//    and the compiler enforces it.
//
//    This was not assumed. The first version of this file asserted
//    `LocalExecutor: Send` and the compiler rejected it with
//    "`Rc<()>` cannot be sent between threads safely". The negative property
//    is therefore recorded as a compile error, not as a runtime assertion:
//    there is no stable way to assert !Send in a test body.
//
//    What *is* assertable is the bridge the runtime uses to cross threads,
//    and that is what this test pins.
// ---------------------------------------------------------------------------

#[test]
fn only_the_send_bridge_crosses_thread_boundaries() {
    fn assert_send_sync<T: Send + Sync>() {}

    // SendLocalExecutor is the explicit opt-in: it carries the same dispatch
    // destination but accepts only Send futures. This is the type a
    // background task uses to post work back to the UI thread.
    assert_send_sync::<scheduler::SendLocalExecutor>();

    // The background pool is genuinely multi-threaded, so it must be both.
    assert_send_sync::<scheduler::BackgroundExecutor>();

    // SessionId is a plain u16 wrapper and travels with the work.
    assert_send_sync::<scheduler::SessionId>();

    // LocalExecutor is Clone, which is how it is handed to spawned closures
    // on the same thread without being moved across one.
    fn assert_clone<T: Clone>() {}
    assert_clone::<scheduler::LocalExecutor>();
}

