#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};

const MAX_DOCUMENT: usize = 1024 * 1024;
const EDITOR_X: i32 = 20;
const EDITOR_W: u32 = 720;
const EDITOR_H: u32 = 342;
const LINE_HEIGHT: i32 = 18;
const CHARS_PER_LINE: usize = 86;

struct Document {
    text: String,
    cursor: usize,
    path: Option<String>,
    dirty: bool,
    first_line: usize,
    status: String,
    selection: Option<(usize, usize)>,
}

impl Document {
    fn new() -> Self {
        Self { text: String::new(), cursor: 0, path: None, dirty: false, first_line: 0, status: String::from("Ready"), selection: None }
    }

    fn line_count(&self) -> usize { self.text.bytes().filter(|&byte| byte == b'\n').count() + 1 }

    fn cursor_line(&self) -> usize { self.text[..self.cursor].bytes().filter(|&byte| byte == b'\n').count() }

    fn clamp_scroll(&mut self) {
        let visible = (EDITOR_H as usize / LINE_HEIGHT as usize).max(1);
        let line = self.cursor_line();
        if line < self.first_line { self.first_line = line; }
        if line >= self.first_line + visible { self.first_line = line + 1 - visible; }
        self.first_line = self.first_line.min(self.line_count().saturating_sub(visible));
    }

    fn insert(&mut self, ch: char) {
        if let Some((start, end)) = self.selection.take() {
            self.text.drain(start..end);
            self.cursor = start;
        }
        if self.text.len().saturating_add(ch.len_utf8()) > MAX_DOCUMENT { self.status = String::from("Document limit reached (1 MiB)"); return; }
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.dirty = true;
        self.status = String::from("Modified");
    }

    fn backspace(&mut self) {
        if let Some((start, end)) = self.selection.take() {
            self.text.drain(start..end);
            self.cursor = start;
            self.changed();
            return;
        }
        if self.cursor == 0 { return; }
        let start = self.text[..self.cursor].char_indices().next_back().map(|(at, _)| at).unwrap_or(0);
        self.text.drain(start..self.cursor);
        self.cursor = start;
        self.changed();
    }

    fn delete(&mut self) {
        if let Some((start, end)) = self.selection.take() {
            self.text.drain(start..end);
            self.cursor = start;
            self.changed();
            return;
        }
        if self.cursor == self.text.len() { return; }
        let end = self.cursor + self.text[self.cursor..].chars().next().map(char::len_utf8).unwrap_or(0);
        self.text.drain(self.cursor..end);
        self.changed();
    }

    fn changed(&mut self) { self.selection = None; self.dirty = true; self.status = String::from("Modified"); }

    fn move_vertical(&mut self, down: bool) {
        let before = &self.text[..self.cursor];
        let line_start = before.rfind('\n').map(|at| at + 1).unwrap_or(0);
        let column = before[line_start..].chars().count();
        if down {
            if let Some(next_rel) = self.text[self.cursor..].find('\n') {
                let next_start = self.cursor + next_rel + 1;
                let next_end = self.text[next_start..].find('\n').map(|n| next_start + n).unwrap_or(self.text.len());
                self.cursor = char_boundary_after_chars(&self.text, next_start, next_end, column);
            }
        } else if line_start > 0 {
            let prev_end = line_start - 1;
            let prev_start = self.text[..prev_end].rfind('\n').map(|at| at + 1).unwrap_or(0);
            self.cursor = char_boundary_after_chars(&self.text, prev_start, prev_end, column);
        }
    }

    fn load(&mut self, path: &str) -> Result<(), String> {
        match eonos_app::system::read_file(path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => {
                    self.text = text;
                    self.cursor = 0;
                    self.path = Some(String::from(path));
                    self.dirty = false;
                    self.first_line = 0;
                    self.selection = None;
                    self.status = format!("Opened {}", path);
                    Ok(())
                }
                Err(_) => Err(String::from("This file is not valid UTF-8 text.")),
            },
            Err(error) => Err(format!("Could not open {}: {:?}", path, error)),
        }
    }
}

fn find_match(doc: &mut Document, query: &str, backwards: bool) -> bool {
    if query.is_empty() { doc.status = String::from("Enter text to find"); return false; }
    let start = doc.selection.map(|(start, _)| start).unwrap_or(doc.cursor);
    let found = if backwards {
        doc.text[..start].match_indices(query).last().map(|(at, _)| at)
            .or_else(|| doc.text.match_indices(query).last().map(|(at, _)| at))
    } else {
        let from = doc.selection.map(|(_, end)| end).unwrap_or(doc.cursor);
        doc.text[from..].find(query).map(|at| from + at)
            .or_else(|| doc.text[..from].find(query))
    };
    if let Some(at) = found {
        let end = at + query.len();
        doc.cursor = at;
        doc.selection = Some((at, end));
        let line = doc.text[..at].bytes().filter(|&byte| byte == b'\n').count() + 1;
        doc.status = format!("Found at line {}", line);
        doc.clamp_scroll();
        true
    } else {
        doc.selection = None;
        doc.status = format!("No match for '{}'", query.chars().take(48).collect::<String>());
        false
    }
}

fn replace_one(doc: &mut Document, query: &str, replacement: &str) -> bool {
    if query.is_empty() { doc.status = String::from("Enter text to find"); return false; }
    let range = doc.selection.filter(|(start, end)| doc.text.get(*start..*end) == Some(query))
        .or_else(|| {
            let from = doc.cursor.min(doc.text.len());
            doc.text[from..].find(query).map(|offset| (from + offset, from + offset + query.len()))
                .or_else(|| doc.text[..from].find(query).map(|offset| (offset, offset + query.len())))
        });
    let Some((start, end)) = range else { doc.status = format!("No match for '{}'", query.chars().take(48).collect::<String>()); return false; };
    let Some(new_len) = doc.text.len().checked_sub(end - start).and_then(|len| len.checked_add(replacement.len())) else {
        doc.status = String::from("Replacement is too large"); return false;
    };
    if new_len > MAX_DOCUMENT { doc.status = String::from("Document limit reached (1 MiB)"); return false; }
    doc.text.replace_range(start..end, replacement);
    doc.cursor = start + replacement.len();
    doc.changed();
    doc.status = String::from("Replaced one match");
    true
}

fn replace_all(doc: &mut Document, query: &str, replacement: &str) {
    if query.is_empty() { doc.status = String::from("Enter text to find"); return; }
    let count = doc.text.match_indices(query).count();
    if count == 0 { doc.status = format!("No match for '{}'", query.chars().take(48).collect::<String>()); return; }
    let Some(new_len) = count.checked_mul(query.len()).and_then(|removed| doc.text.len().checked_sub(removed))
        .and_then(|len| count.checked_mul(replacement.len()).and_then(|added| len.checked_add(added))) else {
        doc.status = String::from("Replacement is too large"); return;
    };
    if new_len > MAX_DOCUMENT { doc.status = String::from("Document limit reached (1 MiB)"); return; }
    doc.text = doc.text.replace(query, replacement);
    doc.cursor = 0;
    doc.changed();
    doc.status = format!("Replaced {} matches", count);
}

fn char_boundary_after_chars(text: &str, start: usize, end: usize, count: usize) -> usize {
    text[start..end].char_indices().nth(count).map(|(offset, _)| start + offset).unwrap_or(end)
}

fn editor_cursor_at(doc: &Document, x: i32, y: i32) -> Option<usize> {
    if x < EDITOR_X || x >= EDITOR_X + EDITOR_W as i32 || y < 98 || y >= 442 { return None; }
    let line_number = doc.first_line + ((y - 98) / LINE_HEIGHT) as usize;
    let mut line_start = 0;
    for _ in 0..line_number {
        let newline = doc.text[line_start..].find('\n')?;
        line_start += newline + 1;
    }
    let line_end = doc.text[line_start..].find('\n').map(|offset| line_start + offset).unwrap_or(doc.text.len());
    let cursor_column = if doc.cursor_line() == line_number {
        Some(doc.text[line_start..doc.cursor.min(line_end)].chars().count())
    } else { None };
    let start_column = cursor_column.map(|column| column.saturating_sub(CHARS_PER_LINE.saturating_sub(1))).unwrap_or(0);
    let clicked_column = ((x - EDITOR_X - 8).max(0) / 8) as usize;
    let target_column = start_column.saturating_add(clicked_column).min(doc.text[line_start..line_end].chars().count());
    Some(line_start + doc.text[line_start..line_end].char_indices().nth(target_column).map(|(offset, _)| offset).unwrap_or(line_end - line_start))
}

fn draw_line(window: &mut eonos_app::era::Window, line: &str, y: i32, cursor_column: Option<usize>, selection: Option<(usize, usize)>) {
    let start_column = cursor_column
        .map(|column| column.saturating_sub(CHARS_PER_LINE.saturating_sub(1)))
        .or_else(|| selection.map(|(start, _)| start.saturating_sub(CHARS_PER_LINE.saturating_sub(1))))
        .unwrap_or(0);
    let visible: String = line.chars().skip(start_column).take(CHARS_PER_LINE).collect();
    if let Some((sel_start, sel_end)) = selection {
        let visible_end = start_column + visible.chars().count();
        let left = sel_start.max(start_column).min(visible_end);
        let right = sel_end.max(left).min(visible_end);
        let before: String = visible.chars().take(left.saturating_sub(start_column)).collect();
        let selected: String = visible.chars().skip(left.saturating_sub(start_column)).take(right.saturating_sub(left)).collect();
        let after: String = visible.chars().skip(right.saturating_sub(start_column)).collect();
        let base_x = EDITOR_X + 8;
        let _ = window.text(base_x, y, 0x202020, &before);
        if !selected.is_empty() {
            let x = base_x + before.chars().count() as i32 * 8;
            let _ = window.fill_rect(x, y, selected.chars().count() as u32 * 8, 16, 0x000080);
            let _ = window.text(x, y, 0xFFFFFF, &selected);
        }
        let suffix_x = base_x + (before.chars().count() + selected.chars().count()) as i32 * 8;
        let _ = window.text(suffix_x, y, 0x202020, &after);
    } else {
        let _ = window.text(EDITOR_X + 8, y, 0x202020, &visible);
    }
    if let Some(column) = cursor_column {
        let x_column = column.saturating_sub(start_column).min(CHARS_PER_LINE);
        let _ = window.fill_rect(EDITOR_X + 8 + x_column as i32 * 8, y, 1, 16, 0x000080);
    }
}

fn key_char(usage: u8, shift: bool) -> Option<char> {
    let c = match usage {
        0x04..=0x1d => (if shift { b'A' } else { b'a' } + usage - 0x04) as char,
        0x1e..=0x26 => (if shift { b'!' + usage - 0x1e } else { b'1' + usage - 0x1e }) as char,
        0x27 => if shift { ')' } else { '0' },
        0x2c => ' ', 0x2d => if shift { '_' } else { '-' },
        0x2e => if shift { '+' } else { '=' }, 0x2f => if shift { '{' } else { '[' },
        0x30 => if shift { '}' } else { ']' }, 0x31 => if shift { '|' } else { '\\' },
        0x33 => if shift { ':' } else { ';' }, 0x34 => if shift { '"' } else { '\'' },
        0x35 => if shift { '~' } else { '`' }, 0x36 => if shift { '<' } else { ',' },
        0x37 => if shift { '>' } else { '.' }, 0x38 => if shift { '?' } else { '/' },
        _ => return None,
    };
    Some(c)
}

fn save(window: &mut eonos_app::era::Window, doc: &mut Document) {
    let path = if let Some(path) = doc.path.clone() { Some(path) }
    else {
        match window.save_file_dialog("/disk", "untitled.txt") {
            Ok(path) => path,
            Err(_) => { let _ = window.error_dialog("The Save dialog could not be opened."); return; }
        }
    };
    let Some(path) = path else { return; };
    match eonos_app::system::write_file(&path, doc.text.as_bytes()) {
        Ok(()) => {
            doc.path = Some(path.clone());
            doc.dirty = false;
            doc.status = format!("Saved {}", path);
            let _ = window.toast("File saved");
        }
        Err(error) => {
            doc.status = String::from("Save failed");
            let _ = window.error_dialog(&format!("Could not save the file: {:?}", error));
        }
    }
}

fn confirm_discard(window: &mut eonos_app::era::Window, doc: &Document) -> bool {
    !doc.dirty || window.confirm_dialog("Unsaved changes", "Discard your unsaved changes?").unwrap_or(false)
}

fn ask_find(window: &mut eonos_app::era::Window, doc: &mut Document, query: &mut String, backwards: bool) {
    match window.text_input_dialog("Find", "Find text:", query) {
        Ok(Some(value)) => {
            if value.is_empty() { doc.status = String::from("Enter text to find"); }
            else { *query = value; find_match(doc, query, backwards); }
        }
        Ok(None) => {}
        Err(_) => { let _ = window.error_dialog("The Find dialog could not be opened."); }
    }
}

fn ask_replace(window: &mut eonos_app::era::Window, doc: &mut Document, query: &mut String, all: bool) {
    let search = match window.text_input_dialog("Replace", "Find text:", query) {
        Ok(Some(value)) if !value.is_empty() => value,
        Ok(Some(_)) => { doc.status = String::from("Enter text to find"); return; }
        Ok(None) => return,
        Err(_) => { let _ = window.error_dialog("The Replace dialog could not be opened."); return; }
    };
    let replacement = match window.text_input_dialog("Replace", "Replace with:", "") {
        Ok(Some(value)) => value,
        Ok(None) => return,
        Err(_) => { let _ = window.error_dialog("The Replace dialog could not be opened."); return; }
    };
    *query = search.clone();
    if all { replace_all(doc, &search, &replacement); }
    else { replace_one(doc, &search, &replacement); }
}

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("Text Editor", 760, 500) else { return 1; };
    let mut doc = Document::new();
    let mut find_query = String::new();
    let mut quit = false;
    let mut pointer_left = false;
    let mut selection_anchor = None;
    while !quit {
        while let Ok(Some(event)) = window.poll_event() {
            use eonos_app::era::Event;
            match event {
                Event::Close => { if confirm_discard(&mut window, &doc) { quit = true; } }
                Event::Pointer { x, y, buttons } => {
                    let down = buttons & 1 != 0;
                    if down {
                        if let Some(position) = editor_cursor_at(&doc, x, y) {
                            if !pointer_left {
                                selection_anchor = Some(position);
                                doc.selection = None;
                            }
                            doc.cursor = position;
                            if let Some(anchor) = selection_anchor {
                                if anchor != position { doc.selection = Some((anchor.min(position), anchor.max(position))); }
                            }
                        }
                    } else {
                        selection_anchor = None;
                    }
                    pointer_left = down;
                }
                Event::Key { usage, pressed: true, modifiers } => {
                    let ctrl = modifiers & 0x11 != 0;
                    let shift = modifiers & 0x22 != 0;
                    if ctrl && (usage == 0x16) { save(&mut window, &mut doc); continue; }
                    if ctrl && usage == 0x09 { ask_find(&mut window, &mut doc, &mut find_query, shift); continue; }
                    if ctrl && usage == 0x0b { ask_replace(&mut window, &mut doc, &mut find_query, shift); continue; }
                    match usage {
                        0x29 => { if confirm_discard(&mut window, &doc) { quit = true; } }
                        0x2a => doc.backspace(),
                        0x4c => doc.delete(),
                        0x50 if doc.cursor > 0 => { doc.selection = None; doc.cursor = doc.text[..doc.cursor].char_indices().next_back().map(|(at, _)| at).unwrap_or(0); }
                        0x4f if doc.cursor < doc.text.len() => { doc.selection = None; doc.cursor += doc.text[doc.cursor..].chars().next().map(char::len_utf8).unwrap_or(0); }
                        0x52 => { doc.selection = None; doc.move_vertical(false); }
                        0x51 => { doc.selection = None; doc.move_vertical(true); }
                        0x4a => { doc.selection = None; doc.cursor = doc.text[..doc.cursor].rfind('\n').map(|at| at + 1).unwrap_or(0); }
                        0x4d => { doc.selection = None; doc.cursor = doc.text[doc.cursor..].find('\n').map(|at| doc.cursor + at).unwrap_or(doc.text.len()); }
                        0x28 => doc.insert('\n'),
                        0x2b => doc.insert('\t'),
                        _ => if let Some(ch) = key_char(usage, shift) { doc.insert(ch); }
                    }
                }
                _ => {}
            }
        }
        doc.clamp_scroll();

        let _ = window.fill_rect(0, 0, 760, 500, 0xD4D0C8);
        let _ = window.fill_rect(10, 10, 740, 480, 0xFFFFFF);
        let _ = window.text(22, 18, 0x000080, "Text Editor");
        if window.button(22, 42, 70, 28, "New").unwrap_or(false) && confirm_discard(&mut window, &doc) {
            doc = Document::new();
        }
        if window.button(100, 42, 70, 28, "Open").unwrap_or(false) && confirm_discard(&mut window, &doc) {
            match window.text_input_dialog("Open file", "Path under /disk:", "/disk/") {
                Ok(Some(path)) => if let Err(message) = doc.load(&path) { let _ = window.error_dialog(&message); doc.status = message; },
                Ok(None) => {},
                Err(_) => { let _ = window.error_dialog("The Open dialog could not be opened."); }
            }
        }
        if window.button(178, 42, 70, 28, "Save").unwrap_or(false) { save(&mut window, &mut doc); }
        if window.button(256, 42, 90, 28, "Save As").unwrap_or(false) {
            let suggested = doc.path.as_deref().and_then(|p| p.rsplit('/').next()).unwrap_or("untitled.txt").to_string();
            match window.save_file_dialog("/disk", &suggested) {
                Ok(Some(path)) => {
                    let old_path = doc.path.replace(path);
                    save(&mut window, &mut doc);
                    if doc.dirty { doc.path = old_path; }
                }
                Ok(None) => {},
                Err(_) => { let _ = window.error_dialog("The Save dialog could not be opened."); }
            }
        }
        if window.button(354, 42, 65, 28, "Find").unwrap_or(false) { ask_find(&mut window, &mut doc, &mut find_query, false); }
        if window.button(427, 42, 76, 28, "Replace").unwrap_or(false) { ask_replace(&mut window, &mut doc, &mut find_query, false); }
        if window.button(511, 42, 92, 28, "Replace All").unwrap_or(false) { ask_replace(&mut window, &mut doc, &mut find_query, true); }
        if window.button(611, 42, 62, 28, "Prev").unwrap_or(false) { find_match(&mut doc, &find_query, true); }
        if window.button(681, 42, 62, 28, "Next").unwrap_or(false) { find_match(&mut doc, &find_query, false); }
        let filename = doc.path.as_deref().unwrap_or("Untitled");
        let title = if doc.dirty { format!("{} *", filename) } else { String::from(filename) };
        let _ = window.text(22, 76, 0x404040, &title.chars().take(88).collect::<String>());
        let _ = window.fill_rect(EDITOR_X, 98, EDITOR_W, 344, 0xFFFFF8);
        let _ = window.fill_rect(EDITOR_X, 98, EDITOR_W, 1, 0x404040);
        let _ = window.fill_rect(EDITOR_X, 98, 1, 344, 0x404040);
        let mut line_start = 0usize;
        let mut line_number = 0usize;
        for (index, byte) in doc.text.bytes().enumerate() {
            if byte == b'\n' {
                if line_number >= doc.first_line {
                    let row = line_number - doc.first_line;
                    let line = &doc.text[line_start..index];
                    let cursor_column = if line_number == doc.cursor_line() {
                        Some(doc.text[line_start..doc.cursor.min(index)].chars().count())
                    } else { None };
                    let line_selection = doc.selection.and_then(|(start, end)| {
                        let low = start.max(line_start);
                        let high = end.min(index);
                        (low < high).then(|| (line[..low - line_start].chars().count(), line[..high - line_start].chars().count()))
                    });
                    draw_line(&mut window, line, 106 + row as i32 * LINE_HEIGHT, cursor_column, line_selection);
                }
                line_number += 1;
                line_start = index + 1;
            }
        }
        if line_number >= doc.first_line {
            let row = line_number - doc.first_line;
            let line = &doc.text[line_start..];
            let cursor_column = if line_number == doc.cursor_line() {
                Some(doc.text[line_start..doc.cursor].chars().count())
            } else { None };
            let line_selection = doc.selection.and_then(|(start, end)| {
                let low = start.max(line_start);
                let high = end.min(doc.text.len());
                (low < high).then(|| (line[..low - line_start].chars().count(), line[..high - line_start].chars().count()))
            });
            draw_line(&mut window, line, 106 + row as i32 * LINE_HEIGHT, cursor_column, line_selection);
        }
        let _ = window.fill_rect(EDITOR_X, 452, EDITOR_W, 1, 0xA0A0A0);
        let _ = window.text(22, 460, 0x505050, &doc.status.chars().take(76).collect::<String>());
        let _ = window.present();
        let _ = eonos_app::system::sleep_ms(16);
    }
    let _ = window.close();
    0
}

eonos_app::app!(ERA, main);
