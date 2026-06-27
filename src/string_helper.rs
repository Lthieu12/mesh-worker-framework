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

/// Converts an error source list into a human-readable string.
pub(crate) fn error_source_list_to_string(error_source_list: &[String]) -> String {

    if error_source_list.is_empty() {
        return "-".to_string();
    }
    error_source_list.join(" -> ")
}

/// Returns the root error source.
pub(crate) fn error_root_source(error_source_list: &[String]) -> String {

    match error_source_list.last() {
        Some(error_source) => error_source.clone(),
        None => "-".to_string(),
    }
}