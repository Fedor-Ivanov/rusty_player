//! Selection in track lists and tile grids: clicks with modifiers and mouse marquee
//! selection.

use eframe::egui::{self, Pos2};
use std::collections::BTreeSet;

/// Mouse marquee selection. The start is stored in content coordinates (list or grid),
/// not screen ones: while dragging, the content may scroll, and the start point on screen
/// moves along with it
pub(super) struct Marquee {
    pub(super) start: Pos2,
    /// What was selected before starting (with ⌘/Ctrl the area is added to it)
    pub(super) base: BTreeSet<usize>,
}

/// Selection like in Finder: a click selects one item, ⌘/Ctrl+click adds or removes
/// an item, Shift+click selects a range from the last click. Items are list row
/// indices or grid tiles
#[derive(Default)]
pub(super) struct Selection {
    indices: BTreeSet<usize>,
    /// Where the range starts on Shift+click
    anchor: Option<usize>,
}

impl Selection {
    pub(super) fn click(&mut self, index: usize, modifiers: egui::Modifiers) {
        if modifiers.shift
            && let Some(anchor) = self.anchor
        {
            // ⌘/Ctrl together with Shift adds the range to the existing selection
            if !modifiers.command {
                self.indices.clear();
            }
            self.indices.extend(anchor.min(index)..=anchor.max(index));
        } else if modifiers.command {
            if !self.indices.remove(&index) {
                self.indices.insert(index);
            }
            self.anchor = Some(index);
        } else {
            self.indices = BTreeSet::from([index]);
            self.anchor = Some(index);
        }
    }

    /// Marquee selection: `base` plus rows `range`. `anchor` — the row where dragging
    /// started: the range on the next Shift+click is counted from it
    pub(super) fn select_range(
        &mut self,
        base: &BTreeSet<usize>,
        range: std::ops::RangeInclusive<usize>,
        anchor: usize,
    ) {
        self.select_many(base, range, anchor);
    }

    /// Marquee selection in a grid: `base` plus tiles `items` — the frame captures
    /// a rectangle of tiles, not a contiguous range. `anchor` — as in `select_range`
    pub(super) fn select_many(
        &mut self,
        base: &BTreeSet<usize>,
        items: impl IntoIterator<Item = usize>,
        anchor: usize,
    ) {
        self.indices = base.iter().copied().chain(items).collect();
        self.anchor = Some(anchor);
    }

    pub(super) fn select_all(&mut self, len: usize) {
        self.indices = (0..len).collect();
    }

    pub(super) fn clear(&mut self) {
        self.indices.clear();
        self.anchor = None;
    }

    pub(super) fn contains(&self, index: usize) -> bool {
        self.indices.contains(&index)
    }

    pub(super) fn len(&self) -> usize {
        self.indices.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub(super) fn indices(&self) -> &BTreeSet<usize> {
        &self.indices
    }
}

#[cfg(test)]
mod tests {
    use super::Selection;
    use eframe::egui::Modifiers;

    fn selected(selection: &Selection) -> Vec<usize> {
        selection.indices().iter().copied().collect()
    }

    #[test]
    fn click_selects_only_one_track() {
        let mut selection = Selection::default();
        selection.click(2, Modifiers::NONE);
        selection.click(5, Modifiers::NONE);
        assert_eq!(selected(&selection), [5]);
    }

    #[test]
    fn command_click_toggles_tracks() {
        let mut selection = Selection::default();
        selection.click(2, Modifiers::NONE);
        selection.click(5, Modifiers::COMMAND);
        selection.click(7, Modifiers::COMMAND);
        selection.click(2, Modifiers::COMMAND);
        assert_eq!(selected(&selection), [5, 7]);
    }

    #[test]
    fn shift_click_selects_range_from_anchor() {
        let mut selection = Selection::default();
        selection.click(5, Modifiers::NONE);
        selection.click(2, Modifiers::SHIFT);
        assert_eq!(selected(&selection), [2, 3, 4, 5]);
        // The anchor stays at 5: a new Shift+click replaces the range rather than extending it
        selection.click(7, Modifiers::SHIFT);
        assert_eq!(selected(&selection), [5, 6, 7]);
    }

    #[test]
    fn command_shift_click_adds_range() {
        let mut selection = Selection::default();
        selection.click(0, Modifiers::NONE);
        selection.click(5, Modifiers::COMMAND);
        selection.click(7, Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(selected(&selection), [0, 5, 6, 7]);
    }

    #[test]
    fn marquee_range_replaces_or_adds_to_base() {
        let mut selection = Selection::default();
        selection.select_range(&Default::default(), 3..=6, 6);
        assert_eq!(selected(&selection), [3, 4, 5, 6]);

        let base = [0, 9].into_iter().collect();
        selection.select_range(&base, 4..=5, 4);
        assert_eq!(selected(&selection), [0, 4, 5, 9]);
        // Shift+click after a marquee counts the range from the row where dragging started
        selection.click(7, Modifiers::SHIFT);
        assert_eq!(selected(&selection), [4, 5, 6, 7]);
    }

    #[test]
    fn grid_marquee_selects_rectangle_of_tiles() {
        // A 4-column grid: the frame touched columns 1–2 in rows 0–1 — that's tiles 1, 2, 5, 6,
        // not the range 1..=6
        let mut selection = Selection::default();
        selection.select_many(&Default::default(), [1, 2, 5, 6], 1);
        assert_eq!(selected(&selection), [1, 2, 5, 6]);

        // With ⌘ the frame is added to the existing selection
        let base = [9].into_iter().collect();
        selection.select_many(&base, [1, 5], 1);
        assert_eq!(selected(&selection), [1, 5, 9]);
    }
}
