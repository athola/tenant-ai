pub(crate) fn normalize_name(value: &str) -> String {
    let cleaned = value.replace(['\u{feff}', '\u{200b}'], "");
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.to_ascii_lowercase()
}

#[cfg(test)]
pub(crate) fn normalize_for_tests(value: &str) -> String {
    normalize_name(value)
}
