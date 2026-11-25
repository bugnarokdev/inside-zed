// Concurrency-model tests against Zed's real `clock` crate.
//
// Why this file exists: at the pinned upstream revision, crates/clock has
// 334 lines and zero `#[test]` functions. The Lamport clock and the version
// vector that every text operation depends on are exercised only indirectly,
// through the collaboration tests in the `text` crate. These tests call the
// clock API directly so the ordering rules are checked as rules, not as a
// side effect of an edit.

use clock::{Global, Lamport, ReplicaId};

fn l(replica: u16, value: u32) -> Lamport {
    Lamport {
        value,
        replica_id: ReplicaId::new(replica),
    }
}

// ---------------------------------------------------------------------------
// 1. Causality: a later value always sorts after an earlier one, regardless of
//    which replica produced it.
// ---------------------------------------------------------------------------

#[test]
fn causally_ordered_events_sort_by_value_not_replica() {
    assert!(l(9, 2) > l(1, 1));
    // Replica id must not be allowed to override causal order.
    assert!(l(1, 5) > l(9, 4));
}

// ---------------------------------------------------------------------------
// 2. Concurrency: equal values from different replicas need a deterministic
//    tie-break, otherwise two nodes could serialize the same pair of
//    concurrent edits differently and diverge.
// ---------------------------------------------------------------------------

#[test]
fn concurrent_events_get_a_total_order_from_the_replica_id() {
    let a = l(1, 7);
    let b = l(2, 7);
    assert_ne!(a.cmp(&b), std::cmp::Ordering::Equal);
    assert_eq!(a.cmp(&b).reverse(), b.cmp(&a));
    // Same replica, same value is the same event.
    assert_eq!(l(3, 7).cmp(&l(3, 7)), std::cmp::Ordering::Equal);
}

// ---------------------------------------------------------------------------
// 3. `observe` implements the Lamport receive rule. It must move the clock
//    strictly past the observed timestamp, including on the fast path where
//    the local clock is already ahead.
// ---------------------------------------------------------------------------

#[test]
fn observe_always_moves_the_clock_strictly_past_the_observed_value() {
    let mut local = Lamport::new(ReplicaId::new(1));
    let start = local.value;

    let remote = l(2, 100);
    local.observe(remote);
    assert!(
        local.value > remote.value,
        "after observing {} the local clock must exceed it, got {}",
        remote.value,
        local.value
    );
    assert_eq!(local.value, 101);

    // Observing an event from our own past must still advance by one, not
    // stand still: observe is unconditional.
    local.observe(l(1, start));
    assert_eq!(local.value, 102);
}

// ---------------------------------------------------------------------------
// 4. `as_u64` must be a lossless packing. If it collided, sorting by the
//    packed form would disagree with `Ord`.
// ---------------------------------------------------------------------------

#[test]
fn as_u64_packs_the_timestamp_without_collision() {
    let samples = [
        l(0, 0),
        l(1, 1),
        l(2, 1),
        l(1, 2),
        l(u16::MAX, 1),
        l(1, u32::MAX),
    ];
    let mut packed: Vec<u64> = samples.iter().map(|t| t.as_u64()).collect();
    packed.sort_unstable();
    packed.dedup();
    assert_eq!(packed.len(), samples.len(), "as_u64 collided");

    // Packed order must agree with Ord for causally related pairs.
    assert!(l(1, 2).as_u64() > l(9, 1).as_u64());
}

// ---------------------------------------------------------------------------
// 5. Version vector join: the merge of two replicas' knowledge. This is the
//    operation that decides whether a remote operation may be applied now or
//    has to wait. It must be commutative and associative, or two nodes that
//    receive the same set of operations in different orders would end up with
//    different knowledge.
// ---------------------------------------------------------------------------

#[test]
fn version_vector_join_is_commutative_and_associative() {
    let a: Global = [l(1, 3), l(2, 1)].into_iter().collect();
    let b: Global = [l(1, 1), l(2, 5)].into_iter().collect();
    let c: Global = [l(2, 2), l(3, 9)].into_iter().collect();

    let ab = {
        let mut x = a.clone();
        x.join(&b);
        x
    };
    let ba = {
        let mut x = b.clone();
        x.join(&a);
        x
    };
    assert_eq!(ab, ba, "join is not commutative");

    let left_assoc = {
        let mut x = ab.clone();
        x.join(&c);
        x
    };
    let right_assoc = {
        let mut y = b.clone();
        y.join(&c);
        let mut x = a.clone();
        x.join(&y);
        x
    };
    assert_eq!(left_assoc, right_assoc, "join is not associative");

    // Join is a max per replica, so it never loses knowledge.
    assert_eq!(ab.get(ReplicaId::new(1)), 3);
    assert_eq!(ab.get(ReplicaId::new(2)), 5);
    assert_eq!(ab.get(ReplicaId::new(3)), 0);
}

// ---------------------------------------------------------------------------
// 6. `observed` / `observed_all` are what gate deferred operations. A missing
//    dependency must read as "not observed" rather than as zero.
// ---------------------------------------------------------------------------

#[test]
fn dependency_checks_treat_unseen_replicas_as_unobserved() {
    let known: Global = [l(1, 3)].into_iter().collect();

    assert!(known.observed(l(1, 3)));
    assert!(known.observed(l(1, 2)));
    assert!(!known.observed(l(1, 4)));
    assert!(!known.observed(l(2, 1)));

    let requires_more: Global = [l(1, 3), l(2, 1)].into_iter().collect();
    assert!(known.observed_all(&known));
    assert!(!known.observed_all(&requires_more));

    // observed_any is weaker: one shared observed event is enough.
    assert!(known.observed_any(&requires_more));
}

// ---------------------------------------------------------------------------
// 7. `meet` looks like the dual of `join`, and on paper it is the per-replica
//    minimum. The subtlety is that a zero entry means "this replica has not
//    been observed", not "sequence 0". A literal element-wise minimum would
//    therefore discard knowledge that only one side has. The implementation
//    special-cases zero, which makes meet symmetric and idempotent, and makes
//    it keep the knowledge of a side that observed nothing.
//    Checked here because the doc comment describes the behaviour in prose
//    and a naive rewrite to `min` would silently break convergence.
// ---------------------------------------------------------------------------

#[test]
fn meet_takes_a_minimum_but_treats_zero_as_unobserved() {
    let a: Global = [l(1, 6), l(2, 1)].into_iter().collect();
    let b: Global = [l(1, 2), l(2, 9)].into_iter().collect();

    let mut j = a.clone();
    j.join(&b);
    assert_eq!(j.get(ReplicaId::new(1)), 6, "join is the per-replica max");
    assert_eq!(j.get(ReplicaId::new(2)), 9);

    let mut m = a.clone();
    m.meet(&b);
    assert_eq!(m.get(ReplicaId::new(1)), 2, "meet is the per-replica min");
    assert_eq!(m.get(ReplicaId::new(2)), 1);

    // The zero-means-unobserved rule: meeting with an empty vector must not
    // obliterate what only the other side knew.
    let empty = Global::new();
    let mut m_empty = a.clone();
    m_empty.meet(&empty);
    assert_eq!(
        m_empty.get(ReplicaId::new(1)),
        6,
        "an unobserved slot must not act as a sequence of 0"
    );
    let mut m_empty_other = empty.clone();
    m_empty_other.meet(&a);
    assert_eq!(
        m_empty_other, m_empty,
        "meet must be symmetric even when one side is empty"
    );

    // And it must be idempotent, since it is applied to pairs repeatedly.
    let mut twice = m.clone();
    twice.meet(&b);
    assert_eq!(twice, m);
}

// ---------------------------------------------------------------------------
// 8. Replica id space. The reserved ids and the collaboration threshold are
//    part of the wire contract: an id below FIRST_COLLAB_ID is never a remote
//    peer, so `is_remote` must agree with the constants.
// ---------------------------------------------------------------------------

#[test]
fn replica_id_reservations_match_the_is_remote_predicate() {
    assert!(!ReplicaId::LOCAL.is_remote());
    assert!(!ReplicaId::AGENT.is_remote());
    assert!(!ReplicaId::LOCAL_BRANCH.is_remote());
    assert!(ReplicaId::REMOTE_SERVER.is_remote());

    let below = ReplicaId::new(ReplicaId::FIRST_COLLAB_ID.as_u16() - 1);
    let at = ReplicaId::FIRST_COLLAB_ID;
    assert!(!below.is_remote());
    assert!(at.is_remote());
    assert_eq!(ReplicaId::FIRST_COLLAB_ID.as_u16(), 8);
}
