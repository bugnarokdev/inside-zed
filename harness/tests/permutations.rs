// The sixth adversarial case: three concurrent insertions at the same cursor,
// delivered in all six possible orders. The original draft never compiled
// (E0282, type annotations needed) and was withdrawn. Re-expressed here
// against the real `text` crate so it actually executes.

use clock::ReplicaId;
use text::{Buffer, BufferId};

fn buf(replica: u16, t: &str) -> Buffer {
    Buffer::new(ReplicaId::new(replica), BufferId::new(1).unwrap(), t)
}

fn permutations3() -> Vec<[usize; 3]> {
    vec![
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ]
}

#[test]
fn test_adversarial_three_same_cursor_insertions_all_orders_converge() {
    // Three replicas each insert one character at offset 1 of "ab".
    let ops: Vec<_> = (1..=3u16)
        .map(|replica| {
            let mut b = buf(replica, "ab");
            b.edit([(1..1, match replica {
                1 => "A",
                2 => "B",
                _ => "C",
            })])
        })
        .collect();

    let mut results = Vec::new();
    for order in permutations3() {
        let mut observer = buf(10, "ab");
        for &i in &order {
            observer.apply_ops([ops[i].clone()]);
        }
        assert!(
            !observer.has_deferred_ops(),
            "deferred ops left after delivering order {order:?}"
        );
        observer.check_invariants();
        results.push((order, observer.snapshot().text().to_string()));
    }

    // Every delivery order must produce byte-identical text.
    let first = results[0].1.clone();
    for (order, text) in &results {
        assert_eq!(
            *text, first,
            "order {order:?} produced {text:?} but order {:?} produced {first:?}",
            results[0].0
        );
    }

    // And every variant of the insertion must be preserved exactly once.
    assert_eq!(first.len(), 5, "expected 2 base + 3 inserted bytes, got {first:?}");
    for ch in ["A", "B", "C"] {
        assert_eq!(
            first.matches(ch).count(),
            1,
            "character {ch} must appear exactly once in {first:?}"
        );
    }

    println!("converged text for all 6 orders: {first:?}");
    for (order, text) in &results {
        println!("  order {order:?} -> {text:?}");
    }
}
