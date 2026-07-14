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

pub enum Outcome {
    Quit,
    Replay,
    Sync(TestResult),
}

pub fn run(
    mode: Mode,
    initial: Option<TestResult>,
    sync_status: Option<&str>,
) -> Result<Outcome> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let outcome = run_inner(&mut terminal, mode, initial, sync_status);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    outcome
}

fn run_inner<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mode: Mode,
    initial: Option<TestResult>,
    sync_status: Option<&str>,
) -> Result<Outcome> {
    let result = match initial {
        Some(r) => r,
        None => {
            let word_count = match mode {
                Mode::Time(t) => (t.max(15) * 4) as usize,
                Mode::Words(n) => n as usize,
            };
            let mut test = Test::new(mode, words::pick(word_count));
            type_loop(terminal, &mut test)?;
            if !test.finished() {
                return Ok(Outcome::Quit);
            }
            test.finalize()
        }
    };

    stats_loop(terminal, &result, sync_status)
}

fn type_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    test: &mut Test,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw_test(f, test))?;

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

fn stats_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    result: &TestResult,
    sync_status: Option<&str>,
) -> Result<Outcome> {
    loop {
        terminal.draw(|f| draw_stats(f, result, sync_status))?;
        if let Event::Key(k) = event::read()? {
            match (k.code, k.modifiers) {
                (KeyCode::Esc, _)
                | (KeyCode::Char('q'), _)
                | (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(Outcome::Quit),
                (KeyCode::Char('r'), _) => return Ok(Outcome::Replay),
                (KeyCode::Char('s'), _) => return Ok(Outcome::Sync(result.clone())),
                _ => {}
            }
        }
    }
}

fn draw_test(f: &mut ratatui::Frame, test: &Test) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

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

fn draw_stats(f: &mut ratatui::Frame, r: &TestResult, sync_status: Option<&str>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let mode_str = match r.mode {
        Mode::Time(t) => format!("time {t}s"),
        Mode::Words(n) => format!("words {n}"),
    };
    f.render_widget(
        Paragraph::new(format!("result — {mode_str}")).block(
            Block::default()
                .borders(Borders::ALL)
                .title("monkeytype-tui"),
        ),
        chunks[0],
    );

    let big = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::Rgb(160, 160, 160));
    let ok = Style::default().fg(Color::Green);
    let err = Style::default().fg(Color::Red);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("wpm          ", dim),
            Span::styled(format!("{:.1}", r.wpm), big),
        ]),
        Line::from(vec![
            Span::styled("raw          ", dim),
            Span::styled(format!("{:.1}", r.raw_wpm), big),
        ]),
        Line::from(vec![
            Span::styled("accuracy     ", dim),
            Span::styled(format!("{:.1}%", r.accuracy), big),
        ]),
        Line::from(vec![
            Span::styled("consistency  ", dim),
            Span::styled(format!("{:.1}%", r.consistency), big),
        ]),
        Line::from(vec![
            Span::styled("duration     ", dim),
            Span::styled(format!("{:.1}s", r.test_duration), big),
        ]),
        Line::from(vec![
            Span::styled("chars        ", dim),
            Span::styled(format!("{}", r.correct_chars), ok),
            Span::raw(" / "),
            Span::styled(format!("{}", r.incorrect_chars), err),
            Span::raw(" / "),
            Span::styled(format!("{}", r.extra_chars), dim),
            Span::raw(" / "),
            Span::styled(format!("{}", r.missed_chars), dim),
            Span::styled("   (correct / wrong / extra / missed)", dim),
        ]),
    ];
    if let Some(s) = sync_status {
        lines.push(Line::from(""));
        let style = if s.starts_with("synced") { ok } else { err };
        lines.push(Line::from(Span::styled(s.to_string(), style)));
    }

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title("stats")),
        chunks[1],
    );

    let footer = "[r] replay   [s] sync   [q] quit";
    f.render_widget(
        Paragraph::new(footer).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}
