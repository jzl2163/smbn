use rand::distr::{Alphanumeric, SampleString};

pub fn new_id(prefix: &str) -> String {
    let suffix = Alphanumeric.sample_string(&mut rand::rng(), 10).to_ascii_lowercase();
    format!("{prefix}-{suffix}")
}

pub fn parse_principals(value: &str) -> Vec<String> {
    let mut output = value
        .split([',', ';', '\n', '\r'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    output.sort_by_key(|item| item.to_ascii_lowercase());
    output.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    output
}

pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let seconds = seconds % 60;
    if days > 0 {
        format!("{days}天 {hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}
