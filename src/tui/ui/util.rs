//! Utility functions for UI rendering
//!
//! Pure functions for layout calculations, text parsing, and formatting.

use crate::parser::output::Alignment;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// Calculate a centered rectangular area within a parent area.
///
/// Returns a `Rect` that is centered both horizontally and vertically,
/// sized as a percentage of the parent area.
///
/// # Arguments
/// * `area` - The parent area to center within
/// * `percent_x` - Width as a percentage of parent (0-100)
/// * `percent_y` - Height as a percentage of parent (0-100)
pub fn centered_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

/// Detect checkbox markers in text for task list items.
///
/// Parses text to identify task list checkbox patterns (`[x]`, `[X]`, `[ ]`).
///
/// # Arguments
/// * `text` - The text to parse
///
/// # Returns
/// A tuple of `(is_task, is_checked, remaining_text)`:
/// - `is_task`: true if a checkbox pattern was found
/// - `is_checked`: true if the checkbox is checked (`[x]` or `[X]`)
/// - `remaining_text`: the text after the checkbox marker (or original text if no checkbox)
pub fn detect_checkbox_in_text(text: &str) -> (bool, bool, &str) {
    let trimmed = text.trim_start();

    // Check for [x] or [X] (checked)
    if let Some(stripped) = trimmed
        .strip_prefix("[x]")
        .or_else(|| trimmed.strip_prefix("[X]"))
    {
        return (true, true, stripped.trim_start());
    }

    // Check for [ ] (unchecked)
    if let Some(stripped) = trimmed.strip_prefix("[ ]") {
        return (true, false, stripped.trim_start());
    }

    // Not a task list item
    (false, false, text)
}

/// Align text within a fixed width using Unicode-aware width calculations.
///
/// Handles left, center, right, and none (defaults to left) alignments.
/// If text is longer than width, it will be truncated with ellipsis.
///
/// # Arguments
/// * `text` - The text to align
/// * `width` - The total width to align within (including padding)
/// * `alignment` - The alignment direction
///
/// # Returns
/// A string padded to the specified width with appropriate alignment.
pub fn align_text(text: &str, width: usize, alignment: &Alignment) -> String {
    // Use Unicode display width instead of character/byte length
    let text_width = text.width();

    // If text is longer than width, truncate it
    if text_width >= width {
        // TODO: Proper Unicode-aware truncation
        if width > 5 {
            // Approximate truncation - not perfect but better than nothing
            let approx_chars = width.saturating_sub(5);
            let truncated = text.chars().take(approx_chars).collect::<String>();
            return format!(" {}... ", truncated);
        }
        return format!(" {} ", text);
    }

    // Width includes padding we added earlier
    let content_width = width;

    match alignment {
        Alignment::Left | Alignment::None => {
            // Left-aligned: " text     "
            let right_padding = content_width.saturating_sub(text_width + 1);
            format!(" {}{}", text, " ".repeat(right_padding))
        }
        Alignment::Center => {
            // Center-aligned: "  text   "
            let total_padding = content_width.saturating_sub(text_width);
            let left_pad = total_padding / 2;
            let right_pad = total_padding - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        Alignment::Right => {
            // Right-aligned: "     text "
            let left_padding = content_width.saturating_sub(text_width + 1);
            format!("{}{} ", " ".repeat(left_padding), text)
        }
    }
}

/// Highlight search matches within text, returning a Line with styled spans.
///
/// Performs case-insensitive matching and splits the text into segments,
/// applying the highlight style to matched portions.
///
/// # Arguments
/// * `text` - The text to search within
/// * `query` - The search query (case-insensitive)
/// * `base_style` - Style for non-matched text
/// * `highlight_style` - Style for matched text
///
/// # Returns
/// A vector of Spans with appropriate styling applied
pub fn highlight_search_matches(
    text: &str,
    query: &str,
    base_style: Style,
    highlight_style: Style,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    let mut spans = Vec::new();
    let mut last_end = 0;

    // Find all matches
    let mut search_start = 0;
    while let Some(rel_pos) = text_lower[search_start..].find(&query_lower) {
        let match_start = search_start + rel_pos;
        let match_end = match_start + query.len();

        // Verify char boundaries
        if !text.is_char_boundary(match_start) || !text.is_char_boundary(match_end) {
            search_start = match_start + 1;
            continue;
        }

        // Add text before match
        if match_start > last_end {
            spans.push(Span::styled(
                text[last_end..match_start].to_string(),
                base_style,
            ));
        }

        // Add highlighted match
        spans.push(Span::styled(
            text[match_start..match_end].to_string(),
            highlight_style,
        ));

        last_end = match_end;
        search_start = match_end;

        if search_start >= text.len() {
            break;
        }
    }

    // Add remaining text after last match
    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_string(), base_style));
    }

    // If no matches found, return original text with base style
    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    spans
}

/// Build a Line with optional search highlighting.
///
/// Convenience wrapper that builds a complete Line, optionally with a prefix.
///
/// # Arguments
/// * `prefix` - Optional prefix spans to prepend
/// * `text` - The main text content
/// * `query` - Optional search query for highlighting
/// * `base_style` - Style for non-matched text
/// * `highlight_style` - Style for matched text
pub fn build_highlighted_line(
    prefix: Vec<Span<'static>>,
    text: &str,
    query: Option<&str>,
    base_style: Style,
    highlight_style: Style,
) -> Line<'static> {
    let mut spans = prefix;

    if let Some(q) = query {
        spans.extend(highlight_search_matches(
            text,
            q,
            base_style,
            highlight_style,
        ));
    } else {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod centered_area_tests {
        use super::*;

        #[test]
        fn test_centered_full_size() {
            let parent = Rect::new(0, 0, 100, 50);
            let result = centered_area(parent, 100, 100);
            assert_eq!(result.width, 100);
            assert_eq!(result.height, 50);
        }

        #[test]
        fn test_centered_half_size() {
            let parent = Rect::new(0, 0, 100, 50);
            let result = centered_area(parent, 50, 50);
            // Should be centered: x offset should be ~25, y offset should be ~12
            assert_eq!(result.width, 50);
            assert_eq!(result.height, 25);
            assert!(result.x >= 24 && result.x <= 26); // Allow for rounding
            assert!(result.y >= 11 && result.y <= 13);
        }

        #[test]
        fn test_centered_zero_size() {
            let parent = Rect::new(0, 0, 100, 50);
            let result = centered_area(parent, 0, 0);
            assert_eq!(result.width, 0);
            assert_eq!(result.height, 0);
        }
    }

    mod detect_checkbox_tests {
        use super::*;

        #[test]
        fn test_checked_lowercase() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[x] Task done");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "Task done");
        }

        #[test]
        fn test_checked_uppercase() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[X] Also done");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "Also done");
        }

        #[test]
        fn test_unchecked() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[ ] Not done yet");
            assert!(is_task);
            assert!(!is_checked);
            assert_eq!(text, "Not done yet");
        }

        #[test]
        fn test_not_a_task() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("Regular text");
            assert!(!is_task);
            assert!(!is_checked);
            assert_eq!(text, "Regular text");
        }

        #[test]
        fn test_with_leading_whitespace() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("  [x] Indented task");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "Indented task");
        }

        #[test]
        fn test_empty_task() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[x]");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "");
        }

        #[test]
        fn test_bracket_but_not_checkbox() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[a] Not a checkbox");
            assert!(!is_task);
            assert!(!is_checked);
            assert_eq!(text, "[a] Not a checkbox");
        }
    }

    mod align_text_tests {
        use super::*;

        #[test]
        fn test_left_align() {
            let result = align_text("Hi", 10, &Alignment::Left);
            assert_eq!(result, " Hi       ");
            assert_eq!(result.len(), 10);
        }

        #[test]
        fn test_right_align() {
            let result = align_text("Hi", 10, &Alignment::Right);
            assert_eq!(result, "       Hi ");
            assert_eq!(result.len(), 10);
        }

        #[test]
        fn test_center_align() {
            let result = align_text("Hi", 10, &Alignment::Center);
            assert_eq!(result, "    Hi    ");
            assert_eq!(result.len(), 10);
        }

        #[test]
        fn test_none_defaults_to_left() {
            let result = align_text("Hi", 10, &Alignment::None);
            assert_eq!(result, " Hi       ");
        }

        #[test]
        fn test_truncation_when_too_long() {
            let result = align_text("This is a very long text", 10, &Alignment::Left);
            assert!(result.contains("..."));
            // Should be truncated with ellipsis
        }

        #[test]
        fn test_exact_width() {
            let result = align_text("Test", 6, &Alignment::Left);
            assert_eq!(result, " Test ");
        }

        #[test]
        fn test_unicode_width() {
            // Japanese characters are typically 2 columns wide
            let result = align_text("日本", 10, &Alignment::Left);
            // "日本" is 4 columns wide (2 chars * 2 width each)
            // Result should be " 日本     " (1 space + 4 cols + 5 spaces = 10)
            assert_eq!(result.width(), 10);
        }

        #[test]
        fn test_center_odd_padding() {
            // When padding can't be split evenly, extra space goes to right
            let result = align_text("A", 10, &Alignment::Center);
            // "A" is 1 wide, 9 spaces to distribute: 4 left, 5 right
            assert_eq!(result, "    A     ");
        }
    }

    mod highlight_search_tests {
        use super::*;
        use ratatui::style::Color;

        #[test]
        fn test_no_match() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "xyz", base, highlight);
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].content.as_ref(), "Hello World");
        }

        #[test]
        fn test_single_match() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "World", base, highlight);
            assert_eq!(spans.len(), 2);
            assert_eq!(spans[0].content.as_ref(), "Hello ");
            assert_eq!(spans[1].content.as_ref(), "World");
            assert_eq!(spans[1].style, highlight);
        }

        #[test]
        fn test_case_insensitive() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "world", base, highlight);
            assert_eq!(spans.len(), 2);
            assert_eq!(spans[1].content.as_ref(), "World"); // Preserves original case
        }

        #[test]
        fn test_multiple_matches() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("foo bar foo", "foo", base, highlight);
            assert_eq!(spans.len(), 3);
            assert_eq!(spans[0].content.as_ref(), "foo");
            assert_eq!(spans[1].content.as_ref(), " bar ");
            assert_eq!(spans[2].content.as_ref(), "foo");
        }

        #[test]
        fn test_empty_query() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello", "", base, highlight);
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].content.as_ref(), "Hello");
        }

        #[test]
        fn test_match_at_start() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "Hello", base, highlight);
            assert_eq!(spans.len(), 2);
            assert_eq!(spans[0].content.as_ref(), "Hello");
            assert_eq!(spans[0].style, highlight);
            assert_eq!(spans[1].content.as_ref(), " World");
        }
    }
}
