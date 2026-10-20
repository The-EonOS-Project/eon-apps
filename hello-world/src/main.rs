#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("Hello World!", 400, 180) else { return 1; };
    loop {
        let _ = window.fill_rect(0, 0, 400, 180, 0xD4D0C8);
        let _ = window.fill_rect(16, 16, 368, 148, 0xFFFFFF);
        let _ = window.text(32, 40, 0x000080, "Hello World!");
        let _ = window.text(32, 72, 0x202020, "My first EonOS app is running.");
        let _ = window.text(32, 112, 0x505050, "Press Escape or close this window to exit.");
        let _ = window.present();
        match window.poll_event() {
            Ok(Some(eonos_app::era::Event::Close)) => break,
            Ok(Some(eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. })) => break,
            _ => { let _ = eonos_app::system::sleep_ms(16); }
        }
    }
    let _ = window.close();
    0
}

eonos_app::app!(ERA, main);
