use crate::helper::{grapheme_width, span_width};
use ratatui_core::text::Span;
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Default)]
pub(crate) struct LineWrapper;

impl LineWrapper {
    /// Splits a given line width into multiple smaller widths, ensuring each width
    /// is no larger than the specified maximum width.
    pub(crate) fn determine_split(line_width: usize, max_width: usize) -> Vec<usize> {
        if line_width == 0 {
            return vec![0];
        }

        let mut remaining_width = line_width;
        let mut split_widths = Vec::new();

        while remaining_width > 0 {
            let current_chunk = std::cmp::min(remaining_width, max_width);
            split_widths.push(current_chunk);
            remaining_width = remaining_width.saturating_sub(max_width);
        }

        split_widths
    }

    /// Wraps a line by grapheme clusters, so that a cluster is never split
    /// across two rows.
    pub(crate) fn wrap_line(line: &[char], max_width: usize, tab_width: usize) -> Vec<Vec<char>> {
        let line: String = line.iter().collect();
        let mut lines = Vec::new();
        let mut line_width = 0;
        let mut current_line = Vec::new();

        for grapheme in line.graphemes(true) {
            let width = grapheme_width(grapheme, tab_width);

            if line_width + width > max_width {
                lines.push(current_line.clone());
                current_line.clear();
                line_width = 0;
            }

            current_line.extend(grapheme.chars());
            line_width += width;
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    pub(crate) fn wrap_spans(
        spans: Vec<Span<'_>>,
        max_width: usize,
        tab_width: usize,
    ) -> Vec<Vec<Span<'_>>> {
        let mut wrapped_lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_line_width = 0;

        for span in spans {
            // If adding this span exceeds the max width, handle wrapping
            if current_line_width + span_width(&span, tab_width) > max_width {
                let mut remaining_span = span.clone();
                let mut split_at = max_width - current_line_width;

                while span_width(&remaining_span, tab_width) > split_at {
                    let (fitting_part, rest) =
                        Self::split_span_at(remaining_span, split_at, tab_width);
                    current_line.push(fitting_part.clone());
                    wrapped_lines.push(current_line.clone());

                    // Prepare for the next line
                    current_line.clear();
                    remaining_span = rest;
                    split_at = max_width;
                }

                // Add remaining part to the current line
                current_line_width = span_width(&remaining_span, tab_width);
                current_line.push(remaining_span);
            } else {
                // No wrapping needed, just add the span
                current_line_width += span_width(&span, tab_width);
                current_line.push(span);
            }
        }

        // Add any remaining content as the last line
        if !current_line.is_empty() {
            wrapped_lines.push(current_line);
        }

        wrapped_lines
    }

    /// Splits a string before the first grapheme cluster that does not fit
    /// into `split_at` columns.
    fn split_str_at(s: Cow<'_, str>, split_at: usize, tab_width: usize) -> (String, String) {
        let mut current_width = 0;
        for (i, grapheme) in s.grapheme_indices(true) {
            current_width += grapheme_width(grapheme, tab_width);
            if current_width > split_at {
                let (a, b) = s.split_at(i);
                return (a.to_string(), b.to_string());
            }
        }

        (s.to_string(), String::new())
    }

    fn split_span_at(span: Span, split_at: usize, tab_width: usize) -> (Span, Span) {
        let (a, b) = Self::split_str_at(span.content, split_at, tab_width);
        (Span::styled(a, span.style), Span::styled(b, span.style))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_spans() {
        let spans = vec![Span::raw("Hello"), Span::raw("World")];
        let wrapped_spans = LineWrapper::wrap_spans(spans, 3, 0);

        assert_eq!(wrapped_spans[0], vec![Span::raw("Hel")]);
        assert_eq!(wrapped_spans[1], vec![Span::raw("lo"), Span::raw("W")]);
        assert_eq!(wrapped_spans[2], vec![Span::raw("orl")]);
        assert_eq!(wrapped_spans[3], vec![Span::raw("d")]);
    }

    #[test]
    fn test_wrap_spans_with_emoji() {
        let spans = vec![Span::raw("Hell🙂!")];
        let wrapped_spans = LineWrapper::wrap_spans(spans, 4, 0);

        assert_eq!(wrapped_spans[0], vec![Span::raw("Hell")]);
        assert_eq!(wrapped_spans[1], vec![Span::raw("🙂!")]);
    }

    #[test]
    fn test_split_span_at_with_emoji() {
        let span = Span::raw("🙂!");
        let (left, right) = LineWrapper::split_span_at(span, 2, 0);

        assert_eq!(left, Span::raw("🙂"));
        assert_eq!(right, Span::raw("!"));
    }

    #[test]
    fn test_wrap_spans_with_grapheme_clusters() {
        // 👍🏽 and 👨‍👩‍👧 are one grapheme cluster of two columns each
        let spans = vec![Span::raw("ab👍🏽cd")];
        let wrapped_spans = LineWrapper::wrap_spans(spans, 4, 0);

        assert_eq!(wrapped_spans[0], vec![Span::raw("ab👍🏽")]);
        assert_eq!(wrapped_spans[1], vec![Span::raw("cd")]);

        let spans = vec![Span::raw("ab👨\u{200d}👩\u{200d}👧cd")];
        let wrapped_spans = LineWrapper::wrap_spans(spans, 4, 0);

        assert_eq!(
            wrapped_spans[0],
            vec![Span::raw("ab👨\u{200d}👩\u{200d}👧")]
        );
        assert_eq!(wrapped_spans[1], vec![Span::raw("cd")]);
    }

    #[test]
    fn test_wrap_line_with_grapheme_clusters() {
        let line: Vec<char> = "ab👍🏽cd".chars().collect();
        let wrapped_line = LineWrapper::wrap_line(&line, 4, 0);

        assert_eq!(wrapped_line[0], "ab👍🏽".chars().collect::<Vec<_>>());
        assert_eq!(wrapped_line[1], "cd".chars().collect::<Vec<_>>());
    }

    fn test_line_wrapper_determine_split() {
        let line_widths = LineWrapper::determine_split(5, 3);

        assert_eq!(line_widths[0], 3);
        assert_eq!(line_widths[1], 2);

        let line_widths = LineWrapper::determine_split(6, 3);

        assert_eq!(line_widths[0], 3);
        assert_eq!(line_widths[1], 3);
    }
}
