use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::token::{Token, TokenKind};

pub struct Lexer<'src> {
    src: &'src str,
    file: u32,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Lexer<'src> {
    pub fn new(file: u32, src: &'src str) -> Self {
        Self {
            src,
            file,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn tokenize(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if eof {
                break;
            }
        }
        (tokens, self.diagnostics)
    }

    fn next_token(&mut self) -> Token {
        self.skip_ws_and_comments();
        let start = self.pos;
        let Some(ch) = self.peek() else {
            return self.mk(TokenKind::Eof, start, "");
        };
        match ch {
            'a'..='z' | 'A'..='Z' | '_' => self.ident_or_kw(start),
            '0'..='9' => self.number(start),
            '"' => self.string(start),
            '\'' => self.char_lit(start),
            '(' => self.bump_simple(TokenKind::LParen, start),
            ')' => self.bump_simple(TokenKind::RParen, start),
            '{' => self.bump_simple(TokenKind::LBrace, start),
            '}' => self.bump_simple(TokenKind::RBrace, start),
            '[' => self.bump_simple(TokenKind::LBracket, start),
            ']' => self.bump_simple(TokenKind::RBracket, start),
            ',' => self.bump_simple(TokenKind::Comma, start),
            ';' => self.bump_simple(TokenKind::Semicolon, start),
            '@' => self.bump_simple(TokenKind::At, start),
            '~' => self.bump_simple(TokenKind::Tilde, start),
            '#' => self.bump_simple(TokenKind::Hash, start),
            '.' => self.bump_simple(TokenKind::Dot, start),
            '+' => self.compound2(
                start,
                '+',
                TokenKind::PlusPlus,
                '=',
                TokenKind::PlusEq,
                TokenKind::Plus,
            ),
            '-' => self.minus(start),
            '*' => self.compound1(start, '=', TokenKind::StarEq, TokenKind::Star),
            '%' => self.compound1(start, '=', TokenKind::PercentEq, TokenKind::Percent),
            '^' => self.compound1(start, '=', TokenKind::CaretEq, TokenKind::Caret),
            '!' => self.compound1(start, '=', TokenKind::NotEq, TokenKind::Bang),
            '=' => self.compound1(start, '=', TokenKind::EqEq, TokenKind::Eq),
            '&' => self.amp(start),
            '|' => self.pipe(start),
            '<' => self.lt(start),
            '>' => self.gt(start),
            ':' => {
                self.bump();
                if self.peek() == Some(':') {
                    self.bump();
                    self.mk(TokenKind::ColonColon, start, "::")
                } else {
                    self.mk(TokenKind::Colon, start, ":")
                }
            }
            '/' => {
                // comments already skipped; this is division
                self.compound1(start, '=', TokenKind::SlashEq, TokenKind::Slash)
            }
            _ => {
                self.bump();
                let span = self.span(start, self.pos);
                self.diagnostics.push(Diagnostic::error(
                    format!("unexpected character `{ch}`"),
                    span,
                ));
                self.mk(TokenKind::Eof, start, "")
            }
        }
    }

    fn ident_or_kw(&mut self, start: usize) -> Token {
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }
        let lex = &self.src[start..self.pos];
        let kind = TokenKind::keyword(lex).unwrap_or(TokenKind::Ident);
        self.mk(kind, start, lex)
    }

    fn number(&mut self, start: usize) -> Token {
        let base = if self.peek() == Some('0') {
            match self.peek_at(1) {
                Some('x') | Some('X') => {
                    self.bump();
                    self.bump();
                    16
                }
                Some('b') | Some('B') => {
                    self.bump();
                    self.bump();
                    2
                }
                _ => 10,
            }
        } else {
            10
        };
        let is_digit = |c: char| match base {
            16 => c.is_ascii_hexdigit(),
            2 => c == '0' || c == '1',
            _ => c.is_ascii_digit(),
        };
        while matches!(self.peek(), Some(c) if is_digit(c) || c == '_') {
            self.bump();
        }
        let mut kind = TokenKind::Integer;
        if base == 10
            && self.peek() == Some('.')
            && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit())
        {
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
                self.bump();
            }
            kind = TokenKind::Float;
        }
        // optional type suffix: u8, i32, …
        if matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
            while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric()) {
                self.bump();
            }
        }
        let lex = &self.src[start..self.pos];
        self.mk(kind, start, lex)
    }

    fn string(&mut self, start: usize) -> Token {
        self.bump(); // "
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.bump();
                let lex = &self.src[start..self.pos];
                return self.mk(TokenKind::String, start, lex);
            }
            if ch == '\\' {
                self.bump();
                self.bump();
                continue;
            }
            if ch == '\n' {
                break;
            }
            self.bump();
        }
        self.diagnostics.push(Diagnostic::error(
            "unterminated string literal",
            self.span(start, self.pos),
        ));
        self.mk(TokenKind::String, start, &self.src[start..self.pos])
    }

    fn char_lit(&mut self, start: usize) -> Token {
        self.bump(); // '
        if self.peek() == Some('\\') {
            self.bump();
            self.bump();
        } else {
            self.bump();
        }
        if self.peek() == Some('\'') {
            self.bump();
        } else {
            self.diagnostics.push(Diagnostic::error(
                "unterminated character literal",
                self.span(start, self.pos),
            ));
        }
        self.mk(TokenKind::Char, start, &self.src[start..self.pos])
    }

    fn minus(&mut self, start: usize) -> Token {
        self.bump();
        match self.peek() {
            Some('-') => {
                self.bump();
                self.mk(TokenKind::MinusMinus, start, "--")
            }
            Some('=') => {
                self.bump();
                self.mk(TokenKind::MinusEq, start, "-=")
            }
            Some('>') => {
                self.bump();
                self.mk(TokenKind::Arrow, start, "->")
            }
            _ => self.mk(TokenKind::Minus, start, "-"),
        }
    }

    fn amp(&mut self, start: usize) -> Token {
        self.bump();
        match self.peek() {
            Some('&') => {
                self.bump();
                self.mk(TokenKind::AmpAmp, start, "&&")
            }
            Some('=') => {
                self.bump();
                self.mk(TokenKind::AmpEq, start, "&=")
            }
            _ => self.mk(TokenKind::Amp, start, "&"),
        }
    }

    fn pipe(&mut self, start: usize) -> Token {
        self.bump();
        match self.peek() {
            Some('|') => {
                self.bump();
                self.mk(TokenKind::PipePipe, start, "||")
            }
            Some('=') => {
                self.bump();
                self.mk(TokenKind::PipeEq, start, "|=")
            }
            _ => self.mk(TokenKind::Pipe, start, "|"),
        }
    }

    fn lt(&mut self, start: usize) -> Token {
        self.bump();
        match self.peek() {
            Some('<') => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    self.mk(TokenKind::LtLtEq, start, "<<=")
                } else {
                    self.mk(TokenKind::LtLt, start, "<<")
                }
            }
            Some('=') => {
                self.bump();
                self.mk(TokenKind::Le, start, "<=")
            }
            _ => self.mk(TokenKind::Lt, start, "<"),
        }
    }

    fn gt(&mut self, start: usize) -> Token {
        self.bump();
        match self.peek() {
            Some('>') => {
                self.bump();
                if self.peek() == Some('=') {
                    self.bump();
                    self.mk(TokenKind::GtGtEq, start, ">>=")
                } else {
                    self.mk(TokenKind::GtGt, start, ">>")
                }
            }
            Some('=') => {
                self.bump();
                self.mk(TokenKind::Ge, start, ">=")
            }
            _ => self.mk(TokenKind::Gt, start, ">"),
        }
    }

    fn compound1(&mut self, start: usize, next: char, two: TokenKind, one: TokenKind) -> Token {
        self.bump();
        if self.peek() == Some(next) {
            self.bump();
            self.mk(two, start, &self.src[start..self.pos])
        } else {
            self.mk(one, start, &self.src[start..self.pos])
        }
    }

    fn compound2(
        &mut self,
        start: usize,
        a: char,
        aa: TokenKind,
        b: char,
        ab: TokenKind,
        one: TokenKind,
    ) -> Token {
        self.bump();
        if self.peek() == Some(a) {
            self.bump();
            self.mk(aa, start, &self.src[start..self.pos])
        } else if self.peek() == Some(b) {
            self.bump();
            self.mk(ab, start, &self.src[start..self.pos])
        } else {
            self.mk(one, start, &self.src[start..self.pos])
        }
    }

    fn bump_simple(&mut self, kind: TokenKind, start: usize) -> Token {
        self.bump();
        self.mk(kind, start, &self.src[start..self.pos])
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.bump();
            }
            if self.peek() == Some('/') && self.peek_at(1) == Some('/') {
                while matches!(self.peek(), Some(c) if c != '\n') {
                    self.bump();
                }
                continue;
            }
            if self.peek() == Some('/') && self.peek_at(1) == Some('*') {
                let start = self.pos;
                self.bump();
                self.bump();
                loop {
                    match self.peek() {
                        None => {
                            self.diagnostics.push(Diagnostic::error(
                                "unterminated block comment",
                                self.span(start, self.pos),
                            ));
                            break;
                        }
                        Some('*') if self.peek_at(1) == Some('/') => {
                            self.bump();
                            self.bump();
                            break;
                        }
                        _ => {
                            self.bump();
                        }
                    }
                }
                continue;
            }
            break;
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_at(&self, offset_chars: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(offset_chars)
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.file, start as u32, end as u32)
    }

    fn mk(&self, kind: TokenKind, start: usize, lex: &str) -> Token {
        Token::new(kind, self.span(start, self.pos), lex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        let (toks, diags) = Lexer::new(0, src).tokenize();
        assert!(diags.is_empty(), "{diags:?}");
        toks.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn keywords_and_idents() {
        let k = kinds("module blink function int PORTB foo");
        assert_eq!(
            k,
            vec![
                TokenKind::Module,
                TokenKind::Ident,
                TokenKind::Function,
                TokenKind::Int,
                TokenKind::Ident,
                TokenKind::Ident,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn numbers() {
        let k = kinds("42 0xFF 0b1010 3.14 1u8");
        assert_eq!(
            k,
            vec![
                TokenKind::Integer,
                TokenKind::Integer,
                TokenKind::Integer,
                TokenKind::Float,
                TokenKind::Integer,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn operators() {
        let k = kinds("+= <<= -> :: && || ++ -- + - * / < >");
        assert_eq!(
            k,
            vec![
                TokenKind::PlusEq,
                TokenKind::LtLtEq,
                TokenKind::Arrow,
                TokenKind::ColonColon,
                TokenKind::AmpAmp,
                TokenKind::PipePipe,
                TokenKind::PlusPlus,
                TokenKind::MinusMinus,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn comments_are_skipped() {
        let k = kinds("u8 /* block */ x // line\n = 1");
        assert_eq!(
            k,
            vec![
                TokenKind::U8,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Integer,
                TokenKind::Eof,
            ]
        );
    }
}
