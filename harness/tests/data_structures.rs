//! Tests against Zed's own `sum_tree` and `rope` crates, at the pinned
//! revision.
//!
//! The theme names "data structures" as a first-class subject, but the site
//! only described `SumTree` from reading the source. These scenarios exercise
//! the two claims that carry the most weight and are the easiest to get wrong
//! by reading alone:
//!
//!   1. A `SumTree` is a persistent (copy-on-write) structure. Cloning one and
//!      editing the clone must not disturb the original. This is the property
//!      that makes `Buffer` snapshots cheap, and it rests on `Arc` sharing
//!      inside `Node`, not on any explicit API promise.
//!   2. The summary at the root equals the fold of every item's summary. Every
//!      seek, every `extent`, and every coordinate conversion in the rope is
//!      downstream of that invariant. If it drifts, offsets silently land in
//!      the wrong place.
//!
//! Everything asserted here is measured by calling the crate. Nothing is
//! restated from a doc comment.

use rope::{Point, Rope};
use sum_tree::{Bias, ContextLessSummary, Dimension, Item, SumTree};

// ---------------------------------------------------------------------------
// A minimal item type, so the tree is exercised on a summary we can compute
// independently. Using text here would make a failure ambiguous: it could be
// the tree or the text accounting.
// ---------------------------------------------------------------------------

/// Summary of a run of integers: how many, and their sum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Total {
    count: u32,
    sum: i64,
}

impl ContextLessSummary for Total {
    fn zero() -> Self {
        Total { count: 0, sum: 0 }
    }

    fn add_summary(&mut self, other: &Self) {
        self.count += other.count;
        self.sum += other.sum;
    }
}

#[derive(Clone, Debug)]
struct Number(i64);

impl Item for Number {
    type Summary = Total;

    fn summary(&self, _: ()) -> Total {
        Total {
            count: 1,
            sum: self.0,
        }
    }
}

/// The summary used as a seek dimension: seeking to a `u32` target walks until
/// the running count reaches it. `Dim` is what `find` returns as its position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Dim(u32);

impl<'a> Dimension<'a, Total> for Dim {
    fn zero(_: ()) -> Self {
        Dim(0)
    }

    fn add_summary(&mut self, summary: &'a Total, _: ()) {
        self.0 += summary.count;
    }
}

/// A dimension over the running sum, so a seek target can fall strictly inside
/// an item. Needed to distinguish "between two items" from "within one item",
/// which the count dimension cannot express.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct SumDim(i64);

impl<'a> Dimension<'a, Total> for SumDim {
    fn zero(_: ()) -> Self {
        SumDim(0)
    }

    fn add_summary(&mut self, summary: &'a Total, _: ()) {
        self.0 += summary.sum;
    }
}

fn build(values: &[i64]) -> SumTree<Number> {
    SumTree::from_iter(values.iter().copied().map(Number), ())
}

// ---------------------------------------------------------------------------

/// The root summary must equal the fold of the items. This is the invariant
/// that every lookup in the tree depends on.
#[test]
fn root_summary_equals_the_fold_of_the_items() {
    // A count chosen to exceed one node's capacity, so the tree has interior
    // nodes and the summary is genuinely aggregated rather than stored flat.
    let values: Vec<i64> = (1..=300).collect();
    let tree = build(&values);

    let summary = tree.summary();
    assert_eq!(summary.count, 300, "root summary must count every item");
    assert_eq!(
        summary.sum,
        values.iter().sum::<i64>(),
        "root summary must sum every item"
    );

    let from_items: i64 = tree.iter().map(|n| n.0).sum();
    assert_eq!(summary.sum, from_items, "root and iteration must agree");
}

/// A clone shares structure and is independent on write. If this failed, every
/// snapshot in the editor would be a deep copy, and the near-flat edit timings
/// would be impossible.
#[test]
fn a_clone_is_independent_of_the_original_on_write() {
    let values: Vec<i64> = (1..=200).collect();
    let original = build(&values);
    let before = *original.summary();

    let mut clone = original.clone();
    clone.push(Number(1000), ());
    clone.push(Number(2000), ());

    assert_eq!(
        *original.summary(),
        before,
        "editing the clone must not change the original's summary"
    );
    assert_eq!(original.summary().count, 200);
    assert_eq!(clone.summary().count, 202);
    assert_eq!(
        clone.summary().sum,
        before.sum + 3000,
        "the clone must account for exactly what was pushed"
    );

    // The original's contents are unchanged, not merely its summary.
    let original_items: Vec<i64> = original.iter().map(|n| n.0).collect();
    assert_eq!(original_items, values);
}

/// Appending two trees must produce the summary of the concatenation. This is
/// what makes pasting and merging cheap.
#[test]
fn append_is_consistent_with_concatenation() {
    let left_values: Vec<i64> = (1..=150).collect();
    let right_values: Vec<i64> = (1000..=1200).collect();

    let mut left = build(&left_values);
    let right = build(&right_values);
    let expected_sum = left.summary().sum + right.summary().sum;

    left.append(right, ());

    assert_eq!(left.summary().count, 150 + 201);
    assert_eq!(left.summary().sum, expected_sum);

    let items: Vec<i64> = left.iter().map(|n| n.0).collect();
    let mut expected = left_values.clone();
    expected.extend(&right_values);
    assert_eq!(items, expected, "append must preserve item order");
}

/// Seeking is bias-sensitive, and this is the part easiest to get wrong by
/// reading. `Bias::Left` attaches to the item that *ends* at the target
/// boundary; `Bias::Right` attaches to the one that *starts* there. Both are
/// measured here rather than assumed.
#[test]
fn find_is_bias_sensitive_at_a_boundary() {
    let values: Vec<i64> = (1..=500).collect();
    let tree = build(&values);

    // Cumulative index 1 is the boundary between the first and second item.
    let (_, _, left) = tree.find::<Dim, _>((), &Dim(1), Bias::Left);
    let (_, _, right) = tree.find::<Dim, _>((), &Dim(1), Bias::Right);

    assert_eq!(
        left.map(|n| n.0),
        Some(1),
        "Bias::Left must attach to the item ending at the boundary"
    );
    assert_eq!(
        right.map(|n| n.0),
        Some(2),
        "Bias::Right must attach to the item starting at the boundary"
    );

    // With one unit of summary per item, *every* integer target is a boundary,
    // so no target can sit strictly inside an item and the two biases always
    // differ. That is a property of this dimension, not of the tree.
    for boundary in [1u32, 2, 200, 499] {
        let (_, _, left) = tree.find::<Dim, _>((), &Dim(boundary), Bias::Left);
        let (_, _, right) = tree.find::<Dim, _>((), &Dim(boundary), Bias::Right);
        assert_eq!(
            left.map(|n| n.0),
            Some(boundary as i64),
            "Bias::Left at boundary {boundary}"
        );
        assert_eq!(
            right.map(|n| n.0),
            Some(boundary as i64 + 1),
            "Bias::Right at boundary {boundary}"
        );
    }
}

/// A target strictly inside an item is the case where the bias must *not*
/// matter. Proving it needs a dimension that accumulates more than one unit
/// per item, so a target can fall within an item rather than between two.
#[test]
fn both_biases_agree_on_a_target_inside_an_item() {
    // Cumulative sums: 10, 30, 60. A target of 20 is strictly inside the
    // second item, which spans 10..30.
    let tree = build(&[10, 20, 30]);

    let (_, _, left) = tree.find::<SumDim, _>((), &SumDim(20), Bias::Left);
    let (_, _, right) = tree.find::<SumDim, _>((), &SumDim(20), Bias::Right);

    assert_eq!(left.map(|n| n.0), Some(20), "Bias::Left inside item 2");
    assert_eq!(right.map(|n| n.0), Some(20), "Bias::Right inside item 2");

    // And on a boundary, where the items meet at 30, they diverge again.
    let (_, _, left_b) = tree.find::<SumDim, _>((), &SumDim(30), Bias::Left);
    let (_, _, right_b) = tree.find::<SumDim, _>((), &SumDim(30), Bias::Right);
    assert_eq!(left_b.map(|n| n.0), Some(20), "Bias::Left ends on item 2");
    assert_eq!(right_b.map(|n| n.0), Some(30), "Bias::Right starts item 3");
}

/// Seeking must agree with a brute-force scan at every interior boundary, not
/// just the ones picked by hand. This is the oracle test: 500 items, every
/// boundary checked.
#[test]
fn find_agrees_with_a_linear_scan_at_every_boundary() {
    let values: Vec<i64> = (1..=500).collect();
    let tree = build(&values);

    for i in 0..values.len() {
        let (_, _, item) = tree.find::<Dim, _>((), &Dim(i as u32), Bias::Right);
        assert_eq!(
            item.map(|n| n.0),
            Some(values[i]),
            "Bias::Right seek to boundary {i} should return the item starting there"
        );
    }
}

/// `extent` must equal the root summary. It is the other public face of the
/// same aggregate, and a divergence between the two would mean two reputable
/// answers disagree.
#[test]
fn extent_matches_the_root_summary() {
    let values: Vec<i64> = (1..=321).collect();
    let tree = build(&values);

    // `extent` takes only the context; the bias belongs to `find`.
    let extent: Dim = tree.extent(());
    assert_eq!(
        extent.0,
        tree.summary().count,
        "extent and summary must report the same total"
    );
}

/// An empty tree must have a zero summary and yield no items, rather than
/// panicking on first or last.
#[test]
fn an_empty_tree_reports_zero_and_yields_nothing() {
    let tree: SumTree<Number> = SumTree::new(());

    assert!(tree.is_empty());
    assert_eq!(tree.summary().count, 0);
    assert_eq!(tree.summary().sum, 0);
    assert_eq!(tree.first().map(|n| n.0), None);
    assert_eq!(tree.last().map(|n| n.0), None);
    assert_eq!(tree.iter().count(), 0);
}

// ---------------------------------------------------------------------------
// Rope: the text layer built on the tree above.
// ---------------------------------------------------------------------------

fn sample_rope() -> Rope {
    // Mixed content on purpose: ASCII, a multi-byte character, a tab, a blank
    // line, and a CRLF, because each one is counted differently by the
    // dimensions the rope exposes.
    Rope::from("alpha\nbéta\tgamma\n\nlast line\r\n")
}

/// Byte offsets and (row, column) points must be inverse operations. Every
/// cursor placement, every selection, and every diagnostic anchor goes through
/// this pair.
#[test]
fn offset_and_point_are_inverse_at_every_line_start() {
    let rope = sample_rope();
    let text = "alpha\nbéta\tgamma\n\nlast line\r\n";

    let mut offset = 0usize;
    for (row, line) in text.split('\n').enumerate() {
        let point = rope.offset_to_point(offset);
        assert_eq!(
            point,
            Point::new(row as u32, 0),
            "byte offset {offset} should be the start of row {row}"
        );
        assert_eq!(
            rope.point_to_offset(Point::new(row as u32, 0)),
            offset,
            "round trip failed at row {row}"
        );
        offset += line.len() + 1;
    }
}

/// `Point.column` counts UTF-8 **bytes**, not characters. This is the crate's
/// sharpest edge: `Point` is a byte column, and the separate `PointUtf16` type
/// exists precisely because a character column is a different coordinate
/// space. Measuring it here prevents the two from being conflated.
#[test]
fn point_columns_are_byte_offsets_not_character_counts() {
    let rope = Rope::from("béta\n");

    // "b" is 1 byte, "é" is 2 bytes. Byte offset 3 is after the accent.
    let point = rope.offset_to_point(3);
    assert_eq!(
        point,
        Point::new(0, 3),
        "the column is the byte offset within the row, so after 'b'+'é' it is 3"
    );
    assert_eq!(
        rope.point_to_offset(point),
        3,
        "and the inverse must return the same byte offset"
    );

    // Character column 2 is the same position, reached through UTF-16/char
    // accounting rather than through `Point`.
    assert_eq!(
        "béta".chars().take(2).count(),
        2,
        "the position is 2 characters, which is why Point and PointUtf16 differ"
    );
}

/// `line_len` reports bytes, consistent with `Point.column`.
#[test]
fn line_len_reports_bytes() {
    let rope = Rope::from("béta\nalpha\n");

    assert_eq!(
        rope.line_len(0),
        5,
        "row 0 is 'béta': 4 characters but 5 UTF-8 bytes"
    );
    assert_eq!(rope.line_len(1), 5, "row 1 is 'alpha', five bytes");
}

/// The root summary's byte length must equal the byte length of the source
/// text, and the same for characters and rows.
#[test]
fn rope_summary_matches_the_source_text() {
    let text = "alpha\nbéta\tgamma\n\nlast line\r\n";
    let rope = Rope::from(text);

    let summary = rope.summary();
    assert_eq!(rope.len(), text.len(), "byte length");
    assert_eq!(summary.len, text.len(), "summarised byte length");
    assert_eq!(summary.chars, text.chars().count(), "character count");

    // Four newlines means the text ends on row 4 (zero-indexed).
    assert_eq!(
        summary.lines.row,
        text.matches('\n').count() as u32,
        "row count must equal the newline count"
    );
}

/// No chunk may exceed the tree's base capacity, and the chunks must
/// concatenate back to the source. The capacity is what the measurements page
/// reports as 128 bytes.
#[test]
fn chunks_respect_the_capacity_and_reconstruct_the_text() {
    // Large enough to force many chunks and a multi-level tree.
    let text: String = (0..5000)
        .map(|i| format!("line {i} with some padding text\n"))
        .collect();
    let rope = Rope::from(text.as_str());

    let mut reconstructed = String::new();
    let mut chunk_count = 0;
    for chunk in rope.chunks() {
        assert!(
            chunk.len() <= 128,
            "chunk of {} bytes exceeds the 128-byte capacity",
            chunk.len()
        );
        reconstructed.push_str(chunk);
        chunk_count += 1;
    }

    assert_eq!(reconstructed, text, "chunks must reconstruct the source");
    assert!(chunk_count > 1, "a 5000-line text must span many chunks");
}

/// A slice of the rope must agree with the equivalent slice of the string,
/// including when the boundaries are not on chunk edges.
#[test]
fn slices_agree_with_string_slicing() {
    let text: String = (0..200)
        .map(|i| format!("row{i}\n"))
        .collect();
    let rope = Rope::from(text.as_str());

    for (start, end) in [(0usize, 10usize), (13, 40), (100, 337), (900, text.len())] {
        let sliced = rope.slice(start..end).to_string();
        assert_eq!(
            sliced,
            text[start..end],
            "slice {start}..{end} does not match the source"
        );
    }
}
