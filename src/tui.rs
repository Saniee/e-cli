use std::{
    io::{self, stdout},
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use e_cli::{CliContext, Login, Tracker, commands, config, duplicate::DuplicateIndex, funcs};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Global,
    Tags,
    Favourites,
    Pool,
    Preset,
}

impl Source {
    fn title(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Tags => "Tags",
            Self::Favourites => "Favourites",
            Self::Pool => "Pool",
            Self::Preset => "Preset",
        }
    }

    fn short_title(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Tags => "Tags",
            Self::Favourites => "Favs",
            Self::Pool => "Pool",
            Self::Preset => "Preset",
        }
    }
}

#[derive(Debug)]
enum WorkerMessage {
    Status(String),
    Progress(e_cli::DownloadProgress),
    Finished(e_cli::DownloadStatistics),
    Failed(String),
}

struct App {
    source: Source,
    fields: Vec<String>,
    selected: usize,
    editing: bool,
    running: bool,
    should_quit: bool,
    progress: u16,
    status: String,
    log: Vec<String>,
    rx: Option<Receiver<WorkerMessage>>,
    cancel: Option<Arc<AtomicBool>>,
    config: config::Config,
}

impl App {
    fn new() -> Result<Self, String> {
        let path = config::path()?;
        let config = config::load(&path)?;
        let global = &config.global;
        Ok(Self {
            source: Source::Global,
            fields: vec![
                config.d_tags.tags.clone().unwrap_or_default(),
                config.d_favs.username.clone().unwrap_or_default(),
                config
                    .d_pool
                    .pool_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
                "5".to_owned(),
                global.pages.unwrap_or(1).to_string(),
                global.dir.clone().unwrap_or_else(|| "./dl/".to_owned()),
                global.num_threads.unwrap_or(5).to_string(),
                "3".to_owned(),
                String::new(),
                global.verbose.unwrap_or(false).to_string(),
                global.nsfw.unwrap_or(false).to_string(),
                global.login.unwrap_or(false).to_string(),
                global.lower_quality.unwrap_or(false).to_string(),
                global
                    .track_file
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ],
            selected: 0,
            editing: false,
            running: false,
            should_quit: false,
            progress: 0,
            status: "Ready. Select a source and press Enter to edit.".to_owned(),
            log: vec!["e-cli TUI".to_owned()],
            rx: None,
            cancel: None,
            config,
        })
    }

    fn field_name(&self, index: usize) -> &'static str {
        match index {
            0 => "Tags",
            1 => "Username",
            2 => "Pool ID",
            3 => "Posts/page",
            4 => "Pages",
            5 => "Directory",
            6 => "Threads",
            7 => "Retries",
            8 => "Preset name",
            9 => "Verbose",
            10 => "NSFW API",
            11 => "Login",
            12 => "Lower quality",
            _ => "Tracking file",
        }
    }

    fn visible_fields(&self) -> Vec<usize> {
        match self.source {
            Source::Global => vec![5, 6, 7, 9, 10, 11, 12, 13],
            Source::Tags => vec![0, 3, 4, 5, 6, 7],
            Source::Favourites => vec![1, 0, 3, 4, 5, 6, 7],
            Source::Pool => vec![2, 5, 6, 7],
            Source::Preset => vec![8, 3, 4, 5, 6, 7],
        }
    }

    fn change_source(&mut self, direction: i32) {
        let next = (self.source as i32 + direction).rem_euclid(5);
        self.source = match next {
            0 => Source::Global,
            1 => Source::Tags,
            2 => Source::Favourites,
            3 => Source::Pool,
            _ => Source::Preset,
        };
        self.selected = 0;
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(cancel) = &self.cancel {
                cancel.store(true, Ordering::Relaxed);
                self.status = "Cancellation requested; finishing the current file...".to_owned();
                return;
            }
            self.should_quit = true;
            return;
        }
        if self.editing {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.editing = false,
                KeyCode::Backspace => {
                    self.fields[self.selected].pop();
                }
                KeyCode::Char(ch) => self.fields[self.selected].push(ch),
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') if self.running => {
                if let Some(cancel) = &self.cancel {
                    cancel.store(true, Ordering::Relaxed);
                    self.status =
                        "Cancellation requested; finishing the current file...".to_owned();
                }
            }
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc if self.running => {
                if let Some(cancel) = &self.cancel {
                    cancel.store(true, Ordering::Relaxed);
                    self.status =
                        "Cancellation requested; finishing the current file...".to_owned();
                }
            }
            KeyCode::Left => self.change_source(-1),
            KeyCode::Right => self.change_source(1),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Enter => {
                if self.running {
                    self.status =
                        "The active worker cannot be interrupted safely; wait for it to finish."
                            .to_owned();
                } else {
                    self.editing = true;
                }
            }
            KeyCode::Char('s') => self.save_config(),
            KeyCode::Char(' ')
                if self.source == Source::Global && (9..=12).contains(&self.selected) =>
            {
                self.fields[self.selected] =
                    (!matches!(self.fields[self.selected].as_str(), "true")).to_string();
            }
            KeyCode::Char(' ') if self.source != Source::Global => self.start_download(),
            _ => {}
        }
    }

    fn move_selection(&mut self, direction: i32) {
        let fields = self.visible_fields();
        let current = fields
            .iter()
            .position(|field| *field == self.selected)
            .unwrap_or(0);
        let next = (current as i32 + direction).clamp(0, fields.len() as i32 - 1) as usize;
        self.selected = fields[next];
    }

    fn save_config(&mut self) {
        self.config.global.dir = Some(self.fields[5].clone());
        self.config.global.pages = self.fields[4].parse().ok();
        self.config.global.num_threads = self.fields[6].parse().ok();
        self.config.global.verbose = self.fields[9].parse().ok();
        self.config.global.nsfw = self.fields[10].parse().ok();
        self.config.global.login = self.fields[11].parse().ok();
        self.config.global.lower_quality = self.fields[12].parse().ok();
        self.config.global.track_file = if self.fields[13].is_empty() {
            None
        } else {
            Some(self.fields[13].clone().into())
        };
        match self.source {
            Source::Global => {}
            Source::Tags => {
                self.config.d_tags.tags = Some(self.fields[0].clone());
                self.config.d_tags.count = self.fields[3].parse().ok();
            }
            Source::Favourites => {
                self.config.d_favs.username = Some(self.fields[1].clone());
                self.config.d_favs.tags = Some(self.fields[0].clone());
                self.config.d_favs.count = self.fields[3].parse().ok();
            }
            Source::Pool => self.config.d_pool.pool_id = self.fields[2].parse().ok(),
            Source::Preset => {
                if !self.fields[8].is_empty() {
                    self.config.presets.insert(
                        self.fields[8].clone(),
                        config::PresetConfig {
                            tags: Some(self.fields[0].clone()),
                            count: self.fields[3].parse().ok(),
                            pages: self.fields[4].parse().ok(),
                            dir: Some(self.fields[5].clone()),
                            ..Default::default()
                        },
                    );
                }
            }
        }
        match config::save(&self.config) {
            Ok(()) => self.status = "Configuration saved.".to_owned(),
            Err(error) => self.status = error,
        }
    }

    fn start_download(&mut self) {
        let source = self.source;
        let fields = self.fields.clone();
        let file_config = self.config.clone();
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.rx = Some(rx);
        self.cancel = Some(cancel.clone());
        self.running = true;
        self.progress = 0;
        self.status = "Starting download worker...".to_owned();
        self.log.push("Starting download...".to_owned());
        thread::spawn(move || run_download(source, fields, file_config, tx, cancel));
    }

    fn poll_worker(&mut self) {
        let Some(rx) = self.rx.take() else { return };
        let mut keep = true;
        while let Ok(message) = rx.try_recv() {
            match message {
                WorkerMessage::Status(status) => {
                    self.status = status.clone();
                    self.log.push(status);
                }
                WorkerMessage::Progress(progress) => {
                    let done = progress.completed + progress.failed + progress.skipped;
                    self.progress = if progress.total == 0 {
                        0
                    } else {
                        (done.max(0) as usize)
                            .saturating_mul(100)
                            .checked_div(progress.total)
                            .unwrap_or(0)
                            .min(99) as u16
                    };
                    self.status = format!(
                        "{} of {} posts processed ({} bytes downloaded).",
                        done, progress.total, progress.downloaded_amount as u64
                    );
                }
                WorkerMessage::Finished(stats) => {
                    self.progress = 100;
                    self.running = false;
                    self.cancel = None;
                    self.status = format!(
                        "Finished: {} downloaded, {} skipped, {} failed.",
                        stats.completed, stats.skipped, stats.failed
                    );
                    self.log.push(self.status.clone());
                    keep = false;
                }
                WorkerMessage::Failed(error) => {
                    self.running = false;
                    self.cancel = None;
                    self.status = error.clone();
                    self.log.push(error);
                    keep = false;
                }
            }
        }
        if keep {
            self.rx = Some(rx);
        }
    }
}

fn run_download(
    source: Source,
    fields: Vec<String>,
    config: config::Config,
    tx: Sender<WorkerMessage>,
    cancel: Arc<AtomicBool>,
) {
    let send = |message| {
        let _ = tx.send(message);
    };
    let dir = Path::new(&fields[5]);
    funcs::ensure_dl_dir(dir);
    let duplicate_path = dir.join(".e-cli-md5.json");
    let duplicate_index = match DuplicateIndex::load(&duplicate_path) {
        Ok(index) => Some(std::sync::Arc::new(index)),
        Err(error) => {
            return send(WorkerMessage::Failed(format!(
                "Failed to load duplicate index: {error}"
            )));
        }
    };
    let context = CliContext {
        verbose: fields[9].parse().unwrap_or(false),
        nsfw: fields[10].parse().unwrap_or(false),
        lower_quality: fields[12].parse().unwrap_or(false),
        pages: fields[4].parse().unwrap_or(1),
        num_threads: fields[6].parse().unwrap_or(5).clamp(1, 10),
        retries: fields[7].parse().unwrap_or(3),
        duplicate_index,
        cancel: Some(cancel.clone()),
        progress: Some(std::sync::Arc::new({
            let tx = tx.clone();
            move |progress| {
                let _ = tx.send(WorkerMessage::Progress(progress));
            }
        })),
    };
    let login = Login {
        username: String::new(),
        api_key: String::new(),
    };
    let tracker_path = if fields[13].is_empty() {
        config.global.track_file.as_deref()
    } else {
        Some(Path::new(&fields[13]))
    };
    let tracker = match tracker_path
        .map(Tracker::load)
        .transpose()
        .map_err(|error| error.to_string())
    {
        Ok(tracker) => tracker,
        Err(error) => return send(WorkerMessage::Failed(error)),
    };
    let mp = indicatif::MultiProgress::new();
    send(WorkerMessage::Status(format!(
        "Fetching {}...",
        source.title()
    )));
    let stats = match source {
        Source::Global => {
            return send(WorkerMessage::Failed(
                "Select a download source before starting.".to_owned(),
            ));
        }
        Source::Tags => commands::download_search(
            &context,
            &login,
            &fields[0],
            &fields[3].parse().unwrap_or(5),
            &false,
            &mp,
            dir,
            tracker.as_ref(),
        ),
        Source::Favourites => commands::download_favourites(
            &context,
            &login,
            &fields[1],
            &fields[3].parse().unwrap_or(5),
            &false,
            &fields[0],
            &mp,
            dir,
            tracker.as_ref(),
        ),
        Source::Pool => commands::download_pool(
            &context,
            &login,
            &fields[2].parse().unwrap_or_default(),
            &mp,
            dir,
            tracker.as_ref(),
        ),
        Source::Preset => {
            let Some(preset) = config.presets.get(&fields[8]) else {
                return send(WorkerMessage::Failed(format!(
                    "Unknown preset '{}'.",
                    fields[8]
                )));
            };
            commands::download_search(
                &context,
                &login,
                preset.tags.as_deref().unwrap_or_default(),
                &fields[3].parse().ok().or(preset.count).unwrap_or(5),
                &preset.random.unwrap_or(false),
                &mp,
                dir,
                tracker.as_ref(),
            )
        }
    };
    if cancel.load(Ordering::Relaxed) {
        send(WorkerMessage::Status(
            "Cancelled after the current file.".to_owned(),
        ));
    }
    send(WorkerMessage::Finished(stats));
}

pub fn run() -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("Could not enable raw terminal mode: {e}"))?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)
        .map_err(|e| format!("Could not enter alternate screen: {e}"))?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Could not create terminal: {e}"))?;
    let result = app_loop(&mut terminal);
    disable_raw_mode().map_err(|e| format!("Could not restore terminal mode: {e}"))?;
    execute!(stdout(), LeaveAlternateScreen)
        .map_err(|e| format!("Could not leave alternate screen: {e}"))?;
    result.map_err(|e| format!("TUI error: {e}"))
}

fn app_loop(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> io::Result<()> {
    let mut app = App::new().map_err(io::Error::other)?;
    while !app.should_quit {
        terminal.draw(|frame| draw(frame, &app))?;
        app.poll_worker();
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key);
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let accent = Color::Rgb(92, 214, 190);
    let muted = Color::Rgb(130, 145, 160);
    let panel = Style::default().fg(Color::Rgb(205, 215, 225));
    let area = frame.area();
    let compact = area.width < 100;
    let tiny = area.height < 22;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);
    let titles = [
        Source::Global,
        Source::Tags,
        Source::Favourites,
        Source::Pool,
        Source::Preset,
    ]
    .into_iter()
    .map(|source| Line::from(source.short_title()))
    .collect::<Vec<_>>();
    frame.render_widget(
        Tabs::new(titles)
            .select(app.source as usize)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .title(" e-cli / DOWNLOAD MODE "),
            )
            .style(panel)
            .highlight_style(Style::default().fg(accent).add_modifier(Modifier::BOLD)),
        layout[0],
    );
    let body = if compact {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(layout[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
            .split(layout[1])
    };
    draw_form(frame, app, body[0], !compact && !tiny);
    draw_activity(frame, app, body[1], panel, muted);
    let footer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(layout[2]);
    frame.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .title(format!(" Progress {:>3}% ", app.progress)),
            )
            .gauge_style(Style::default().fg(accent))
            .percent(app.progress),
        footer[0],
    );
    let controls = if tiny {
        "Arrows navigate  Enter edit  Space start  Esc cancel  q quit"
    } else {
        "<-/-> source  ^/v field  Enter edit  Space start  s save  Esc cancel  q quit"
    };
    frame.render_widget(
        Paragraph::new(format!("{} | {controls}", app.status))
            .style(Style::default().fg(muted))
            .wrap(Wrap { trim: true }),
        footer[1],
    );
}

fn draw_activity(frame: &mut Frame, app: &App, area: Rect, panel: Style, muted: Color) {
    let log = app
        .log
        .iter()
        .rev()
        .take(10)
        .map(|line| ListItem::new(line.clone()))
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(log).style(panel).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(muted))
                .title(" Activity / WORKER LOG "),
        ),
        area,
    );
}

fn draw_form(frame: &mut Frame, app: &App, area: Rect, spacious: bool) {
    let fields = app.visible_fields();
    let items = fields
        .into_iter()
        .map(|index| {
            let selected = app.selected == index;
            let marker = if selected { "◆" } else { "·" };
            let value = if app.selected == index && app.editing {
                format!("{}_", app.fields[index])
            } else {
                app.fields[index].clone()
            };
            let value_style = if selected {
                Style::default()
                    .fg(Color::Rgb(92, 214, 190))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let primary = Line::from(vec![
                Span::raw(format!("{marker} {:<14}", app.field_name(index))),
                Span::styled(value, value_style),
            ]);
            if spacious {
                vec![
                    primary,
                    Line::from(Span::styled(
                        field_hint(index),
                        Style::default().fg(Color::Rgb(130, 145, 160)),
                    )),
                ]
            } else {
                vec![primary]
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(130, 145, 160)))
                    .title(format!(" Settings / {} ", app.source.title())),
            )
            .highlight_style(Style::default().bg(Color::Rgb(25, 45, 50))),
        area,
    );
}

fn field_hint(index: usize) -> &'static str {
    match index {
        0 => "Search expression used by the API.",
        1 => "Account whose favorites should be downloaded.",
        2 => "Numeric pool identifier.",
        3 => "Posts requested per API page.",
        4 => "Number of API pages to fetch.",
        5 => "Destination directory for downloaded files.",
        6 => "Parallel workers, from 1 to 10.",
        7 => "Retries for transient download failures.",
        8 => "Saved preset name to create or edit.",
        9 => "Show detailed diagnostic output.",
        10 => "Use the e621.net API instead of e926.net.",
        11 => "Use configured credentials for API requests.",
        12 => "Prefer sample files when available.",
        _ => "Optional file storing downloaded post IDs.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_tabs_cycle() {
        let mut app = App::new().expect("config path should be available");
        app.change_source(1);
        assert_eq!(app.source, Source::Tags);
        app.change_source(-1);
        assert_eq!(app.source, Source::Global);
    }

    #[test]
    fn visible_fields_match_source() {
        let mut app = App::new().expect("config path should be available");
        app.source = Source::Tags;
        assert!(app.visible_fields().contains(&0));
        app.source = Source::Pool;
        assert_eq!(app.visible_fields(), vec![2, 5, 6, 7]);
    }
}
