use clap::Parser;
use crossterm::{
    execute,
    event::{
        poll, 
        read, 
        Event, 
        KeyCode, 
        KeyEventKind,
        PopKeyboardEnhancementFlags,
    },
    terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use shinobu8_core::*;
use std::time::Duration;
use std::{io::Stdout, thread};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    rom: String,
}

fn is_event_available() -> std::io::Result<bool> {
    // Zero duration says that the `poll` function must return immediately
    // with an `Event` availability information
    poll(Duration::from_secs(0))
}

fn main() {
    execute!(
        std::io::stdout(),
        PopKeyboardEnhancementFlags,
    ).expect("Failed to pop keyboard enhancement flags.");

    let args = Args::parse();
    if args.rom.is_empty() {
        println!("Please provide a ROM file.");
        return;
    }
    enable_raw_mode().expect("Failed to enable raw mode.");

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))
        .expect("Failed to create terminal.");
    terminal.clear().expect("Failed to clear terminal.");

    let rom = std::fs::read(&args.rom).unwrap();
    let mut emu = Emu::new();
    emu.load(&rom);

    loop {
        if is_event_available().expect("Failed to poll event.") {
            let event = read().unwrap();
            match event {
                Event::Key(event) => match event.code {
                    KeyCode::Esc => {
                        disable_raw_mode().expect("Failed to disable raw mode.");
                        terminal
                            .backend_mut()
                            .execute(LeaveAlternateScreen)
                            .unwrap();
                        break;
                    }
                    _ => {
                        if let Some(key) = to_chip8_key(event.code) {
                            match event.kind {
                                KeyEventKind::Press => emu.key_press(key),
                                KeyEventKind::Release => emu.key_release(key),
                                KeyEventKind::Repeat => {}
                            }
                        }
                    }
                },
                _ => {}
            }
        }

        emu.cycle().expect("Failed to execute instruction.");

        let matrix = emu.get_diaplay();

        draw(&mut terminal, matrix);
    }
}

fn draw(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    matrix: &[bool; SCREEN_WIDTH * SCREEN_HEIGHT],
) {
    terminal
        .draw(|f| {
            f.render_widget(
                Game::new(matrix),
                Rect::new(0, 0, SCREEN_WIDTH as u16, SCREEN_HEIGHT as u16),
            );
        })
        .expect("Failed to draw.");
}

struct Game<'a>(&'a [bool; SCREEN_WIDTH * SCREEN_HEIGHT]);

impl Widget for Game<'_> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let pixel = self.0[y * SCREEN_WIDTH + x];
                let style = Style::default().bg(if pixel { Color::White } else { Color::Black });
                let char = if pixel { " " } else { " " };
                buf.set_string(x as u16, y as u16, char, style);
            }
        }
    }
}

impl<'a> Game<'a> {
    fn new(matrix: &'a [bool; SCREEN_WIDTH * SCREEN_HEIGHT]) -> Self {
        Game(matrix)
    }
}

// Chip-8 keypad layout mapping:
// 1 2 3 4 -> 1 2 3 C
// Q W E R -> 4 5 6 D
// A S D F -> 7 8 9 E
// Z X C V -> A 0 B F
fn to_chip8_key(key: KeyCode) -> Option<u8> {
    match key {
        KeyCode::Char('1') => Some(0x1),
        KeyCode::Char('2') => Some(0x2),
        KeyCode::Char('3') => Some(0x3),
        KeyCode::Char('4') => Some(0xC),

        KeyCode::Char('q') => Some(0x4),
        KeyCode::Char('w') => Some(0x5),
        KeyCode::Char('e') => Some(0x6),
        KeyCode::Char('r') => Some(0xD),

        KeyCode::Char('a') => Some(0x7),
        KeyCode::Char('s') => Some(0x8),
        KeyCode::Char('d') => Some(0x9),
        KeyCode::Char('f') => Some(0xE),

        KeyCode::Char('z') => Some(0xA),
        KeyCode::Char('x') => Some(0x0),
        KeyCode::Char('c') => Some(0xB),
        KeyCode::Char('v') => Some(0xF),
        _ => None,
    }
}
