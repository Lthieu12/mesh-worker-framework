//! String formatting helpers.

/// Returns a shortened human-readable representation of the string.
pub(crate) fn to_short_string(text: &str, max_len: usize) -> String {

    if text.len() <= max_len {
        return text.to_string();
    }

    if max_len <= 3 {
        return text[..max_len].to_string();
    }
    format!("{}...", &text[..max_len - 3])
}

/// Escapes a string for CSV output.
pub(crate) fn csv_escape(value: &str) -> String {

    format!("\"{}\"", value.replace('"', "\"\""))
}