#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

struct Package {
    id: String,
    name: String,
    description: String,
    kind: String,
    path: String,
}

struct InstalledApp {
    id: String,
    name: String,
    kind: String,
    built_in: bool,
}

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open_auto("App Manager") else { return 1; };
    let mut packages = discover_packages();
    let mut installed = discover_installed();
    let mut show_installed = false;
    let mut page = 0usize;
    let mut status = String::from("Choose a package to install or update.");

    loop {
        let (width, height) = window.dimensions();
        let mut close = false;
        while let Ok(Some(event)) = window.poll_event() {
            match event {
                eonos_app::era::Event::Close
                | eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. } => close = true,
                _ => {}
            }
        }
        if close { break; }

        if window.fill_rect(0, 0, width, height, 0xD4D0C8).is_err()
            || window.fill_rect(14, 14, width.saturating_sub(28), height.saturating_sub(28), 0xFFFFFF).is_err()
        { break; }
        let _ = window.text(28, 24, 0x000080, "EonOS App Manager");
        let _ = window.text(28, 48, 0x404040, "Manage apps available on this system.");

        let tabs_y = 76;
        let installed_tab = window.button(28, tabs_y, 112, 30, "Installed").unwrap_or(false);
        let available_tab = window.button(148, tabs_y, 112, 30, "Available").unwrap_or(false);
        if installed_tab != available_tab {
            show_installed = installed_tab;
            page = 0;
        }
        let refresh_x = (width as i32 - 126).max(280);
        if window.button(refresh_x, tabs_y, 98, 30, "Refresh").unwrap_or(false) {
            packages = discover_packages();
            installed = discover_installed();
            page = 0;
            status = String::from("App lists refreshed.");
        }

        let row_top = 122;
        let row_height = 54;
        let page_size = ((height as i32 - row_top - 76) / row_height).clamp(1, 12) as usize;
        let item_count = if show_installed { installed.len() } else { packages.len() };
        let page_count = item_count.saturating_sub(1) / page_size + 1;
        page = page.min(page_count.saturating_sub(1));
        let start = page * page_size;
        let visible = item_count.saturating_sub(start).min(page_size);
        let row_width = width.saturating_sub(64);

        for row in 0..visible {
            let y = row_top + row as i32 * row_height;
            let x = 28;
            let _ = window.fill_rect(x, y, row_width, 48, 0xE8E8E8);
            let _ = window.fill_rect(x, y, row_width, 1, 0xFFFFFF);
            let item_index = start + row;
            if show_installed {
                let app = &installed[item_index];
                let _ = window.text(x + 10, y + 7, 0x000080, &truncate(&app.name, 36));
                let detail = format!("{}  |  {}", app.id, app.kind);
                let _ = window.text(x + 10, y + 27, 0x505050, &truncate(&detail, 48));
                if app.built_in {
                    let _ = window.text(width as i32 - 116, y + 16, 0x505050, "Built in");
                } else if window.button(width as i32 - 126, y + 10, 86, 28, "Remove").unwrap_or(false) {
                    if window.confirm_dialog("Remove app?", &format!("Remove {} from this system?", app.name)).unwrap_or(false) {
                        match eonos_app::system::uninstall_app(&app.id) {
                            Ok(output) => status = if output.is_empty() { format!("Removed {}.", app.name) } else { output },
                            Err(_) => status = format!("Could not remove {}.", app.name),
                        }
                        installed = discover_installed();
                        page = 0;
                    }
                }
            } else {
                let package = &packages[item_index];
                let _ = window.text(x + 10, y + 6, 0x000080, &truncate(&package.name, 34));
                let _ = window.text(x + 10, y + 24, 0x404040, &truncate(&package.description, 54));
                let detail = format!("{}  |  {}", package.id, package.kind);
                let _ = window.text(x + 10, y + 37, 0x606060, &truncate(&detail, 44));
                let already_installed = installed.iter().any(|app| app.id == package.id && !app.built_in);
                let label = if already_installed { "Update" } else { "Install" };
                if window.button(width as i32 - 126, y + 10, 86, 28, label).unwrap_or(false) {
                    let proceed = !already_installed || window.confirm_dialog("Update app?", &format!("Replace the installed {} app?", package.name)).unwrap_or(false);
                    if proceed {
                        let _ = window.text(x + 10, height as i32 - 36, 0x404040, "Installing app package...");
                        let _ = window.present();
                        let removed = if already_installed {
                            match eonos_app::system::uninstall_app(&package.id) {
                                Ok(_) => true,
                                Err(_) => { status = format!("Could not remove the old {} app.", package.name); false }
                            }
                        } else { true };
                        if removed {
                            match eonos_app::system::install_app_package(&package.path) {
                                Ok(output) => status = output.lines().next().filter(|line| !line.is_empty())
                                    .map(String::from).unwrap_or_else(|| format!("Installed {}.", package.name)),
                                Err(_) => status = format!("Could not install {}. Check that /disk is mounted.", package.name),
                            }
                        }
                        installed = discover_installed();
                        page = 0;
                    }
                }
            }
        }

        let footer_y = height as i32 - 62;
        let previous = window.button(28, footer_y, 80, 28, "Previous").unwrap_or(false);
        if previous { page = page.saturating_sub(1); }
        let next = window.button(116, footer_y, 80, 28, "Next").unwrap_or(false);
        if next { page = (page + 1).min(page_count.saturating_sub(1)); }
        let summary = if show_installed {
            format!("Installed: {} app(s)  |  Page {}/{}", item_count, page + 1, page_count)
        } else {
            format!("Packages: {}  |  Page {}/{}", item_count, page + 1, page_count)
        };
        let _ = window.text(210, footer_y + 7, 0x404040, &truncate(&summary, 50));
        let _ = window.text(28, height as i32 - 29, 0x505050, &truncate(&status, 76));
        if window.present().is_err() { break; }
        let _ = eonos_app::system::sleep_ms(16);
    }
    let _ = window.close();
    0
}

fn discover_packages() -> Vec<Package> {
    let mut packages: Vec<Package> = Vec::new();
    for directory in ["/disk/eonappfiles", "/disk", "/packages"] {
        let Ok(entries) = eonos_app::system::list_directory(directory) else { continue; };
        for filename in entries.lines().filter(|name| name.ends_with(".eonapp")).take(128) {
            let path = format!("{}/{}", directory.trim_end_matches('/'), filename);
            let Some(package) = read_package(&path) else { continue; };
            if let Some(index) = packages.iter().position(|existing| existing.id == package.id) {
                if directory == "/disk/eonappfiles" { packages[index] = package; }
            } else {
                packages.push(package);
            }
        }
    }
    packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    packages
}

fn read_package(path: &str) -> Option<Package> {
    let bytes = eonos_app::system::read_file(path).ok()?;
    if bytes.len() < 20 || &bytes[..8] != b"EONAPP1\0" { return None; }
    let metadata_len = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    let elf_len = usize::try_from(u64::from_le_bytes(bytes[12..20].try_into().ok()?)).ok()?;
    let elf_start = 20usize.checked_add(metadata_len)?;
    let end = elf_start.checked_add(elf_len)?;
    if metadata_len == 0 || metadata_len > 1024 || elf_len < 64 || end != bytes.len() { return None; }
    let metadata = core::str::from_utf8(&bytes[20..elf_start]).ok()?;
    let mut fields = metadata.lines();
    if fields.next()? != "EONOSAPP1" { return None; }
    let id = fields.next()?;
    let kind = fields.next()?;
    let name = fields.next()?;
    let description = fields.next()?;
    if fields.next().is_some() || !valid_id(id) || !matches!(kind, "ERA" | "SHELL")
        || name.is_empty() || name.len() > 80 || description.len() > 160
        || path.rsplit('/').next()? != format!("{}.eonapp", id)
    { return None; }
    let elf = &bytes[elf_start..end];
    let marker = format!("EONOSAPP1:{}", kind);
    if &elf[..4] != b"\x7fELF" || !elf.windows(marker.len()).any(|part| part == marker.as_bytes()) { return None; }
    Some(Package {
        id: id.to_string(), name: name.to_string(), description: description.to_string(),
        kind: kind.to_string(), path: path.to_string(),
    })
}

fn discover_installed() -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    for (directory, built_in) in [("/disk/apps/manifests", false), ("/apps/manifests", true)] {
        let Ok(entries) = eonos_app::system::list_directory(directory) else { continue; };
        for filename in entries.lines().filter(|name| name.ends_with(".eapp")).take(128) {
            let Some(id) = filename.strip_suffix(".eapp") else { continue; };
            let path = format!("{}/{}", directory, filename);
            let Ok(contents) = eonos_app::system::read_file(&path) else { continue; };
            let Ok(contents) = core::str::from_utf8(&contents) else { continue; };
            let mut fields = contents.lines();
            if fields.next() != Some("EONOSAPP1") || fields.next() != Some(id) { continue; }
            let Some(kind) = fields.next().filter(|kind| matches!(*kind, "ERA" | "SHELL")) else { continue; };
            let Some(name) = fields.next().filter(|name| !name.is_empty()) else { continue; };
            if apps.iter().any(|app: &InstalledApp| app.id == id) { continue; }
            apps.push(InstalledApp { id: id.to_string(), name: name.to_string(), kind: kind.to_string(), built_in });
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.as_bytes()[0].is_ascii_lowercase()
        && id.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-')
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

eonos_app::app!(ERA, main);
