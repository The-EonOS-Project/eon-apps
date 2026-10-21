#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

const SIDE: usize = 9;
const CELLS: usize = SIDE * SIDE;
const MINES: usize = 10;
const CELL_SIZE: i32 = 36;
const BOARD_X: i32 = 32;
const BOARD_Y: i32 = 88;

#[derive(Clone, Copy)]
struct Cell {
    mine: bool,
    revealed: bool,
    flagged: bool,
}

impl Cell {
    const EMPTY: Cell = Cell { mine: false, revealed: false, flagged: false };
}

fn new_board(seed: u64) -> [Cell; CELLS] {
    let mut board = [Cell::EMPTY; CELLS];
    let mut random = seed ^ 0x9e37_79b9_7f4a_7c15;
    if random == 0 { random = 1; }
    let mut placed = 0;
    while placed < MINES {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let index = (random as usize) % CELLS;
        if !board[index].mine {
            board[index].mine = true;
            placed += 1;
        }
    }
    board
}

fn neighbors(index: usize, output: &mut [usize; 8]) -> usize {
    let x = (index % SIDE) as i32;
    let y = (index / SIDE) as i32;
    let mut length = 0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 { continue; }
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0 && ny >= 0 && nx < SIDE as i32 && ny < SIDE as i32 {
                output[length] = ny as usize * SIDE + nx as usize;
                length += 1;
            }
        }
    }
    length
}

fn adjacent_mines(board: &[Cell; CELLS], index: usize) -> usize {
    let mut adjacent = [0usize; 8];
    let length = neighbors(index, &mut adjacent);
    adjacent[..length].iter().filter(|&&cell| board[cell].mine).count()
}

fn reveal(board: &mut [Cell; CELLS], start: usize, game_over: &mut bool) {
    if board[start].flagged || board[start].revealed { return; }
    if !board.iter().any(|cell| cell.revealed) && board[start].mine {
        board[start].mine = false;
        if let Some(replacement) = board
            .iter()
            .enumerate()
            .position(|(index, cell)| index != start && !cell.mine)
        {
            board[replacement].mine = true;
        }
    }
    if board[start].mine {
        board[start].revealed = true;
        *game_over = true;
        return;
    }

    let mut queue = [0usize; CELLS];
    let mut read = 0;
    let mut write = 1;
    queue[0] = start;
    board[start].revealed = true;
    while read < write {
        let index = queue[read];
        read += 1;
        if adjacent_mines(board, index) != 0 { continue; }
        let mut adjacent = [0usize; 8];
        let length = neighbors(index, &mut adjacent);
        for &next in &adjacent[..length] {
            if !board[next].revealed && !board[next].flagged && !board[next].mine {
                board[next].revealed = true;
                if adjacent_mines(board, next) == 0 && write < CELLS {
                    queue[write] = next;
                    write += 1;
                }
            }
        }
    }
}

fn main(_args: eonos_app::Args<'_>) -> i32 {
    let Ok(mut window) = eonos_app::era::Window::open("Minesweeper", 420, 468) else { return 1; };
    let mut board = new_board(eonos_app::system::uptime_ms());
    let mut previous_left = false;
    let mut previous_right = false;
    let mut game_over = false;
    let mut won = false;

    loop {
        let mut reveal_at = None;
        let mut flag_at = None;
        while let Ok(Some(event)) = window.poll_event() {
            match event {
                eonos_app::era::Event::Close => { let _ = window.close(); return 0; }
                eonos_app::era::Event::Key { usage: 0x29, pressed: true, .. } => {
                    let _ = window.close();
                    return 0;
                }
                eonos_app::era::Event::Pointer { x, y, buttons } => {
                    let left = buttons & 1 != 0;
                    let right = buttons & 2 != 0;
                    if x >= BOARD_X && y >= BOARD_Y {
                        let col = (x - BOARD_X) / CELL_SIZE;
                        let row = (y - BOARD_Y) / CELL_SIZE;
                        if col >= 0 && row >= 0 && col < SIDE as i32 && row < SIDE as i32 {
                            let index = row as usize * SIDE + col as usize;
                            if left && !previous_left { reveal_at = Some(index); }
                            if right && !previous_right { flag_at = Some(index); }
                        }
                    }
                    previous_left = left;
                    previous_right = right;
                }
                _ => {}
            }
        }

        if !game_over && !won {
            if let Some(index) = reveal_at {
                reveal(&mut board, index, &mut game_over);
                won = board.iter().filter(|cell| cell.revealed).count() == CELLS - MINES;
            }
            if let Some(index) = flag_at {
                if !board[index].revealed { board[index].flagged = !board[index].flagged; }
            }
        }

        let _ = window.fill_rect(0, 0, 420, 468, 0xD4D0C8);
        let _ = window.fill_rect(10, 10, 400, 448, 0xFFFFFF);
        let _ = window.text(24, 26, 0x000080, "Minesweeper");
        let flagged = board.iter().filter(|cell| cell.flagged).count();
        let status = if game_over {
            "Mine hit! Start a new game to try again."
        } else if won {
            "You cleared the field! Start a new game."
        } else {
            "Left-click reveal   Right-click flag"
        };
        let _ = window.text(24, 52, if game_over { 0xA00000 } else { 0x303030 }, status);
        let _ = window.text(24, 74, 0x303030, &alloc::format!("Mines: {}   Flags: {}", MINES, flagged));

        for index in 0..CELLS {
            let x = BOARD_X + (index % SIDE) as i32 * CELL_SIZE;
            let y = BOARD_Y + (index / SIDE) as i32 * CELL_SIZE;
            let cell = board[index];
            let show_mine = game_over && cell.mine;
            let revealed = cell.revealed || show_mine;
            let color = if revealed { 0xE8E4DC } else { 0xB8B8B8 };
            let _ = window.fill_rect(x + 1, y + 1, (CELL_SIZE - 2) as u32, (CELL_SIZE - 2) as u32, color);

            let label = if cell.flagged && !cell.revealed {
                "F"
            } else if show_mine {
                "*"
            } else if cell.revealed && !cell.mine {
                match adjacent_mines(&board, index) {
                    1 => "1", 2 => "2", 3 => "3", 4 => "4",
                    5 => "5", 6 => "6", 7 => "7", 8 => "8", _ => "",
                }
            } else {
                ""
            };
            if !label.is_empty() {
                let label_color = if label == "F" || label == "*" { 0xA00000 } else { 0x000080 };
                let _ = window.text(x + 13, y + 10, label_color, label);
            }
        }

        if window.button(BOARD_X, 420, 140, 34, "New Game").unwrap_or(false) {
            board = new_board(eonos_app::system::uptime_ms());
            game_over = false;
            won = false;
            previous_left = false;
            previous_right = false;
        }
        let _ = window.present();
        let _ = eonos_app::system::sleep_ms(16);
    }
}

eonos_app::app!(ERA, main);
