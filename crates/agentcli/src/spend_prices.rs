//! What a model's tokens cost, per million, as Anthropic lists them.
//!
//! Built in rather than fetched: the screen that shows spend must not need the
//! network to add it up. A model this table does not know is unpriced, and the
//! screen says how much of what it counted that was.

/// US dollars per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rates {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    /// A cache write kept five minutes: 1.25× input.
    pub cache_write_5m: f64,
    /// A cache write kept an hour: 2× input.
    pub cache_write_1h: f64,
}

const fn rates(input: f64, output: f64) -> Rates {
    Rates {
        input,
        output,
        cache_read: input / 10.0,
        cache_write_5m: input * 1.25,
        cache_write_1h: input * 2.0,
    }
}

const FABLE_5: Rates = rates(10.0, 50.0);
const OPUS_CURRENT: Rates = rates(5.0, 25.0);
const OPUS_LEGACY: Rates = rates(15.0, 75.0);
const SONNET_5: Rates = rates(2.0, 10.0);
const SONNET: Rates = rates(3.0, 15.0);
const HAIKU_4_5: Rates = rates(1.0, 5.0);
const HAIKU_3_5: Rates = rates(0.8, 4.0);
const HAIKU_3: Rates = Rates {
    input: 0.25,
    output: 1.25,
    cache_read: 0.03,
    cache_write_5m: 0.3,
    cache_write_1h: 0.5,
};

/// The rates for a model id, or `None` for one this table does not know.
pub fn rates_for(model: &str) -> Option<Rates> {
    let lower = model.trim().to_ascii_lowercase();
    let name = lower
        .trim_start_matches("anthropic/")
        .trim_start_matches("anthropic:")
        .replace('.', "-");
    let names = |family: &str| {
        name.match_indices(family).any(|(at, _)| {
            !name[at + family.len()..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_digit())
        })
    };
    let legacy_opus_4 = name.match_indices("opus-4").any(|(at, _)| {
        let rest = &name[at + "opus-4".len()..];
        rest.is_empty() || rest.starts_with("-20") || rest.starts_with('@') || rest == "-thinking"
    });
    Some(match () {
        _ if names("fable-5") => FABLE_5,
        _ if names("opus-5") => OPUS_CURRENT,
        _ if names("sonnet-5") => SONNET_5,
        _ if ["opus-4-8", "opus-4-7", "opus-4-6", "opus-4-5"]
            .iter()
            .any(|one| names(one)) =>
        {
            OPUS_CURRENT
        }
        _ if names("opus-4-1") || legacy_opus_4 => OPUS_LEGACY,
        // A later Opus 4 point release shares the current price.
        _ if name.contains("opus-4") => OPUS_CURRENT,
        _ if name.contains("sonnet-4") || names("sonnet-3-7") || names("sonnet-3-5") => SONNET,
        _ if names("haiku-4-5") => HAIKU_4_5,
        _ if names("haiku-3-5") => HAIKU_3_5,
        _ if names("haiku-3") => HAIKU_3,
        _ => return None,
    })
}

/// What one message cost, or `None` when its model is not priced.
pub fn cost_of(record: &crate::spend_scan::Record) -> Option<f64> {
    let rates = rates_for(&record.model)?;
    let per_token = |count: u64, rate: f64| count as f64 * rate / 1_000_000.0;
    Some(
        per_token(record.input, rates.input)
            + per_token(record.output, rates.output)
            + per_token(record.cache_read, rates.cache_read)
            + per_token(record.cache_write_5m, rates.cache_write_5m)
            + per_token(record.cache_write_1h, rates.cache_write_1h),
    )
}

#[cfg(test)]
#[path = "spend_prices_tests.rs"]
mod tests;
