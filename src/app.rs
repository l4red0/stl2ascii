use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::io::stdout;
use std::time::Duration;

use crate::camera::Camera;
use crate::cli::ShadeMode;
use crate::mesh::Mesh;
use crate::render::{format_status, render_grid};

pub struct App {
    mesh: Mesh,
    camera: Camera,
    initial: Camera,
    shade: ShadeMode,
}

impl App {
    pub fn new(mesh: Mesh, camera: Camera, shade: ShadeMode) -> Self {
        let initial = camera.clone();
        Self {
            mesh,
            camera,
            initial,
            shade,
        }
    }

    pub fn run(mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;
        let result = self.event_loop(&mut terminal);
        restore_terminal()?;
        result
    }

    fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if !event::poll(Duration::from_millis(50))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Left | KeyCode::Char('h') => self.camera.yaw -= 5.0,
                KeyCode::Right | KeyCode::Char('l') => self.camera.yaw += 5.0,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.camera.pitch = (self.camera.pitch + 5.0).clamp(-89.0, 89.0)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.camera.pitch = (self.camera.pitch - 5.0).clamp(-89.0, 89.0)
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    self.camera.zoom = (self.camera.zoom * 1.1).min(50.0)
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    self.camera.zoom = (self.camera.zoom / 1.1).max(0.05)
                }
                KeyCode::Char('w') => self.camera.pan_y += 0.05,
                KeyCode::Char('s') => self.camera.pan_y -= 0.05,
                KeyCode::Char('a') => self.camera.pan_x -= 0.05,
                KeyCode::Char('d') => self.camera.pan_x += 0.05,
                KeyCode::Char('m') => self.shade = self.shade.toggle(),
                KeyCode::Char('r') => self.camera.reset_from(&self.initial),
                _ => {}
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);

        let block = Block::default()
            .title(" stl2ascii ")
            .borders(Borders::ALL);
        let inner = block.inner(chunks[0]);
        let w = inner.width as usize;
        let h = inner.height as usize;
        let grid = render_grid(&self.mesh, &self.camera, self.shade, w, h);
        let text: Vec<Line> = grid.into_iter().map(Line::from).collect();

        let view = Paragraph::new(text).block(block);
        frame.render_widget(view, chunks[0]);

        let status = format_status(
            &self.mesh,
            &self.camera,
            self.shade,
            chunks[1].width as usize,
        );
        frame.render_widget(Paragraph::new(status), chunks[1]);
    }
}

fn setup_terminal() -> Result<DefaultTerminal> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(ratatui::init())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    ratatui::restore();
    Ok(())
}
