use crate::span::Span;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: u32,
    pub name: String,
    pub src: String,
}

#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add(&mut self, name: impl Into<String>, src: impl Into<String>) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(SourceFile {
            id,
            name: name.into(),
            src: src.into(),
        });
        id
    }

    pub fn get(&self, id: u32) -> &SourceFile {
        &self.files[id as usize]
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// 1-based line and column of a byte offset.
    pub fn offset_line_col(&self, file: u32, offset: usize) -> (usize, usize) {
        if (file as usize) >= self.files.len() {
            return (1, 1);
        }
        let src = &self.get(file).src;
        let start = offset.min(src.len());
        let mut line = 1usize;
        let mut col = 1usize;
        for (i, ch) in src.char_indices() {
            if i >= start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// 1-based line and column of `span.start`.
    pub fn line_col(&self, span: Span) -> (usize, usize) {
        self.offset_line_col(span.file, span.start as usize)
    }

    /// Inclusive-start exclusive-end line/col (1-based) for a span.
    pub fn span_range(&self, span: Span) -> ((usize, usize), (usize, usize)) {
        (
            self.offset_line_col(span.file, span.start as usize),
            self.offset_line_col(span.file, span.end as usize),
        )
    }

    pub fn line_text(&self, span: Span) -> (usize, String) {
        let src = &self.get(span.file).src;
        let start = span.start as usize;
        let line_start = src[..start.min(src.len())]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let line_end = src[start.min(src.len())..]
            .find('\n')
            .map(|i| start + i)
            .unwrap_or(src.len());
        let (line, _) = self.line_col(span);
        (line, src[line_start..line_end.min(src.len())].to_string())
    }
}
