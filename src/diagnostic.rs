use crate::source::SourceMap;
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: Level::Error,
            message: message.into(),
            span,
            notes: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            level: Level::Warning,
            message: message.into(),
            span,
            notes: Vec::new(),
        }
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn render(&self, sources: &SourceMap) -> String {
        let mut out = String::new();
        let level = match self.level {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
        };
        if self.span.is_dummy() || (self.span.file as usize) >= sources.files().len() {
            out.push_str(&format!("{level}: {}\n", self.message));
        } else {
            let file = sources.get(self.span.file);
            let (line, col) = sources.line_col(self.span);
            let (line_no, text) = sources.line_text(self.span);
            out.push_str(&format!(
                "{level}: {}\n  --> {}:{}:{}\n",
                self.message, file.name, line, col
            ));
            out.push_str(&format!("   |\n{line_no:>4} | {text}\n   |"));
            let caret_col = col.saturating_sub(1);
            let width = (self.span.end.saturating_sub(self.span.start)).max(1) as usize;
            out.push_str(&format!(
                "\n   | {}{}\n",
                " ".repeat(caret_col),
                "^".repeat(width.min(40))
            ));
        }
        for note in &self.notes {
            out.push_str(&format!("   = note: {note}\n"));
        }
        out
    }
}

pub fn render_all(diags: &[Diagnostic], sources: &SourceMap) -> String {
    diags.iter().map(|d| d.render(sources)).collect()
}
