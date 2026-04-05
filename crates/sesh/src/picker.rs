// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Fuzzy matching adapter for the session picker.

use std::sync::Arc;

use nucleo::Config;
use nucleo::Nucleo;
use nucleo::Snapshot;
use nucleo::Status;
use nucleo::Utf32String;
use nucleo::pattern::CaseMatching;
use nucleo::pattern::Normalization;

const TICK_TIMEOUT_MS: u64 = 10;

/// Items that can be displayed and matched in the picker.
pub(crate) trait Item {
    /// Return the text shown for this item in the picker list.
    fn text(&self) -> String;
}

/// Fuzzy matcher state for the session picker.
pub(crate) struct Picker<I: Send + Sync + 'static> {
    query: String,
    matcher: Nucleo<I>,
}

impl<I: Item + Send + Sync + 'static> Picker<I> {
    /// Construct a fuzzy matcher for the provided sessions.
    pub(crate) fn new(items: Vec<I>) -> Self {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
        let injector = matcher.injector();

        for item in items {
            injector.push(item, |item, columns| {
                columns[0] = Utf32String::from(item.text().as_str())
            });
        }

        Self {
            query: String::new(),
            matcher,
        }
    }

    /// Refresh fuzzy matches and return the currently visible rows.
    pub(crate) fn refresh(&mut self) -> (Status, &Snapshot<I>, &str) {
        let status = self.matcher.tick(TICK_TIMEOUT_MS);
        (status, self.matcher.snapshot(), &self.query)
    }

    /// Return the current snapshot of visible rows without refreshing against the matcher state.
    pub(crate) fn snapshot(&self) -> &Snapshot<I> {
        self.matcher.snapshot()
    }

    /// Append one character to the active query string.
    pub(crate) fn push(&mut self, ch: char) {
        self.query.push(ch);
        self.matcher.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            true,
        );
    }

    /// Remove the trailing character from the active query string.
    pub(crate) fn pop(&mut self) {
        self.query.pop();
        self.matcher.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            false,
        );
    }

    /// Clear the active query string.
    pub(crate) fn clear(&mut self) {
        self.query.clear();
        self.matcher.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            false,
        );
    }
}
