#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

struct Report {
    lines: Vec<String>,
    error: Option<String>,
}

fn load_report() -> Report {
    match eonos_app::system::read_file("/disk/crashlog.txt") {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            let lines = text
                .lines()
                .take(256)
                .map(|line| line.chars().take(160).collect())
                .collect();
            Report { lines, error: None }
        }
        Err(error) => Report {
            lines: Vec::new(),
            error: Some(format!("Could not read /disk/crashlog.txt: {:?}", error)),
        },
    }
}

fn value<'a>(lines: &'a [String], prefix: &str) -> &'a str {
    lines
        .iter()
        .find_map(|line| line.strip_prefix(prefix))
        .unwrap_or("not recorded")
}

fn short(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("Crash Report Analyzer", 680, 470) else {
        return 1;
    };
    let mut report = load_report();
    let mut offset = 0usize;

    loop {
        while let Ok(Some(event)) = window.poll_event() {
            match event {
                eonos_app::era::Event::Close => {
                    let _ = window.close();
                    return 0;
                }
                eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. } => {
                    let _ = window.close();
                    return 0;
                }
                eonos_app::era::Event::Key { usage: 0x52, pressed: true, .. } => {
                    offset = offset.saturating_sub(1);
                }
                eonos_app::era::Event::Key { usage: 0x51, pressed: true, .. } => {
                    offset = (offset + 1).min(report.lines.len().saturating_sub(10));
                }
                _ => {}
            }
        }

        let _ = window.fill_rect(0, 0, 680, 470, 0xD4D0C8);
        let _ = window.fill_rect(12, 12, 656, 446, 0xFFFFFF);
        let _ = window.text(28, 28, 0x000080, "EonOS Crash Report Analyzer");
        let _ = window.text(28, 54, 0x404040, "Report: /disk/crashlog.txt");

        if let Some(error) = &report.error {
            let _ = window.text(28, 100, 0xA00000, &short(error, 76));
            let _ = window.text(28, 128, 0x404040, "Mount a writable disk at /disk before a crash to save reports.");
        } else {
            let cause = report
                .lines
                .iter()
                .find(|line| line.starts_with("*** KERNEL PANIC :"))
                .map(String::as_str)
                .unwrap_or("Cause: not recorded");
            let process = report
                .lines
                .iter()
                .find(|line| line.starts_with("Process:") || line.starts_with("Kernel thread:"))
                .map(String::as_str)
                .unwrap_or("Execution context: not recorded");
            let fault = report
                .lines
                .iter()
                .find(|line| line.starts_with("Faulting address:"))
                .map(String::as_str)
                .unwrap_or("Faulting address: not recorded");
            let page_fault = report
                .lines
                .iter()
                .find(|line| line.starts_with("Page fault:"))
                .map(String::as_str)
                .unwrap_or("Page fault: not applicable or not recorded");

            let _ = window.text(28, 86, 0xA00000, &short(cause, 76));
            let _ = window.text(28, 110, 0x202020, &format!("Error code:{}", value(&report.lines, "ERROR CODE:")));
            let _ = window.text(28, 134, 0x202020, &short(process, 76));
            let _ = window.text(28, 158, 0x202020, &short(fault, 76));
            let _ = window.text(28, 182, 0x202020, &short(page_fault, 76));
            let _ = window.text(28, 210, 0x000080, "Saved report details (Up/Down to scroll):");
            for (index, line) in report.lines.iter().skip(offset).take(9).enumerate() {
                let _ = window.text(32, 234 + index as i32 * 18, 0x303030, &short(line, 76));
            }
        }

        if window.button(28, 412, 120, 32, "Reload") .unwrap_or(false) {
            report = load_report();
            offset = 0;
        }
        if window.button(160, 412, 120, 32, "Close").unwrap_or(false) {
            let _ = window.close();
            return 0;
        }
        let _ = window.present();
        let _ = eonos_app::system::sleep_ms(16);
    }
}

eonos_app::app!(ERA, main);
