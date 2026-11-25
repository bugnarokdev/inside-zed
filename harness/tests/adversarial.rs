// The five adversarial scenarios from research/patches/text-adversarial-tests.patch,
// re-expressed against the real `text` crate so they run without gpui.
// Semantics are identical to the patch; only Buffer::text() is adapted to the
// snapshot accessor that upstream exposes.

use clock::ReplicaId;
use text::{Buffer, BufferId};

fn buf(replica: u16, t: &str) -> Buffer {
    Buffer::new(ReplicaId::new(replica), BufferId::new(1).unwrap(), t)
}

#[test]
fn test_adversarial_same_cursor_insertions_converge() {
    let mut a = buf(1, "ab");
    let mut b = buf(2, "ab");

    let op_a = a.edit([(1..1, "X")]);
    let op_b = b.edit([(1..1, "Y")]);

    a.apply_ops([op_b]);
    b.apply_ops([op_a]);

    assert_eq!(a.snapshot().text(), "aYXb");
    assert_eq!(b.snapshot().text(), "aYXb");
    a.check_invariants();
    b.check_invariants();
}

#[test]
fn test_adversarial_concurrent_delete_preserves_unobserved_insert() {
    let mut a = buf(1, "abcdef");
    let mut b = buf(2, "abcdef");

    let delete = a.edit([(2..5, "")]);
    let insert = b.edit([(3..3, "X")]);

    a.apply_ops([insert.clone()]);
    b.apply_ops([delete.clone()]);

    let mut c = buf(3, "abcdef");
    c.apply_ops([delete.clone(), insert.clone()]);
    let mut d = buf(4, "abcdef");
    d.apply_ops([insert, delete]);

    for buffer in [&a, &b, &c, &d] {
        assert_eq!(buffer.snapshot().text(), "abXf");
        buffer.check_invariants();
    }
}

#[test]
fn test_adversarial_overlapping_deletes_undo_independently() {
    let mut a = buf(1, "abcdef");
    let mut b = buf(2, "abcdef");

    let delete_a = a.edit([(1..4, "")]);
    let delete_b = b.edit([(2..5, "")]);

    a.apply_ops([delete_b]);
    b.apply_ops([delete_a]);
    assert_eq!(a.snapshot().text(), "af");
    assert_eq!(b.snapshot().text(), "af");

    let (_, undo_a) = a.undo().unwrap();
    b.apply_ops([undo_a]);
    assert_eq!(a.snapshot().text(), "abf");
    assert_eq!(b.snapshot().text(), "abf");
    a.check_invariants();
    b.check_invariants();

    let (_, undo_b) = b.undo().unwrap();
    a.apply_ops([undo_b]);
    assert_eq!(a.snapshot().text(), "abcdef");
    assert_eq!(b.snapshot().text(), "abcdef");
    a.check_invariants();
    b.check_invariants();
}

#[test]
fn test_adversarial_deferred_operation_applies_after_dependency() {
    let mut sender = buf(1, "ab");
    let mut receiver = buf(2, "ab");

    let first = sender.edit([(1..1, "X")]);
    let second = sender.edit([(2..2, "Y")]);

    receiver.apply_ops([second]);
    assert!(receiver.has_deferred_ops());

    receiver.apply_ops([first]);
    assert!(!receiver.has_deferred_ops());
    assert_eq!(receiver.snapshot().text(), "aXYb");
    receiver.check_invariants();
}

#[test]
fn test_adversarial_duplicate_delivery_is_idempotent() {
    let mut sender = buf(1, "ab");
    let mut receiver = buf(2, "ab");

    let operation = sender.edit([(1..1, "X")]);
    receiver.apply_ops([operation.clone(), operation.clone()]);
    assert_eq!(receiver.snapshot().text(), "aXb");

    receiver.apply_ops([operation]);
    assert_eq!(receiver.snapshot().text(), "aXb");
    receiver.check_invariants();
}
