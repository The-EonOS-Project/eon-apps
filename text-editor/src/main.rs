#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};

const MAX_DOCUMENT: usize = 1024 * 1024;
const EDITOR_X: i32 = 20;
const EDITOR_Y: i32 = 82;
const EDITOR_W: u32 = 720;
const EDITOR_H: u32 = 360;
const LINE_HEIGHT: i32 = 18;
const CHARS_PER_LINE: usize = 86;

struct Document {
    text: String,
    cursor: usize,
    path: Option<String>,
    dirty: bool,
    first_line: usize,
    status: String,
}

impl Document {
    fn new() -> Self {
        Self { text: String::new(), cursor: 0, path: None, dirty: false, first_line: 0, status: String::from("Ready") }
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
        if self.text.len().saturating_add(ch.len_utf8()) > MAX_DOCUMENT { self.status = String::from("Document limit reached (1 MiB)"); return; }
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.dirty = true;
        self.status = String::from("Modified");
    }

    fn backspace(&mut self) {
        if self.cursor == 0 { return; }
        let start = self.text[..self.cursor].char_indices().next_back().map(|(at, _)| at).unwrap_or(0);
        self.text.drain(start..self.cursor);
        self.cursor = start;
        self.changed();
    }

    fn delete(&mut self) {
        if self.cursor == self.text.len() { return; }
        let end = self.cursor + self.text[self.cursor..].chars().next().map(char::len_utf8).unwrap_or(0);
        self.text.drain(self.cursor..end);
        self.changed();
    }

    fn changed(&mut self) { self.dirty = true; self.status = String::from("Modified"); }

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
                    self.status = format!("Opened {}", path);
                    Ok(())
                }
                Err(_) => Err(String::from("This file is not valid UTF-8 text.")),
            },
            Err(error) => Err(format!("Could not open {}: {:?}", path, error)),
        }
    }
}

fn char_boundary_after_chars(text: &str, start: usize, end: usize, count: usize) -> usize {
    text[start..end].char_indices().nth(count).map(|(offset, _)| start + offset).unwrap_or(end)
}

fn draw_line(window: &mut eonos_app::era::Window, line: &str, y: i32, cursor_column: Option<usize>) {
    let start_column = cursor_column
        .map(|column| column.saturating_sub(CHARS_PER_LINE.saturating_sub(1)))
        .unwrap_or(0);
    let visible: String = line.chars().skip(start_column).take(CHARS_PER_LINE).collect();
    let _ = window.text(EDITOR_X + 8, y, 0x202020, &visible);
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

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("Text Editor", 760, 500) else { return 1; };
    let mut doc = Document::new();
    let mut quit = false;
    while !quit {
        while let Ok(Some(event)) = window.poll_event() {
            use eonos_app::era::Event;
            match event {
                Event::Close => { if confirm_discard(&mut window, &doc) { quit = true; } }
                Event::Key { usage, pressed: true, modifiers } => {
                    let ctrl = modifiers & 0x11 != 0;
                    let shift = modifiers & 0x22 != 0;
                    if ctrl && (usage == 0x16) { save(&mut window, &mut doc); continue; }
                    match usage {
                        0x29 => { if confirm_discard(&mut window, &doc) { quit = true; } }
                        0x2a => doc.backspace(),
                        0x4c => doc.delete(),
                        0x50 if doc.cursor > 0 => { doc.cursor = doc.text[..doc.cursor].char_indices().next_back().map(|(at, _)| at).unwrap_or(0); }
                        0x4f if doc.cursor < doc.text.len() => { doc.cursor += doc.text[doc.cursor..].chars().next().map(char::len_utf8).unwrap_or(0); }
                        0x52 => doc.move_vertical(false),
                        0x51 => doc.move_vertical(true),
                        0x4a => { doc.cursor = doc.text[..doc.cursor].rfind('\n').map(|at| at + 1).unwrap_or(0); }
                        0x4d => { doc.cursor = doc.text[doc.cursor..].find('\n').map(|at| doc.cursor + at).unwrap_or(doc.text.len()); }
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
        if window.button(22, 42, 72, 28, "New").unwrap_or(false) && confirm_discard(&mut window, &doc) {
            doc = Document::new();
        }
        if window.button(102, 42, 72, 28, "Open").unwrap_or(false) && confirm_discard(&mut window, &doc) {
            match window.text_input_dialog("Open file", "Path under /disk:", "/disk/") {
                Ok(Some(path)) => if let Err(message) = doc.load(&path) { let _ = window.error_dialog(&message); doc.status = message; },
                Ok(None) => {},
                Err(_) => { let _ = window.error_dialog("The Open dialog could not be opened."); }
            }
        }
        if window.button(182, 42, 72, 28, "Save").unwrap_or(false) { save(&mut window, &mut doc); }
        if window.button(262, 42, 92, 28, "Save As").unwrap_or(false) {
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
        let filename = doc.path.as_deref().unwrap_or("Untitled");
        let title = if doc.dirty { format!("{} *", filename) } else { String::from(filename) };
        let _ = window.text(370, 48, 0x404040, &title.chars().take(42).collect::<String>());
        let _ = window.fill_rect(EDITOR_X, EDITOR_Y, EDITOR_W, EDITOR_H, 0xFFFFF8);
        let _ = window.fill_rect(EDITOR_X, EDITOR_Y, EDITOR_W, 1, 0x404040);
        let _ = window.fill_rect(EDITOR_X, EDITOR_Y, 1, EDITOR_H, 0x404040);
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
                    draw_line(&mut window, line, EDITOR_Y + 8 + row as i32 * LINE_HEIGHT, cursor_column);
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
            draw_line(&mut window, line, EDITOR_Y + 8 + row as i32 * LINE_HEIGHT, cursor_column);
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
