use std::cmp::{min, Ordering};

use crate::{
    helper::{find_matching_bracket, skip_empty_lines},
    state::selection::set_selection_with_lines,
};
use jagged::{index::RowIndex, Index2};

use super::Execute;
use crate::{
    helper::{max_col, max_col_normal, skip_whitespace, skip_whitespace_rev},
    EditorMode, EditorState,
};

#[derive(Clone, Debug, Copy)]
pub struct MoveForward(pub usize);

impl Execute for MoveForward {
    fn execute(&mut self, state: &mut EditorState) {
        for _ in 0..self.0 {
            if state.cursor.col >= max_col(&state.lines, &state.cursor, state.mode) {
                break;
            }
            state.cursor.col += 1;
        }
        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MoveBackward(pub usize);

impl Execute for MoveBackward {
    fn execute(&mut self, state: &mut EditorState) {
        for _ in 0..self.0 {
            if state.cursor.col == 0 {
                break;
            }
            let max_col = max_col(&state.lines, &state.cursor, state.mode);
            if state.cursor.col > max_col {
                state.cursor.col = max_col;
            }
            state.cursor.col = state.cursor.col.saturating_sub(1);
        }
        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MoveUp(pub usize);

impl Execute for MoveUp {
    fn execute(&mut self, state: &mut EditorState) {
        for _ in 0..self.0 {
            if state.cursor.row == 0 {
                break;
            }
            state.cursor.row = state.cursor.row.saturating_sub(1);
        }
        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MoveDown(pub usize);

impl Execute for MoveDown {
    fn execute(&mut self, state: &mut EditorState) {
        for _ in 0..self.0 {
            if state.cursor.row >= state.lines.len().saturating_sub(1) {
                break;
            }
            state.cursor.row += 1;
        }
        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

/// Move one word forward. Breaks on the first character that is not of
/// the same class as the initial character or breaks on line ending.
/// Furthermore, after the first break, whitespaces are skipped.
#[derive(Clone, Debug, Copy)]
pub struct MoveWordForward(pub usize);

impl Execute for MoveWordForward {
    fn execute(&mut self, state: &mut EditorState) {
        if state.lines.is_empty() {
            return;
        }

        state.clamp_column();

        for _ in 0..self.0 {
            move_word_forward(state);
        }

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

fn move_word_forward(state: &mut EditorState) {
    let start_char_class = CharacterClass::from(state.lines.get(state.cursor));

    let start_index = match (
        state.lines.is_last_col(state.cursor),
        state.lines.is_last_row(state.cursor),
    ) {
        (true, true) => return,
        (true, false) => {
            state.cursor = Index2::new(state.cursor.row.saturating_add(1), 0);
            return;
        }
        _ => Index2::new(state.cursor.row, state.cursor.col.saturating_add(1)),
    };

    for (next_char, index) in state.lines.iter().from(start_index) {
        if CharacterClass::from(next_char) != start_char_class {
            state.cursor = index;
            skip_whitespace(&state.lines, &mut state.cursor);
            return;
        }
    }

    let max_col = max_col(&state.lines, &state.cursor, state.mode);
    state.cursor = Index2::new(state.cursor.row, max_col);
}

/// Move one word forward to the end of the word.
#[derive(Clone, Debug, Copy)]
pub struct MoveWordForwardToEndOfWord(pub usize);
impl Execute for MoveWordForwardToEndOfWord {
    fn execute(&mut self, state: &mut EditorState) {
        if state.lines.is_empty() {
            return;
        }

        state.clamp_column();

        for _ in 0..self.0 {
            move_word_forward_to_end_of_word(state);
        }

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

fn move_word_forward_to_end_of_word(state: &mut EditorState) {
    let mut start_index = match (
        state.lines.is_last_col(state.cursor),
        state.lines.is_last_row(state.cursor),
    ) {
        (true, true) => return,
        (true, false) => Index2::new(state.cursor.row.saturating_add(1), 0),
        _ => Index2::new(state.cursor.row, state.cursor.col.saturating_add(1)),
    };
    skip_empty_lines(&state.lines, &mut start_index.row);
    skip_whitespace(&state.lines, &mut start_index);
    let start_char_class = CharacterClass::from(state.lines.get(start_index));

    for (next_char, index) in state.lines.iter().from(start_index) {
        // Break loop if characters don't belong to the same class
        if CharacterClass::from(next_char) != start_char_class {
            break;
        }
        state.cursor = index;

        // Break loop if it reaches the end of the line
        if state.lines.is_last_col(index) {
            break;
        }
    }
}

/// Move one word forward. Breaks on the first character that is not of
/// the same class as the initial character or breaks on line starts.
/// Skips whitespaces if necessary.
#[derive(Clone, Debug, Copy)]
pub struct MoveWordBackward(pub usize);

impl Execute for MoveWordBackward {
    fn execute(&mut self, state: &mut EditorState) {
        if state.lines.is_empty() {
            return;
        }

        let max_col = max_col(&state.lines, &state.cursor, state.mode);
        if state.cursor.col > max_col {
            state.cursor.col = max_col;
        }

        for _ in 0..self.0 {
            move_word_backward(state);
        }

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

fn move_word_backward(state: &mut EditorState) {
    let mut start_index = state.cursor;
    if start_index.row == 0 && start_index.col == 0 {
        return;
    }

    if start_index.col == 0 {
        state.cursor.row = start_index.row.saturating_sub(1);
        state.cursor.col = state.lines.last_col_index(state.cursor.row);
        return;
    }

    start_index.col = start_index.col.saturating_sub(1);
    skip_whitespace_rev(&state.lines, &mut start_index);
    let start_char_class = CharacterClass::from(state.lines.get(start_index));

    for (next_char, i) in state.lines.iter().from(start_index).rev() {
        // Break loop if characters don't belong to the same class
        if CharacterClass::from(next_char) != start_char_class {
            break;
        }
        start_index = i;
        // Break loop if it reaches the start of the line
        if i.col == 0 {
            break;
        }
    }

    state.cursor = start_index;
}

// Move the cursor to the start of the line.
#[derive(Clone, Debug, Copy)]
pub struct MoveToStartOfLine();

impl Execute for MoveToStartOfLine {
    fn execute(&mut self, state: &mut EditorState) {
        state.cursor.col = 0;

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}
// move to the first non-whitespace character in the line.
#[derive(Clone, Debug, Copy)]
pub struct MoveToFirst();

impl Execute for MoveToFirst {
    fn execute(&mut self, state: &mut EditorState) {
        // Like Vim's `^`, only ASCII whitespace is skipped, not the other
        // blanks of word motions, such as the ideographic space.
        state.cursor.col = state
            .lines
            .get(RowIndex::new(state.cursor.row))
            .and_then(|line| line.iter().position(|ch| !ch.is_ascii_whitespace()))
            .unwrap_or(0);

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

// Move the cursor to the end of the line.
#[derive(Clone, Debug, Copy)]
pub struct MoveToEndOfLine();

impl Execute for MoveToEndOfLine {
    fn execute(&mut self, state: &mut EditorState) {
        state.cursor.col = max_col(&state.lines, &state.cursor, state.mode);

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

// Move the cursor to the start of the buffer.
#[derive(Clone, Debug, Copy)]
pub struct MoveToFirstRow();

impl Execute for MoveToFirstRow {
    fn execute(&mut self, state: &mut EditorState) {
        state.cursor.row = 0;

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

// Move the cursor to the end of the buffer.
#[derive(Clone, Debug, Copy)]
pub struct MoveToLastRow();

impl Execute for MoveToLastRow {
    fn execute(&mut self, state: &mut EditorState) {
        state.cursor.row = state.lines.len().saturating_sub(1);

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

// Move the cursor to the closing bracket.
#[derive(Clone, Debug, Copy)]
pub struct MoveToMatchinBracket();

impl Execute for MoveToMatchinBracket {
    fn execute(&mut self, state: &mut EditorState) {
        let max_col = max_col_normal(&state.lines, &state.cursor);
        let index = Index2::new(state.cursor.row, state.cursor.col.min(max_col));
        if let Some(index) = find_matching_bracket(&state.lines, index) {
            state.cursor = index;
            if state.mode == EditorMode::Visual {
                set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
            }
        };
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MoveHalfPageDown();

impl Execute for MoveHalfPageDown {
    fn execute(&mut self, state: &mut EditorState) {
        let jump_rows = state.view.num_rows / 2;
        state.cursor.row = min(state.cursor.row + jump_rows, state.lines.last_row_index());

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MoveHalfPageUp();

impl Execute for MoveHalfPageUp {
    fn execute(&mut self, state: &mut EditorState) {
        let jump_rows = state.view.num_rows / 2;
        state.cursor.row = state.cursor.row.saturating_sub(jump_rows);

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MovePageDown();

impl Execute for MovePageDown {
    fn execute(&mut self, state: &mut EditorState) {
        let jump_rows = state.view.num_rows;
        let max_viewport_y = state.lines.len().saturating_sub(jump_rows);
        let old_viewport_y = state.view.viewport.y;

        state.view.viewport.y = min(state.view.viewport.y + jump_rows, max_viewport_y);

        if old_viewport_y == max_viewport_y {
            state.cursor.row = state.lines.last_row_index();
        } else {
            state.cursor.row = state.view.viewport.y;
        }

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MovePageUp();

impl Execute for MovePageUp {
    fn execute(&mut self, state: &mut EditorState) {
        let jump_rows = state.view.num_rows;
        let old_viewport_y = state.view.viewport.y;

        state.view.viewport.y = state.view.viewport.y.saturating_sub(jump_rows);

        if old_viewport_y == 0 {
            state.cursor.row = 0;
        } else {
            let last_visible_row = state.view.viewport.y + jump_rows.saturating_sub(1);
            state.cursor.row = min(last_visible_row, state.lines.last_row_index());
        }

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

/// Move to the next paragraph boundary (Vim `}`).
/// A paragraph boundary is the first blank line after a block of non-blank
/// lines, or the end of the buffer.
#[derive(Clone, Debug, Copy)]
pub struct MoveParagraphForward();

impl Execute for MoveParagraphForward {
    fn execute(&mut self, state: &mut EditorState) {
        let last = state.lines.last_row_index();
        let mut row = state.cursor.row;

        if !state.lines.is_last_row(state.cursor) {
            while row < last && state.lines.is_empty_row(row).unwrap_or(true) {
                row += 1;
            }
            while row < last && !state.lines.is_empty_row(row).unwrap_or(true) {
                row += 1;
            }

            state.cursor.row = row;
        }

        if state.lines.is_last_row(state.cursor) {
            state.cursor.col = max_col_normal(&state.lines, &state.cursor);
        } else {
            state.cursor.col = 0;
        };

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

/// Move to the previous paragraph boundary (Vim `{`).
#[derive(Clone, Debug, Copy)]
pub struct MoveParagraphBackward();

impl Execute for MoveParagraphBackward {
    fn execute(&mut self, state: &mut EditorState) {
        let mut row = state.cursor.row;

        if !state.lines.is_first_row(state.cursor) {
            while row > 0 && state.lines.is_empty_row(row).unwrap_or(true) {
                row -= 1;
            }
            while row > 0 && !state.lines.is_empty_row(row).unwrap_or(true) {
                row -= 1;
            }

            state.cursor.row = row;
        }

        state.cursor.col = 0;

        if state.mode == EditorMode::Visual {
            set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
        }
    }
}

/// Moves the cursor to the next occurrence of a character to the right on the
/// current line (Vim `f`). Does nothing if the character is not found.
///
/// The target is `None` until the key handler supplies the next keystroke via
/// [`Execute::char_arg`].
#[derive(Clone, Debug, Copy)]
pub struct FindForward(pub Option<char>);

impl Execute for FindForward {
    fn execute(&mut self, state: &mut EditorState) {
        let Some(target) = self.0 else {
            return;
        };
        if let Some(col) = find_char_forward(state, target) {
            state.cursor.col = col;
            if state.mode == EditorMode::Visual {
                set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
            }
        }
    }

    fn char_arg(&mut self) -> Option<&mut Option<char>> {
        Some(&mut self.0)
    }
}

/// Moves the cursor to just before the next occurrence of a character to the
/// right on the current line (Vim `t`). Does nothing if the character is not
/// found.
///
/// The target is `None` until the key handler supplies the next keystroke via
/// [`Execute::char_arg`].
#[derive(Clone, Debug, Copy)]
pub struct TillForward(pub Option<char>);

impl Execute for TillForward {
    fn execute(&mut self, state: &mut EditorState) {
        let Some(target) = self.0 else {
            return;
        };
        if let Some(col) = find_char_forward(state, target) {
            state.cursor.col = col.saturating_sub(1);
            if state.mode == EditorMode::Visual {
                set_selection_with_lines(&mut state.selection, state.cursor, &state.lines);
            }
        }
    }

    fn char_arg(&mut self) -> Option<&mut Option<char>> {
        Some(&mut self.0)
    }
}

/// Returns the column of the next occurrence of `target` to the right of the
/// cursor on the current line, if any.
pub(crate) fn find_char_forward(state: &EditorState, target: char) -> Option<usize> {
    let row = state.cursor.row;
    let start = Index2::new(row, state.cursor.col + 1);
    for (ch, index) in state.lines.iter().from(start) {
        if index.row != row {
            break;
        }
        if ch == Some(&target) {
            return Some(index.col);
        }
    }
    None
}

/// The class of a character for word motions and operators (`w`, `e`, `b`,
/// `dw`, `cw`, `iw`, ...): a word is a run of characters of one class.
///
/// The classes follow Vim's `utf_class()` (`src/mbyte.c`), so that, as in
/// Vim, `日本語のテキスト` is made of the words `日本語`, `の` and `テキスト`.
#[derive(Debug, Clone, Copy, Eq)]
pub(crate) enum CharacterClass {
    /// No character, as on an empty line. Not equal to any class, itself
    /// included.
    Unknown,
    /// ASCII letters and digits, `_`, Latin-1 letters, and the letters of
    /// scripts without a class of their own (Greek, Cyrillic, ...).
    Alphanumeric,
    /// Punctuation and symbols, ASCII, full-width and ideographic alike.
    /// Emoji count as symbols.
    Punctuation,
    /// Spaces, including the no-break and ideographic spaces.
    Whitespace,
    Superscript,
    Subscript,
    Braille,
    Hiragana,
    Katakana,
    CjkIdeograph,
    Hangul,
}

impl CharacterClass {
    /// Vim's classes of the characters above Latin-1 (`utf_class_buf()` in
    /// Vim's `src/mbyte.c`): sorted, non-overlapping ranges. Characters
    /// outside them are word characters.
    ///
    /// Rows marked `*` are not in Vim's table:
    /// - Vim gives emoji a class of their own; here they are symbols. The
    ///   `*` symbol rows cover the emoji outside Vim's symbol ranges (whole
    ///   symbol blocks, or single code points).
    /// - Vim treats variation selectors as composing characters, part of the
    ///   character before them. Motions here move by char, so the selectors
    ///   are symbols, which keeps a word together after emoji and symbols,
    ///   and the ideographic selectors are ideographs.
    /// - Vim gives tags the word class; here they are symbols, like the flag
    ///   emoji of a tag sequence.
    const CLASSES: [(char, char, Self); 78] = [
        ('\u{037e}', '\u{037e}', Self::Punctuation), // Greek question mark
        ('\u{0387}', '\u{0387}', Self::Punctuation), // Greek ano teleia
        ('\u{055a}', '\u{055f}', Self::Punctuation), // Armenian punctuation
        ('\u{0589}', '\u{0589}', Self::Punctuation), // Armenian full stop
        ('\u{05be}', '\u{05be}', Self::Punctuation),
        ('\u{05c0}', '\u{05c0}', Self::Punctuation),
        ('\u{05c3}', '\u{05c3}', Self::Punctuation),
        ('\u{05f3}', '\u{05f4}', Self::Punctuation),
        ('\u{060c}', '\u{060c}', Self::Punctuation),
        ('\u{061b}', '\u{061b}', Self::Punctuation),
        ('\u{061f}', '\u{061f}', Self::Punctuation),
        ('\u{066a}', '\u{066d}', Self::Punctuation),
        ('\u{06d4}', '\u{06d4}', Self::Punctuation),
        ('\u{0700}', '\u{070d}', Self::Punctuation), // Syriac punctuation
        ('\u{0964}', '\u{0965}', Self::Punctuation),
        ('\u{0970}', '\u{0970}', Self::Punctuation),
        ('\u{0df4}', '\u{0df4}', Self::Punctuation),
        ('\u{0e4f}', '\u{0e4f}', Self::Punctuation),
        ('\u{0e5a}', '\u{0e5b}', Self::Punctuation),
        ('\u{0f04}', '\u{0f12}', Self::Punctuation),
        ('\u{0f3a}', '\u{0f3d}', Self::Punctuation),
        ('\u{0f85}', '\u{0f85}', Self::Punctuation),
        ('\u{104a}', '\u{104f}', Self::Punctuation), // Myanmar punctuation
        ('\u{10fb}', '\u{10fb}', Self::Punctuation), // Georgian punctuation
        ('\u{1361}', '\u{1368}', Self::Punctuation), // Ethiopic punctuation
        ('\u{166d}', '\u{166e}', Self::Punctuation), // Canadian Syl. punctuation
        ('\u{1680}', '\u{1680}', Self::Whitespace),
        ('\u{169b}', '\u{169c}', Self::Punctuation),
        ('\u{16eb}', '\u{16ed}', Self::Punctuation),
        ('\u{1735}', '\u{1736}', Self::Punctuation),
        ('\u{17d4}', '\u{17dc}', Self::Punctuation), // Khmer punctuation
        ('\u{1800}', '\u{180a}', Self::Punctuation), // Mongolian punctuation
        ('\u{2000}', '\u{200b}', Self::Whitespace),  // spaces
        ('\u{200c}', '\u{2027}', Self::Punctuation), // punctuation and symbols
        ('\u{2028}', '\u{2029}', Self::Whitespace),
        ('\u{202a}', '\u{202e}', Self::Punctuation), // punctuation and symbols
        ('\u{202f}', '\u{202f}', Self::Whitespace),
        ('\u{2030}', '\u{205e}', Self::Punctuation), // punctuation and symbols
        ('\u{205f}', '\u{205f}', Self::Whitespace),
        ('\u{2060}', '\u{206f}', Self::Punctuation), // punctuation and symbols
        ('\u{2070}', '\u{207f}', Self::Superscript),
        ('\u{2080}', '\u{2094}', Self::Subscript),
        ('\u{20a0}', '\u{27ff}', Self::Punctuation), // all kinds of symbols
        ('\u{2800}', '\u{28ff}', Self::Braille),
        ('\u{2900}', '\u{2998}', Self::Punctuation), // arrows, brackets, etc.
        ('\u{29d8}', '\u{29db}', Self::Punctuation),
        ('\u{29fc}', '\u{29fd}', Self::Punctuation),
        ('\u{2b00}', '\u{2bff}', Self::Punctuation), // * misc. symbols and arrows
        ('\u{2e00}', '\u{2e7f}', Self::Punctuation), // supplemental punctuation
        ('\u{3000}', '\u{3000}', Self::Whitespace),  // ideographic space
        ('\u{3001}', '\u{3020}', Self::Punctuation), // ideographic punctuation
        ('\u{3030}', '\u{3030}', Self::Punctuation),
        ('\u{303d}', '\u{303d}', Self::Punctuation),
        ('\u{3040}', '\u{309f}', Self::Hiragana),
        ('\u{30a0}', '\u{30ff}', Self::Katakana),
        ('\u{3297}', '\u{3297}', Self::Punctuation), // * circled ideograph congratulation
        ('\u{3299}', '\u{3299}', Self::Punctuation), // * circled ideograph secret
        ('\u{3300}', '\u{9fff}', Self::CjkIdeograph),
        ('\u{ac00}', '\u{d7a3}', Self::Hangul),
        ('\u{f900}', '\u{faff}', Self::CjkIdeograph),
        ('\u{fd3e}', '\u{fd3f}', Self::Punctuation),
        ('\u{fe00}', '\u{fe0f}', Self::Punctuation), // * variation selectors
        ('\u{fe30}', '\u{fe6b}', Self::Punctuation), // punctuation forms
        ('\u{ff00}', '\u{ff0f}', Self::Punctuation), // half/fullwidth ASCII
        ('\u{ff1a}', '\u{ff20}', Self::Punctuation), // half/fullwidth ASCII
        ('\u{ff3b}', '\u{ff40}', Self::Punctuation), // half/fullwidth ASCII
        ('\u{ff5b}', '\u{ff65}', Self::Punctuation), // half/fullwidth ASCII
        ('\u{1d000}', '\u{1d24f}', Self::Punctuation), // musical notation
        ('\u{1d400}', '\u{1d7ff}', Self::Punctuation), // mathematical alphanumeric symbols
        ('\u{1f000}', '\u{1f2ff}', Self::Punctuation), // game pieces; enclosed characters
        ('\u{1f300}', '\u{1f9ff}', Self::Punctuation), // many symbol blocks
        ('\u{1fa70}', '\u{1faff}', Self::Punctuation), // * symbols and pictographs ext. A
        ('\u{20000}', '\u{2a6df}', Self::CjkIdeograph),
        ('\u{2a700}', '\u{2b73f}', Self::CjkIdeograph),
        ('\u{2b740}', '\u{2b81f}', Self::CjkIdeograph),
        ('\u{2f800}', '\u{2fa1f}', Self::CjkIdeograph),
        ('\u{e0020}', '\u{e007f}', Self::Punctuation), // * tags
        ('\u{e0100}', '\u{e01ef}', Self::CjkIdeograph), // * ideographic variation selectors
    ];
}

impl From<&char> for CharacterClass {
    fn from(value: &char) -> Self {
        let ch = *value;
        if u32::from(ch) < 0x100 {
            if ch.is_ascii_whitespace() || ch == '\u{a0}' {
                return Self::Whitespace;
            }
            // Underscore and the Latin-1 letters count as word characters
            // (matching Vim's default `iskeyword`, `@,48-57,_,192-255`).
            if ch.is_ascii_alphanumeric() || ch == '_' || ch >= '\u{c0}' {
                return Self::Alphanumeric;
            }
            return Self::Punctuation;
        }
        Self::CLASSES
            .binary_search_by(|&(first, last, _)| {
                if last < ch {
                    Ordering::Less
                } else if first > ch {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .map_or(Self::Alphanumeric, |i| Self::CLASSES[i].2)
    }
}

impl From<Option<&char>> for CharacterClass {
    fn from(value: Option<&char>) -> Self {
        value.map_or(CharacterClass::Unknown, Self::from)
    }
}

impl PartialEq for CharacterClass {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CharacterClass::Unknown, _) | (_, CharacterClass::Unknown) => false,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Index2, Lines};

    use super::*;
    fn test_state() -> EditorState {
        EditorState::new(Lines::from("Hello World!\n\n123."))
    }

    #[test]
    fn test_move_forward() {
        let mut state = test_state();

        MoveForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 1));

        MoveForward(10).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));

        MoveForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));
    }

    #[test]
    fn test_move_backward() {
        let mut state = test_state();
        state.cursor = Index2::new(0, 11);

        MoveBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 10));

        MoveBackward(10).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));

        MoveBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
    }

    #[test]
    fn test_move_down() {
        let mut state = test_state();
        state.cursor = Index2::new(0, 6);

        MoveDown(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 6));

        MoveDown(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 6));

        MoveDown(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 6));
    }

    #[test]
    fn test_move_up() {
        let mut state = test_state();
        state.cursor = Index2::new(2, 2);

        MoveUp(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 2));

        MoveUp(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 2));

        MoveUp(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 2));
    }

    #[test]
    fn test_move_word_forward() {
        let mut state = test_state();

        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 6));

        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));

        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 0));

        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 0));

        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 3));
    }

    #[test]
    fn test_move_word_forward_with_punctuation() {
        let mut state = EditorState::new(Lines::from("forward (w)"));

        // Start at 'f', move forward through "forward" and skip space to land on '('
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 8));

        // At '(', move through '(' and skip space (none) to land on 'w'
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 9));

        // At 'w', move through 'w' to land on ')'
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 10));
    }

    #[test]
    fn test_move_word_forward_punctuation_detailed() {
        // Test the exact case from the bug report
        let mut state = EditorState::new(Lines::from("forward (w)"));

        // Start at 'f' (position 0)
        assert_eq!(state.cursor, Index2::new(0, 0));

        // First move: from 'f' through "forward" to '('
        MoveWordForward(1).execute(&mut state);
        assert_eq!(
            state.cursor,
            Index2::new(0, 8),
            "Should move from 'f' to '(' after skipping whitespace"
        );

        // Second move: from '(' to 'w' (this was the bug - it went to ')' instead)
        MoveWordForward(1).execute(&mut state);
        assert_eq!(
            state.cursor,
            Index2::new(0, 9),
            "Should move from '(' to 'w', not to ')'"
        );

        // Third move: from 'w' to ')'
        MoveWordForward(1).execute(&mut state);
        assert_eq!(
            state.cursor,
            Index2::new(0, 10),
            "Should move from 'w' to ')'"
        );

        // Test with multiple punctuation characters
        let mut state = EditorState::new(Lines::from("hello()world"));

        // From 'h' to '('
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 5));

        // From '(' through ')' to 'w' (consecutive punctuation treated as one word)
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 7));

        // From 'w' through "world" to end
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));
    }

    #[test]
    fn test_move_word_forward_single_line() {
        // "Hello World" - should land on 'W' first, then on 'd' (end of line)
        let mut state = EditorState::new(Lines::from("Hello World"));

        // Start at 'H', move to 'W'
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 6));

        // From 'W', move to end of "World" (position 10, the 'd')
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 10));

        // From 'd', should stay at 'd' (already at end)
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 10));
    }

    #[test]
    fn test_move_word_forward_single_line_insert_mode() {
        // In insert mode, cursor should land AFTER 'd' (position 11)
        let mut state = EditorState::new(Lines::from("Hello World"));
        state.mode = EditorMode::Insert;

        // Start at 'H', move to 'W'
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 6));

        // From 'W', move to position after "World" (position 11, after 'd')
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));
    }

    #[test]
    fn test_move_word_forward_out_of_bounds() {
        let mut state = test_state();

        state.cursor = Index2::new(0, 99);
        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 0));
    }

    #[test]
    fn test_move_page_down() {
        // After page down, viewport scrolls and cursor is at top of new viewport (screen row 0)
        let mut state = EditorState::new(Lines::from(
            "Line 0\nLine 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9",
        ));
        state.view.num_rows = 3; // Viewport shows 3 lines

        // Start at line 0, viewport at 0
        assert_eq!(state.cursor.row, 0);
        assert_eq!(state.view.viewport.y, 0);

        // Page down: viewport scrolls to line 3, cursor at top of viewport (line 3)
        MovePageDown().execute(&mut state);
        assert_eq!(state.view.viewport.y, 3);
        assert_eq!(state.cursor.row, 3); // cursor at screen row 0

        // Page down: viewport scrolls to line 6, cursor at top of viewport (line 6)
        MovePageDown().execute(&mut state);
        assert_eq!(state.view.viewport.y, 6);
        assert_eq!(state.cursor.row, 6); // cursor at screen row 0

        // Page down: viewport can only go to line 7 (10 lines - 3 visible = max 7)
        // cursor at top of viewport (line 7)
        MovePageDown().execute(&mut state);
        assert_eq!(state.view.viewport.y, 7);
        assert_eq!(state.cursor.row, 7); // cursor at screen row 0

        // Page down again: already at bottom, cursor moves to last line
        MovePageDown().execute(&mut state);
        assert_eq!(state.view.viewport.y, 7);
        assert_eq!(state.cursor.row, 9); // cursor at last line
    }

    #[test]
    fn test_move_page_up() {
        // After page up, viewport scrolls and cursor is at bottom of new viewport
        let mut state = EditorState::new(Lines::from(
            "Line 0\nLine 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9",
        ));
        state.view.num_rows = 3; // Viewport shows 3 lines
        state.cursor.row = 9; // Start at last line
        state.view.viewport.y = 7; // Viewport showing lines 7-9

        // Page up: viewport scrolls to line 4, cursor at bottom of viewport (line 6)
        MovePageUp().execute(&mut state);
        assert_eq!(state.view.viewport.y, 4);
        assert_eq!(state.cursor.row, 6); // cursor at screen row 2 (bottom)

        // Page up: viewport scrolls to line 1, cursor at bottom of viewport (line 3)
        MovePageUp().execute(&mut state);
        assert_eq!(state.view.viewport.y, 1);
        assert_eq!(state.cursor.row, 3); // cursor at screen row 2 (bottom)

        // Page up: viewport scrolls to line 0, cursor at bottom of viewport (line 2)
        MovePageUp().execute(&mut state);
        assert_eq!(state.view.viewport.y, 0);
        assert_eq!(state.cursor.row, 2); // cursor at screen row 2 (bottom)

        // Page up again: already at top, cursor moves to first line
        MovePageUp().execute(&mut state);
        assert_eq!(state.view.viewport.y, 0);
        assert_eq!(state.cursor.row, 0); // cursor at first line
    }

    #[test]
    fn test_move_word_forward_to_end_of_word() {
        let mut state = test_state();

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 4));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 10));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 2));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 3));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 3));
    }

    #[test]
    fn test_move_word_backward() {
        let mut state = test_state();
        state.cursor = Index2::new(2, 3);

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 0));

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(1, 0));

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 6));

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
    }

    #[test]
    fn test_move_word_forward_japanese() {
        // Vim: CJK ideographs, hiragana and katakana are separate classes
        // (`utf_class()`), so `w` from `日` stops at `の`, then at `テ`.
        let mut state = EditorState::new(Lines::from("日本語のテキスト"));

        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 3));

        MoveWordForward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 4));
    }

    #[test]
    fn test_move_word_forward_to_end_of_word_japanese() {
        // Vim: `e` from `日` stops at `語`, `の` and `ト`.
        let mut state = EditorState::new(Lines::from("日本語のテキスト"));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 2));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 3));

        MoveWordForwardToEndOfWord(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 7));
    }

    #[test]
    fn test_move_word_backward_japanese() {
        // Vim: `b` from `ト` stops at `テ`, `の` and `日`.
        let mut state = EditorState::new(Lines::from("日本語のテキスト"));
        state.cursor = Index2::new(0, 7);

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 4));

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 3));

        MoveWordBackward(1).execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
    }

    #[test]
    fn test_move_word_backward_to_word_at_second_column() {
        // Vim: `b` stops at the start of a word at column 1 when column 0
        // has another class: from `c` of `(abc` at `a`, from `」` of
        // `「日本」` at `日`, from `本` of `a日本` at `日`.
        for (line, col) in [("(abc", 3), ("「日本」", 3), ("a日本", 2)] {
            let mut state = EditorState::new(Lines::from(line));
            state.cursor = Index2::new(0, col);

            MoveWordBackward(1).execute(&mut state);
            assert_eq!(state.cursor, Index2::new(0, 1), "{line}");
        }
    }

    #[test]
    fn test_move_word_forward_full_width_punctuation_and_ideographic_space() {
        // Vim: `、` and `。` are punctuation, and the ideographic space is
        // skipped like a space, so `w` stops at `、`, `了`, `で`, `。` and `o`.
        let mut state = EditorState::new(Lines::from("はい、了解です。\u{3000}ok"));

        for col in [2, 3, 5, 7, 9] {
            MoveWordForward(1).execute(&mut state);
            assert_eq!(state.cursor, Index2::new(0, col));
        }
    }

    #[test]
    fn test_move_word_forward_other_scripts() {
        // Vim: Latin-1 and Cyrillic letters are word characters like ASCII
        // letters, and Hangul is a class of its own, so `w` stops at `П`,
        // `м`, `한` and `a`.
        let mut state = EditorState::new(Lines::from("naïve Привет мир 한국어abc"));

        for col in [6, 13, 17, 20] {
            MoveWordForward(1).execute(&mut state);
            assert_eq!(state.cursor, Index2::new(0, col));
        }
    }

    #[test]
    fn test_move_word_forward_emoji() {
        // A skin tone modifier and a variation selector do not end a word
        // (in Vim the selector is part of the emoji's cell), so `w` stops at
        // `👍`, `❤` and `x`.
        let mut state = EditorState::new(Lines::from("ok👍🏽 ❤\u{fe0f} x"));

        for col in [2, 5, 8] {
            MoveWordForward(1).execute(&mut state);
            assert_eq!(state.cursor, Index2::new(0, col));
        }
    }

    #[test]
    fn test_move_word_forward_emoji_outside_symbol_blocks() {
        // Vim: `⭐` (Miscellaneous Symbols and Arrows) and `㊙` (Enclosed CJK
        // Letters) are emoji, not word characters, so `w` from `o` stops at
        // the emoji, then at `x`.
        for line in ["ok⭐ x", "ok㊙ x"] {
            let mut state = EditorState::new(Lines::from(line));

            for col in [2, 4] {
                MoveWordForward(1).execute(&mut state);
                assert_eq!(state.cursor, Index2::new(0, col), "{line}");
            }
        }
    }

    #[test]
    fn test_character_classes_are_sorted() {
        // `CharacterClass::from` binary-searches the ranges.
        for (first, last, _) in CharacterClass::CLASSES {
            assert!(first <= last, "{first:?}..={last:?}");
        }
        for pair in CharacterClass::CLASSES.windows(2) {
            assert!(pair[0].1 < pair[1].0, "{pair:?}");
        }
    }

    #[test]
    fn test_move_to_start() {
        let mut state = test_state();
        state.cursor = Index2::new(0, 2);

        MoveToStartOfLine().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
    }

    #[test]
    fn test_move_to_end() {
        let mut state = test_state();
        state.cursor = Index2::new(0, 2);

        MoveToEndOfLine().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 11));
    }

    #[test]
    fn test_move_to_first() {
        let mut state = EditorState::new(Lines::from(" Hello"));
        state.cursor = Index2::new(0, 3);

        MoveToFirst().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 1));
    }

    #[test]
    fn test_move_to_first_stops_at_ideographic_space() {
        // Vim's `^` skips only spaces and tabs, unlike `w`.
        let mut state = EditorState::new(Lines::from(" \u{3000}Hello"));
        state.cursor = Index2::new(0, 4);

        MoveToFirst().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 1));
    }

    fn paragraph_state() -> EditorState {
        // Lines:
        //   0: "first paragraph"
        //   1: "still first"
        //   2: ""
        //   3: ""
        //   4: ""
        //   5: "second paragraph"
        //   6: ""
        //   7: ""
        //   8: "third paragraph"
        EditorState::new(Lines::from(
            "first paragraph\nstill first\n\n\n\nsecond paragraph\n\n\nthird paragraph",
        ))
    }

    #[test]
    fn test_move_paragraph_forward() {
        let mut state = paragraph_state();
        assert_eq!(state.cursor, Index2::new(0, 0));

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 0));

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(6, 0));

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(8, 14));

        // Already at last line
        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(8, 14));
    }

    #[test]
    fn test_move_paragraph_backward() {
        let mut state = paragraph_state();
        state.cursor = Index2::new(8, 0);

        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(7, 0));

        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(4, 0));

        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));

        // Already at first line
        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
    }

    #[test]
    fn test_move_paragraph_forward_from_middle() {
        let mut state = paragraph_state();
        state.cursor = Index2::new(1, 5);

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 0));
    }

    #[test]
    fn test_move_paragraph_backward_from_middle() {
        let mut state = paragraph_state();
        state.cursor = Index2::new(5, 5);

        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(4, 0));
    }

    #[test]
    fn test_move_paragraph_forward_visual_extends_selection() {
        use crate::actions::SwitchMode;
        let mut state = paragraph_state();
        SwitchMode(EditorMode::Visual).execute(&mut state);

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(2, 0));
        assert!(state.selection.is_some());
        let sel = state.selection.unwrap();
        assert_eq!(sel.start, Index2::new(0, 0));
    }

    #[test]
    fn test_move_paragraph_single_line() {
        let mut state = EditorState::new(Lines::from("just one line"));

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 12));

        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
    }

    #[test]
    fn test_move_paragraph_single_line_visual_selects_to_boundaries() {
        use crate::actions::SwitchMode;
        let mut state = EditorState::new(Lines::from("just one line"));
        state.cursor = Index2::new(0, 5);
        SwitchMode(EditorMode::Visual).execute(&mut state);

        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 0));
        let sel = state.selection.clone().unwrap();
        assert_eq!(sel.start, Index2::new(0, 5));
        assert_eq!(sel.end, Index2::new(0, 0));

        SwitchMode(EditorMode::Normal).execute(&mut state);
        state.cursor = Index2::new(0, 5);
        SwitchMode(EditorMode::Visual).execute(&mut state);

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor, Index2::new(0, 12));
        let sel = state.selection.unwrap();
        assert_eq!(sel.start, Index2::new(0, 5));
        assert_eq!(sel.end, Index2::new(0, 12));
    }

    #[test]
    fn test_move_paragraph_all_blank() {
        let mut state = EditorState::new(Lines::from("\n\n\n"));
        assert_eq!(state.cursor, Index2::new(0, 0));

        MoveParagraphForward().execute(&mut state);
        assert_eq!(state.cursor.row, 3);

        MoveParagraphBackward().execute(&mut state);
        assert_eq!(state.cursor.row, 0);
    }
}
