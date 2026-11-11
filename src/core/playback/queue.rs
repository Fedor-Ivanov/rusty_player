//! Track list: playback order, shuffle, repeat, removal.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::core::library::Track;

/// Repeat mode: only one can be on
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    /// The current track on loop
    One,
    /// The whole list on loop
    All,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TrackList {
    pub tracks: Vec<Track>,
    pub current_index: Option<usize>,
    /// Shuffled playback order
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// Id of the saved playlist the queue is linked to: it was saved into it or
    /// started from it. "Save" then overwrites this playlist. Reset
    /// when the queue is replaced with other content or cleared. Old files lack the field
    #[serde(default)]
    pub playlist: Option<u64>,
    /// Playback order with `shuffle`: a permutation of `tracks` indices.
    /// Without `shuffle` tracks go in order, and this list is unused
    #[serde(skip)]
    pub(super) shuffle_order: Vec<usize>,
}

impl TrackList {
    /// The order tracks play in: shuffled or normal
    pub(super) fn play_order(&self) -> Vec<usize> {
        if self.shuffle {
            self.shuffle_order.clone()
        } else {
            (0..self.tracks.len()).collect()
        }
    }

    /// The track after the current one (`step` = 1) or before it (`step` = -1).
    /// At the list edge, with `wrap` goes to the other end, without it — `None`.
    pub(super) fn neighbor(&self, step: isize, wrap: bool) -> Option<usize> {
        let order = self.play_order();
        let len = order.len() as isize;
        if len == 0 {
            return None;
        }
        let Some(position) = self
            .current_index
            .and_then(|current| order.iter().position(|&index| index == current))
        else {
            return order.first().copied();
        };
        let target = position as isize + step;
        if (0..len).contains(&target) {
            Some(order[target as usize])
        } else if wrap {
            Some(order[target.rem_euclid(len) as usize])
        } else {
            None
        }
    }

    /// Reshuffles the order. The current track is put first, so that after
    /// enabling shuffle all others play before it repeats
    pub(super) fn reshuffle(&mut self) {
        let mut rest: Vec<usize> = (0..self.tracks.len())
            .filter(|&index| Some(index) != self.current_index)
            .collect();
        fastrand::shuffle(&mut rest);
        self.shuffle_order = self.current_index.into_iter().chain(rest).collect();
    }

    /// Removes tracks with indices `indices`. The others' indices shift, so
    /// the current track and the shuffled order are recomputed for the new indices.
    /// If the current track is removed, `current_index` becomes `None`.
    pub(super) fn remove(&mut self, indices: &BTreeSet<usize>) {
        // New index of each track; `None` — the track is removed
        let mut next = 0;
        let remap: Vec<Option<usize>> = (0..self.tracks.len())
            .map(|old| {
                (!indices.contains(&old)).then(|| {
                    next += 1;
                    next - 1
                })
            })
            .collect();
        let new_index = |old: usize| remap.get(old).copied().flatten();

        let mut old = 0;
        self.tracks.retain(|_| {
            old += 1;
            remap[old - 1].is_some()
        });
        self.current_index = self.current_index.and_then(new_index);
        self.shuffle_order = self
            .shuffle_order
            .iter()
            .filter_map(|&old| new_index(old))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::TrackList;
    use crate::core::library::Track;
    use std::{collections::BTreeSet, path::PathBuf};

    fn list(len: usize) -> TrackList {
        TrackList {
            tracks: (0..len)
                .map(|i| Track {
                    path: PathBuf::from(format!("{i}.mp3")),
                    title: i.to_string(),
                    artist: String::new(),
                    album_artist: String::new(),
                    duration_secs: 0,
                    year: None,
                    track_number: None,
                    disc_number: None,
                    album: String::new(),
                })
                .collect(),
            shuffle_order: vec![3, 0, 4, 1, 2],
            ..TrackList::default()
        }
    }

    fn titles(list: &TrackList) -> Vec<&str> {
        list.tracks.iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn remove_shifts_current_and_shuffle_order() {
        let mut list = list(5);
        list.current_index = Some(3);
        list.remove(&BTreeSet::from([0, 2]));

        assert_eq!(titles(&list), ["1", "3", "4"]);
        // Track "3" was at position 3, two before it were removed — now it's at position 1
        assert_eq!(list.current_index, Some(1));
        // Old order 3, 0, 4, 1, 2 without the removed 0 and 2 — in new indices
        assert_eq!(list.shuffle_order, [1, 2, 0]);
    }

    #[test]
    fn remove_current_track_clears_current_index() {
        let mut list = list(5);
        list.current_index = Some(2);
        list.remove(&BTreeSet::from([2]));

        assert_eq!(titles(&list), ["0", "1", "3", "4"]);
        assert_eq!(list.current_index, None);
    }
}
