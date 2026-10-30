//! String formatting helpers.

/// Returns the longest prefix that fits within the specified byte length.
///
/// The returned slice always ends at a valid UTF-8 character boundary.
pub(crate) fn truncate_prefix(text: &str, max_bytes: usize) -> &str {
    let mut end = 0;

    for (index, character) in text.char_indices() {
        let next_end = index + character.len_utf8();

        if next_end > max_bytes {
            break;
        }

        end = next_end;
    }

    &text[..end]
}

/// Returns a shortened human-readable representation of the string.
pub(crate) fn to_short_string(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    if max_len <= 3 {
        return truncate_prefix(text, max_len).to_string();
    }

    format!("{}...", truncate_prefix(text, max_len - 3))
}

/// Escapes a string for CSV output.
pub(crate) fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Converts an error source list into a human-readable string.
pub(crate) fn error_context_list_to_string(error_source_list: &[String]) -> String {
    if error_source_list.is_empty() {
        return "-".to_string();
    }
    error_source_list.join(" -> ")
}

/// Returns the root error source.
pub(crate) fn error_root_context(error_source_list: &[String]) -> String {
    match error_source_list.last() {
        Some(error_source) => error_source.clone(),
        None => "-".to_string(),
    }
}
