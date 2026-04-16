use nucleo::{Config, Nucleo, Utf32String};
use std::sync::Arc;

pub struct FuzzyFinder {
    pub matcher: Nucleo<String>,
}

impl FuzzyFinder {
    pub fn new() -> Self {
        let notify = Arc::new(|| {});
        Self {
            matcher: Nucleo::new(Config::DEFAULT, notify, None, 1),
        }
    }

   pub fn populate(&mut self, items: Vec<String>) {
    self.matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
    let injector = self.matcher.injector();
    for item in items {
        injector.push(item, |data, dst| dst[0] = Utf32String::from(data.as_str()));
    }
} 

    pub fn query(&mut self, pattern: &str) -> Vec<String> {
        self.matcher.pattern.reparse(
            0,
            pattern,
            nucleo::pattern::CaseMatching::Smart,
            nucleo::pattern::Normalization::Smart,
            true,
        );
        loop {
            let status = self.matcher.tick(10);
            if !status.running { break; }
        }
        let snapshot = self.matcher.snapshot();
        (0..snapshot.matched_item_count())
            .filter_map(|i| snapshot.get_matched_item(i))
            .map(|item| item.data.clone())
            .collect()
    }
}

/// Old API shim — kept so nothing else breaks if fuzzy_score is imported anywhere
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    if candidate.to_lowercase().contains(&query.to_lowercase()) {
        Some(100)
    } else {
        None
    }
}
