//! Quake-style drop-down debug console.

/// A Quake-style drop-down debug console that captures input when active.
pub struct DebugConsole {
    /// Whether the console overlay is visible
    pub active: bool,
    /// Current text being typed by the user
    pub input_buffer: String,
    /// Command + output history lines displayed in the console
    pub history: Vec<String>,
    /// Scroll offset for browsing history (0 = bottom / most recent)
    pub scroll_offset: usize,
    /// Previously executed commands for up/down recall
    pub cmd_history: Vec<String>,
    /// Index into `cmd_history` while recalling (None = fresh input)
    pub cmd_index: Option<usize>,
    /// Tab-completion: cached candidate matches
    pub tab_matches: Vec<String>,
    /// Current position when cycling through tab matches
    pub tab_cycle_index: usize,
    /// The prefix that was used to generate the current tab matches
    pub tab_prefix: String,
}

impl DebugConsole {
    pub fn new() -> Self {
        Self {
            active: false,
            input_buffer: String::new(),
            history: Vec::new(),
            scroll_offset: 0,
            cmd_history: Vec::new(),
            cmd_index: None,
            tab_matches: Vec::new(),
            tab_cycle_index: 0,
            tab_prefix: String::new(),
        }
    }

    /// Push a line to history, capping at 200 lines.
    pub fn push_history(&mut self, line: String) {
        self.history.push(line);
        while self.history.len() > 200 {
            self.history.remove(0);
        }
        // Reset scroll to bottom when new output arrives
        self.scroll_offset = 0;
    }

    /// Scroll up by `n` lines (toward older history).
    pub fn scroll_up(&mut self, n: usize) {
        let max = self.history.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    /// Scroll down by `n` lines (toward newer history).
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }
}
