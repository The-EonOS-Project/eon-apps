#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

const WIDTH: u32 = 760;
const HEIGHT: u32 = 560;
const VISIBLE_ROWS: usize = 20;

fn wrap_text(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next_width = line.chars().count() + usize::from(!line.is_empty()) + word.chars().count();
        if !line.is_empty() && next_width > 82 {
            lines.push(core::mem::take(&mut line));
            if lines.len() == 512 { return lines; }
        }
        if !line.is_empty() { line.push(' '); }
        line.push_str(word);
    }
    if !line.is_empty() && lines.len() < 512 { lines.push(line); }
    lines
}

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("EonOS Web Fetch Viewer", WIDTH, HEIGHT) else { return 1; };
    let mut url = String::from("https://example.com");
    let mut title = String::from("No page loaded");
    let mut status = String::from("Enter an HTTP(S) address and select Fetch.");
    let mut lines: Vec<String> = Vec::new();
    let mut scroll = 0usize;
    let mut close = false;

    while !close {
        while let Ok(Some(event)) = window.poll_event() {
            match event {
                eonos_app::era::Event::Close => close = true,
                eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. } => close = true,
                eonos_app::era::Event::Key { usage: 0x52, pressed: true, .. } => scroll = scroll.saturating_sub(1),
                eonos_app::era::Event::Key { usage: 0x51, pressed: true, .. } => {
                    scroll = (scroll + 1).min(lines.len().saturating_sub(VISIBLE_ROWS));
                }
                _ => {}
            }
        }
        if close { break; }

        let _ = window.fill_rect(0, 0, WIDTH, HEIGHT, 0xD4D0C8);
        let _ = window.text(18, 16, 0x000080, "EonOS Web Fetch Viewer");
        let _ = window.text(18, 42, 0x202020, "Address:");
        let _ = window.text_input(92, 34, 560, 30, &mut url, "https://example.com");
        let fetch_clicked = window.button(666, 34, 76, 30, "Fetch").unwrap_or(false);
        let _ = window.text(18, 82, 0x000080, &title);
        let _ = window.fill_rect(16, 108, 728, 398, 0xFFFFFF);
        let _ = window.text(24, 116, 0x505050, &status);
        for (row, line) in lines.iter().skip(scroll).take(VISIBLE_ROWS).enumerate() {
            let y = 144 + row as i32 * 17;
            let _ = window.text(24, y, 0x101010, line);
        }
        let _ = window.text(18, 520, 0x505050, "Up/Down scroll  |  Escape closes  |  HTTPS certificates are verified");
        let _ = window.present();

        if fetch_clicked {
            status = String::from("Fetching page...");
            let _ = window.fill_rect(16, 108, 728, 398, 0xFFFFFF);
            let _ = window.text(24, 116, 0x505050, &status);
            let _ = window.present();
            match eonos_app::web::fetch_html(&url) {
                Ok(page) => {
                    title = page.title;
                    lines = wrap_text(&page.text);
                    scroll = 0;
                    status = alloc::format!("Loaded {} text rows; page content is limited by the SDK.", lines.len());
                }
                Err(error) => {
                    title = String::from("Fetch failed");
                    lines.clear();
                    scroll = 0;
                    status = alloc::format!("Request or page parsing failed (error {}).", error.0);
                }
            }
        }
        let _ = eonos_app::system::sleep_ms(16);
    }
    let _ = window.close();
    0
}

eonos_app::app!(ERA, main);
