#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("Calculator", 360, 470) else { return 1; };
    let mut display = String::from("0");
    let mut accumulator = None;
    let mut pending_operator = None;
    let mut replace_display = false;
    let mut needs_operand = false;
    let mut precision = 4;

    loop {
        let mut close = false;
        let mut enter = false;
        let mut keyboard_actions = Vec::new();
        while let Ok(Some(event)) = window.poll_event() {
            match event {
                eonos_app::era::Event::Close => close = true,
                eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. } => close = true,
                eonos_app::era::Event::Key { usage: 0x28, pressed: true, .. } => enter = true,
                eonos_app::era::Event::Key { usage, pressed: true, modifiers } => {
                    if let Some(action) = key_action(usage, modifiers) {
                        if keyboard_actions.len() < 16 { keyboard_actions.push(action); }
                    }
                }
                _ => {}
            }
        }
        if close { break; }

        let _ = window.fill_rect(0, 0, 360, 470, 0xD4D0C8);
        let _ = window.text(20, 24, 0x000080, "EonOS Calculator");
        let _ = window.fill_rect(20, 52, 320, 46, 0xFFFFFF);
        let visible: String = display.chars().rev().take(36).collect::<String>().chars().rev().collect();
        let _ = window.text(328 - visible.len() as i32 * 8, 66, 0x101010, &visible);

        let labels = [
            ["C", "DEL", "%", "/"],
            ["7", "8", "9", "*"],
            ["4", "5", "6", "-"],
            ["1", "2", "3", "+"],
            ["0", ".", "+/-", "="],
        ];
        let mut clicked = None;
        for (row, buttons) in labels.iter().enumerate() {
            for (column, label) in buttons.iter().enumerate() {
                let x = 24 + column as i32 * 78;
                let y = 116 + row as i32 * 52;
                if window.button(x, y, 70, 42, label).unwrap_or(false) {
                    clicked = Some(*label);
                }
            }
        }

        let _ = window.text(24, 390, 0x202020, "Precision");
        let _ = window.slider(104, 384, 220, 30, 0, 8, &mut precision);
        let _ = window.text(24, 418, 0x505050, "Type a number, then choose an operation.");
        let _ = window.present();

        let mut actions = keyboard_actions;
        if enter { actions.push("="); }
        else if let Some(action) = clicked { actions.push(action); }
        for key in actions {
            match key {
                "C" => {
                    display = String::from("0");
                    accumulator = None;
                    pending_operator = None;
                    replace_display = false;
                    needs_operand = false;
                }
                "DEL" => {
                    if !replace_display {
                        display.pop();
                        if display.is_empty() || display == "-" { display = String::from("0"); }
                    }
                }
                "." => {
                    if replace_display { display = String::from("0"); replace_display = false; }
                    if display.is_empty() { display = String::from("0"); }
                    if !display.contains('.') { display.push('.'); }
                    needs_operand = false;
                }
                "+/-" => {
                    if display.starts_with('-') { display.remove(0); }
                    else if display != "0" { display.insert(0, '-'); }
                    replace_display = false;
                }
                "%" => {
                    if let Ok(value) = display.parse::<f64>() {
                        display = format_number(value / 100.0, precision as usize);
                        replace_display = true;
                    }
                }
                "+" | "-" | "*" | "/" => {
                    if needs_operand {
                        pending_operator = Some(key.chars().next().unwrap_or('+'));
                    } else if let Ok(value) = display.parse::<f64>() {
                        if let (Some(left), Some(operator)) = (accumulator, pending_operator) {
                            if let Some(result) = calculate(left, value, operator) {
                                display = format_number(result, precision as usize);
                                accumulator = Some(result);
                            } else {
                                display = String::from("Error");
                                accumulator = None;
                            }
                        } else {
                            accumulator = Some(value);
                        }
                        pending_operator = Some(key.chars().next().unwrap_or('+'));
                    }
                    if pending_operator.is_some() {
                        display.clear();
                        replace_display = false;
                        needs_operand = true;
                    }
                }
                "=" => {
                    if let (Some(left), Some(operator)) = (accumulator, pending_operator) {
                        let right = if needs_operand { Some(left) } else { display.parse::<f64>().ok() };
                        if let Some(right) = right {
                            if let Some(result) = calculate(left, right, operator) {
                                display = format_number(result, precision as usize);
                            } else {
                                display = String::from("Cannot divide by zero");
                            }
                        }
                        accumulator = None;
                        pending_operator = None;
                        replace_display = true;
                        needs_operand = false;
                    }
                }
                digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => {
                    if replace_display || display == "0" || display.is_empty() || display == "Error" || display.starts_with("Cannot") {
                        display.clear();
                        replace_display = false;
                    }
                    if display.len() < 24 { display.push(digit.as_bytes()[0] as char); }
                    needs_operand = false;
                }
                _ => {}
            }
        }
        let _ = eonos_app::system::sleep_ms(16);
    }

    let _ = window.close();
    0
}

fn calculate(left: f64, right: f64, operator: char) -> Option<f64> {
    let result = match operator {
        '+' => left + right,
        '-' => left - right,
        '*' => left * right,
        '/' if right != 0.0 => left / right,
        _ => return None,
    };
    result.is_finite().then_some(result)
}

fn key_action(usage: u8, modifiers: u8) -> Option<&'static str> {
    let shifted = modifiers & 0x22 != 0;
    Some(match usage {
        0x1e => "1",
        0x1f => "2",
        0x20 => "3",
        0x21 => "4",
        0x22 if shifted => "%",
        0x22 => "5",
        0x23 => "6",
        0x24 => "7",
        0x25 if shifted => "*",
        0x25 => "8",
        0x26 => "9",
        0x27 => "0",
        0x2a | 0x4c => "DEL",
        0x2d | 0x56 => "-",
        0x57 => "+",
        0x2e if shifted => "+",
        0x2e => "=",
        0x37 => ".",
        0x38 | 0x54 => "/",
        0x55 => "*",
        0x06 => "C",
        _ => return None,
    })
}

fn format_number(value: f64, precision: usize) -> String {
    if !value.is_finite() { return String::from("Error"); }
    let mut output = format!("{value:.precision$}");
    if output.contains('.') {
        while output.ends_with('0') { output.pop(); }
        if output.ends_with('.') { output.pop(); }
    }
    if output == "-0" || output.is_empty() { output = String::from("0"); }
    if output.len() > 24 { return String::from("Overflow"); }
    output
}

eonos_app::app!(ERA, main);
