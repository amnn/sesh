// Copyright (c) Ashok Menon
// SPDX-License-Identifier: Apache-2.0

//! Fuzzy matching adapter for the session picker.

use std::sync::Arc;

use nucleo::Config;
use nucleo::Nucleo;
use nucleo::Utf32String;
use nucleo::pattern::CaseMatching;
use nucleo::pattern::Normalization;

use crate::app::App;
use crate::app::Item;
use crate::session::Session;

const MATCH_COLUMNS: u32 = 1;
const TICK_TIMEOUT_MS: u64 = 10;

/// Fuzzy matcher state for the session picker.
pub(crate) struct Picker {
    matcher: Nucleo<Item>,
}

impl Picker {
    /// Construct a fuzzy matcher for the provided sessions.
    pub(crate) fn new(sessions: Vec<Session>) -> Self {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, MATCH_COLUMNS);
        let injector = matcher.injector();

        for session in sessions {
            let item = Item {
                text: session.item(),
                session,
            };
            injector.push(item, |item, columns| {
                columns[0] = Utf32String::from(item.text.as_str())
            });
        }

        Self { matcher }
    }

    /// Refresh fuzzy matches into the app's currently visible rows.
    pub(crate) fn refresh_matches(&mut self, app: &mut App) {
        let mut status = self.matcher.tick(TICK_TIMEOUT_MS);
        while self.matcher.snapshot().item_count() == 0 && status.running {
            status = self.matcher.tick(TICK_TIMEOUT_MS);
        }

        let snapshot = self.matcher.snapshot();
        let matched = snapshot.matched_item_count();
        let items = snapshot
            .matched_items(0..matched)
            .map(|item| Item {
                session: item.data.session.clone(),
                text: item.data.text.clone(),
            })
            .collect();
        app.replace_visible_items(items);
    }

    /// Re-parse the current query string in the fuzzy matcher.
    pub(crate) fn set_query(&mut self, previous: &str, query: &str) {
        let append = query.starts_with(previous);
        self.matcher
            .pattern
            .reparse(0, query, CaseMatching::Smart, Normalization::Smart, append);
    }

    /// Return the number of items known to the matcher.
    pub(crate) fn total_items(&self) -> usize {
        self.matcher.snapshot().item_count() as usize
    }
}
