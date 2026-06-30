// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Fuzzy matching adapter for pickable items.

use std::sync::Arc;

use nucleo::Config;
use nucleo::Nucleo;
use nucleo::Snapshot;
use nucleo::Status;
use nucleo::Utf32String;
use nucleo::pattern::CaseMatching;
use nucleo::pattern::Normalization;

const TICK_TIMEOUT_MS: u64 = 10;

/// Items that can be fuzzy matched by the picker.
pub(crate) trait Pickable {
    /// Return the text matched by the picker.
    fn text(&self) -> String;
}

/// Fuzzy matcher state for the session picker.
pub(crate) struct Picker<I: Send + Sync + 'static> {
    matcher: Nucleo<I>,
    query: String,
}

impl<I: Pickable + Send + Sync + 'static> Picker<I> {
    /// Construct an empty fuzzy matcher seeded with `query`.
    pub(crate) fn new(query: String) -> Self {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
        let mut picker = Self { matcher, query };
        picker.reparse(false);
        picker
    }

    /// Clear the active query string.
    pub(crate) fn clear(&mut self) {
        self.query.clear();
        self.reparse(false);
    }

    /// Inject replacement items into the matcher.
    pub(crate) fn inject(&self, items: Vec<I>) {
        let injector = self.matcher.injector();
        for item in items {
            injector.push(item, |item, columns| {
                columns[0] = Utf32String::from(item.text())
            });
        }
    }

    /// Remove the trailing character from the active query string.
    pub(crate) fn pop(&mut self) {
        self.query.pop();
        self.reparse(false);
    }

    /// Append one character to the active query string.
    pub(crate) fn push(&mut self, ch: char) {
        self.query.push(ch);
        self.reparse(true);
    }

    /// Return the active query string.
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    /// Refresh fuzzy matches and return the currently visible rows.
    pub(crate) fn refresh(&mut self) -> (Status, &Snapshot<I>, &str) {
        let status = self.matcher.tick(TICK_TIMEOUT_MS);
        (status, self.matcher.snapshot(), &self.query)
    }

    /// Reset matcher contents while preserving the active query string.
    pub(crate) fn reset(&mut self) {
        self.matcher.restart(true);
    }

    /// Reparse the active query and tell nucleo whether the change appended text.
    fn reparse(&mut self, append: bool) {
        self.matcher.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            append,
        );
    }
}
