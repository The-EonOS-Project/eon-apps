#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::format;

const FOCUS_MINUTES: [u64; 3] = [15, 25, 45];
const BREAK_MS: u64 = 5 * 60 * 1000;

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("Pomodoro Timer", 500, 330) else {
        return 1;
    };
    let choices = ["15 minutes", "25 minutes", "45 minutes"];
    let mut selected = 1usize;
    let mut focus_phase = true;
    let mut remaining_ms = FOCUS_MINUTES[selected] * 60 * 1000;
    let mut deadline = None;
    let mut message = "Choose a focus length, then start the timer.";

    loop {
        while let Ok(Some(event)) = window.poll_event() {
            if matches!(event, eonos_app::era::Event::Close)
                || matches!(event, eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. })
            {
                let _ = window.close();
                return 0;
            }
        }

        let now = eonos_app::system::uptime_ms();
        if let Some(end) = deadline {
            if now >= end {
                deadline = None;
                focus_phase = !focus_phase;
                remaining_ms = if focus_phase { FOCUS_MINUTES[selected] * 60 * 1000 } else { BREAK_MS };
                message = if focus_phase {
                    "Break complete. Ready for another focus session."
                } else {
                    "Focus complete. Start your five-minute break."
                };
            }
        }
        let display_ms = deadline.map_or(remaining_ms, |end| end.saturating_sub(now));
        let seconds = display_ms.saturating_add(999) / 1000;
        let clock = format!("{:02}:{:02}", seconds / 60, seconds % 60);

        let _ = window.fill_rect(0, 0, 500, 330, 0xD4D0C8);
        let _ = window.fill_rect(14, 14, 472, 302, 0xFFFFFF);
        let _ = window.text(32, 30, 0x000080, "EonOS Pomodoro Timer");
        let phase = if focus_phase { "FOCUS SESSION" } else { "BREAK" };
        let _ = window.text(32, 72, 0x404040, phase);
        let _ = window.text(32, 104, 0x000000, &clock);
        let _ = window.text(32, 144, 0x404040, message);

        let running = deadline.is_some();
        let options_changed = window.dropdown(32, 194, 180, 30, &choices, &mut selected).unwrap_or(false);
        if options_changed && deadline.is_none() && focus_phase {
            remaining_ms = FOCUS_MINUTES[selected] * 60 * 1000;
        }
        let toggle_label = if running { "Pause" } else if focus_phase { "Start focus" } else { "Start break" };
        if window.button(238, 190, 112, 38, toggle_label).unwrap_or(false) {
            if let Some(end) = deadline.take() {
                remaining_ms = end.saturating_sub(now);
                message = "Paused. Start when you are ready.";
            } else {
                deadline = Some(now.saturating_add(remaining_ms));
                message = if focus_phase { "Stay focused until the timer ends." } else { "Take a short break." };
            }
        }
        if window.button(366, 190, 100, 38, "Reset").unwrap_or(false) {
            deadline = None;
            focus_phase = true;
            remaining_ms = FOCUS_MINUTES[selected] * 60 * 1000;
            message = "Timer reset. Ready when you are.";
        }
        let _ = window.text(32, 258, 0x606060, "The next phase waits for you to press Start.");
        let _ = window.present();
        let _ = eonos_app::system::sleep_ms(100);
    }
}

eonos_app::app!(ERA, main);
