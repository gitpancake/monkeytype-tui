use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io;
use std::time::Duration;

use crate::test::{Mode, Test, TestResult};
use crate::words;

pub fn run(mode: Mode) -> Result<TestResult> {
    let word_count = match mode {
        Mode::Time(t) => (t.max(15) * 4) as usize, // generous buffer
        Mode::Words(n) => n as usize,
    };
    let mut test = Test::new(mode, words::pick(word_count));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = event_loop(&mut terminal, &mut test);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res?;
    Ok(test.finalize())
}

fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    test: &mut Test,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, test))?;

        if test.finished() {
            return Ok(());
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(k) = event::read()? {
                match (k.code, k.modifiers) {
                    (KeyCode::Esc, _) => return Ok(()),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),
                    (KeyCode::Backspace, _) => test.type_char('\u{8}'),
                    (KeyCode::Char(c), _) => test.type_char(c),
                    _ => {}
                }
            }
        }
    }
}

fn draw(f: &mut ratatui::Frame, test: &Test) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    // header
    let header = match test.mode {
        Mode::Time(t) => format!(
            "time {:>4.1}s / {t}s   chars {}",
            test.time_left().unwrap_or(0.0),
            test.char_counts().1
        ),
        Mode::Words(n) => format!("words {}/{}", test.current_word, n),
    };
    f.render_widget(
        Paragraph::new(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title("monkeytype-tui"),
        ),
        chunks[0],
    );

    // words
    let mut spans: Vec<Span> = Vec::new();
    for (wi, word) in test.words.iter().enumerate() {
        let typed = &test.typed[wi];
        let target: Vec<char> = word.chars().collect();
        let max_len = target.len().max(typed.len());
        for ci in 0..max_len {
            let target_c = target.get(ci).copied();
            let typed_c = typed.get(ci).and_then(|x| *x);
            let (ch, style) = match (target_c, typed_c) {
                (Some(t), Some(c)) if t == c => (
                    t,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                (Some(t), Some(_)) => (
                    t,
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                (Some(t), None) => (t, Style::default().fg(Color::Rgb(110, 110, 110))),
                (None, Some(c)) => (c, Style::default().fg(Color::Magenta)),
                _ => (' ', Style::default()),
            };
            if wi == test.current_word && ci == test.current_char {
                spans.push(Span::styled(
                    ch.to_string(),
                    style.bg(Color::Yellow).fg(Color::Black),
                ));
            } else {
                spans.push(Span::styled(ch.to_string(), style));
            }
        }
        spans.push(Span::raw(" "));
    }
    let para = Paragraph::new(Line::from(spans))
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title("type"));
    f.render_widget(para, chunks[1]);

    // footer
    let (correct, total) = test.char_counts();
    let acc = if total == 0 {
        0.0
    } else {
        correct as f64 / total as f64 * 100.0
    };
    let elapsed = test.elapsed().max(0.001);
    let live_wpm = (correct as f64 / 5.0) / (elapsed / 60.0);
    let footer = format!("wpm {:.1}   acc {:.1}%   esc to quit", live_wpm, acc);
    f.render_widget(
        Paragraph::new(footer).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}
