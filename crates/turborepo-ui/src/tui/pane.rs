use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
};

use ratatui::{
    backend::Backend,
    style::Style,
    widgets::{
        block::{Position, Title},
        Block, Borders, Widget,
    },
    Terminal,
};
use tracing::debug;
use tui_term::widget::PseudoTerminal;
use turborepo_vt100 as vt100;

use super::{app::Direction, Error};

pub struct TerminalPane<W> {
    tasks: BTreeMap<String, TerminalOutput<W>>,
    displayed: Option<String>,
    rows: u16,
    cols: u16,
    highlight: bool,
}

struct TerminalOutput<W> {
    rows: u16,
    cols: u16,
    parser: vt100::Parser,
    stdin: Option<W>,
    has_been_persisted: bool,
}

impl<W> TerminalPane<W> {
    pub fn new(rows: u16, cols: u16, tasks: impl IntoIterator<Item = String>) -> Self {
        // We trim 2 from rows and cols as we use them for borders
        let rows = rows.saturating_sub(2);
        let cols = cols.saturating_sub(2);
        Self {
            tasks: tasks
                .into_iter()
                .map(|name| (name, TerminalOutput::new(rows, cols, None)))
                .collect(),
            displayed: None,
            rows,
            cols,
            highlight: false,
        }
    }

    pub fn highlight(&mut self, highlight: bool) {
        self.highlight = highlight;
    }

    pub fn process_output(&mut self, task: &str, output: &[u8]) -> Result<(), Error> {
        let task = self
            .task_mut(task)
            .inspect_err(|_| debug!("cannot find task on process output"))?;
        task.parser.process(output);
        Ok(())
    }

    pub fn has_stdin(&self, task: &str) -> bool {
        self.tasks
            .get(task)
            .map(|task| task.stdin.is_some())
            .unwrap_or_default()
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), Error> {
        let changed = self.rows != rows || self.cols != cols;
        self.rows = rows;
        self.cols = cols;
        if changed {
            // Eagerly resize currently displayed terminal
            if let Some(task_name) = self.displayed.as_deref() {
                let task = self
                    .tasks
                    .get_mut(task_name)
                    .expect("displayed should always point to valid task");
                task.resize(rows, cols);
            }
        }

        Ok(())
    }

    pub fn select(&mut self, task: &str) -> Result<(), Error> {
        let rows = self.rows;
        let cols = self.cols;
        {
            let terminal = self.task_mut(task)?;
            terminal.resize(rows, cols);
        }
        self.displayed = Some(task.into());

        Ok(())
    }

    pub fn scroll(&mut self, task: &str, direction: Direction) -> Result<(), Error> {
        let task = self.task_mut(task)?;
        let scrollback = task.parser.screen().scrollback();
        let new_scrollback = match direction {
            Direction::Up => scrollback + 1,
            Direction::Down => scrollback.saturating_sub(1),
        };
        task.parser.screen_mut().set_scrollback(new_scrollback);
        Ok(())
    }

    pub fn render_screen<B: Backend>(
        &mut self,
        task_name: &str,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Error> {
        let task = self.task_mut(task_name)?;
        task.persist_screen(task_name, terminal)
    }

    pub fn render_remaining<B: Backend>(
        &mut self,
        started_tasks: HashSet<&str>,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Error> {
        for (task_name, task) in self.tasks.iter_mut() {
            if !task.has_been_persisted && started_tasks.contains(task_name.as_str()) {
                task.persist_screen(task_name, terminal)?;
            }
        }
        Ok(())
    }

    pub fn term_size(&self) -> (u16, u16) {
        (self.rows, self.cols)
    }

    fn selected(&self) -> Option<(&String, &TerminalOutput<W>)> {
        let task_name = self.displayed.as_deref()?;
        self.tasks.get_key_value(task_name)
    }

    fn task_mut(&mut self, task: &str) -> Result<&mut TerminalOutput<W>, Error> {
        self.tasks.get_mut(task).ok_or_else(|| Error::TaskNotFound {
            name: task.to_string(),
        })
    }
}

impl<W: Write> TerminalPane<W> {
    /// Insert a stdin to be associated with a task
    pub fn insert_stdin(&mut self, task_name: &str, stdin: Option<W>) -> Result<(), Error> {
        let task = self.task_mut(task_name)?;
        task.stdin = stdin;
        Ok(())
    }

    pub fn process_input(&mut self, task: &str, input: &[u8]) -> Result<(), Error> {
        let task_output = self.task_mut(task)?;
        if let Some(stdin) = &mut task_output.stdin {
            stdin.write_all(input).map_err(|e| Error::Stdin {
                name: task.into(),
                e,
            })?;
        }
        Ok(())
    }
}

impl<W> TerminalOutput<W> {
    fn new(rows: u16, cols: u16, stdin: Option<W>) -> Self {
        Self {
            parser: vt100::Parser::new(rows, cols, 1024),
            stdin,
            rows,
            cols,
            has_been_persisted: false,
        }
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        if self.rows != rows || self.cols != cols {
            self.parser.screen_mut().set_size(rows, cols);
        }
        self.rows = rows;
        self.cols = cols;
    }

    fn persist_screen<B: Backend>(
        &mut self,
        task_name: &str,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Error> {
        let screen = self.parser.entire_screen();
        let (rows, _) = screen.size();
        let mut cursor = tui_term::widget::Cursor::default();
        cursor.hide();
        let title = format!(" {task_name} >");
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title.as_str())
            .title(Title::from(title.as_str()).position(Position::Bottom));
        let term = PseudoTerminal::new(&screen).cursor(cursor).block(block);
        terminal.insert_before((rows as u16).saturating_add(2), |buf| {
            term.render(buf.area, buf)
        })?;
        self.has_been_persisted = true;

        Ok(())
    }
}

impl<W> Widget for &TerminalPane<W> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let Some((task_name, task)) = self.selected() else {
            return;
        };
        let screen = task.parser.screen();
        let mut block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {task_name} >"));
        if self.highlight {
            block = block.border_style(Style::new().fg(ratatui::style::Color::Yellow));
        }
        let term = PseudoTerminal::new(screen).block(block);
        term.render(area, buf)
    }
}

#[cfg(test)]
mod test {
    // Used by assert_buffer_eq
    #[allow(unused_imports)]
    use indoc::indoc;
    use ratatui::{assert_buffer_eq, buffer::Buffer, layout::Rect, style::Style};

    use super::*;

    #[test]
    fn test_basic() {
        let mut pane: TerminalPane<()> = TerminalPane::new(6, 8, vec!["foo".into()]);
        pane.select("foo").unwrap();
        pane.process_output("foo", b"1\r\n2\r\n3\r\n4\r\n5\r\n")
            .unwrap();

        let area = Rect::new(0, 0, 8, 6);
        let mut buffer = Buffer::empty(area);
        pane.render(area, &mut buffer);
        // Reset style change of the cursor
        buffer.set_style(Rect::new(1, 4, 1, 1), Style::reset());
        assert_buffer_eq!(
            buffer,
            Buffer::with_lines(vec![
                "┌ foo >┐",
                "│3     │",
                "│4     │",
                "│5     │",
                "│█     │",
                "└──────┘",
            ])
        );
    }
}
