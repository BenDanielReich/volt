use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // keywords
    Module,
    Use,
    Struct,
    Enum,
    Const,
    Static,
    Reg,
    Extern,
    If,
    Else,
    While,
    For,
    Return,
    Break,
    Continue,
    As,
    True,
    False,
    Function,
    // primitive types
    Void,
    Int,
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    Usize,
    Isize,
    F32,
    F64,
    // literals / names
    Ident,
    Integer,
    Float,
    String,
    Char,
    // punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Bang,
    Eq,
    Lt,
    Gt,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    LtLt,
    GtGt,
    LtLtEq,
    GtGtEq,
    EqEq,
    NotEq,
    Le,
    Ge,
    AmpAmp,
    PipePipe,
    PlusPlus,
    MinusMinus,
    Dot,
    Comma,
    Colon,
    ColonColon,
    Semicolon,
    At,
    Arrow,
    Hash,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Eof,
}

impl TokenKind {
    pub fn keyword(name: &str) -> Option<Self> {
        Some(match name {
            "module" => Self::Module,
            "use" => Self::Use,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "const" => Self::Const,
            "static" => Self::Static,
            "reg" => Self::Reg,
            "extern" => Self::Extern,
            "if" => Self::If,
            "else" => Self::Else,
            "while" => Self::While,
            "for" => Self::For,
            "return" => Self::Return,
            "break" => Self::Break,
            "continue" => Self::Continue,
            "as" => Self::As,
            "true" => Self::True,
            "false" => Self::False,
            "function" => Self::Function,
            "void" => Self::Void,
            "int" => Self::Int,
            "bool" => Self::Bool,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "usize" => Self::Usize,
            "isize" => Self::Isize,
            "f32" => Self::F32,
            "f64" => Self::F64,
            _ => return None,
        })
    }

    pub fn is_type_keyword(self) -> bool {
        matches!(
            self,
            Self::Void
                | Self::Function
                | Self::Int
                | Self::Bool
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::Usize
                | Self::Isize
                | Self::F32
                | Self::F64
        )
    }

    pub fn is_assign_op(self) -> bool {
        matches!(
            self,
            Self::Eq
                | Self::PlusEq
                | Self::MinusEq
                | Self::StarEq
                | Self::SlashEq
                | Self::PercentEq
                | Self::AmpEq
                | Self::PipeEq
                | Self::CaretEq
                | Self::LtLtEq
                | Self::GtGtEq
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, lexeme: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            lexeme: lexeme.into(),
        }
    }
}
