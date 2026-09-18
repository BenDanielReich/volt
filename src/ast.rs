use crate::span::{Ident, Span};

#[derive(Debug, Clone)]
pub struct Module {
    pub name: Option<Ident>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    Use(UseItem),
    Include(IncludeItem),
    Fn(FnItem),
    Struct(StructItem),
    Enum(EnumItem),
    Const(ConstItem),
    Static(StaticItem),
    Reg(RegItem),
    TypeAlias(TypeAliasItem),
    Impl(ImplItem),
    Trait(TraitItem),
    Pin(PinItem),
    Peripheral(PeripheralItem),
}

#[derive(Debug, Clone)]
pub struct UseItem {
    pub path: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: Ident,
    pub value: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeStyle {
    Angle,
    Quote,
}

#[derive(Debug, Clone)]
pub struct IncludeItem {
    pub path: String,
    pub style: IncludeStyle,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnItem {
    pub attrs: Vec<Attribute>,
    pub is_extern: bool,
    pub is_export: bool,
    pub is_internal: bool,
    pub is_comptime: bool,
    pub is_async: bool,
    pub return_ty: Type,
    pub name: Ident,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ty: Type,
    pub name: Ident,
    pub is_self: bool,
}

#[derive(Debug, Clone)]
pub struct StructItem {
    pub name: Ident,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub ty: Type,
    pub name: Ident,
}

#[derive(Debug, Clone)]
pub struct EnumItem {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: Ident,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct ConstItem {
    pub ty: Type,
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StaticItem {
    pub is_extern: bool,
    pub ty: Type,
    pub name: Ident,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RegItem {
    pub ty: Type,
    pub name: Ident,
    pub address: Expr,
    pub bits: Vec<BitField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BitField {
    pub name: Ident,
    pub start: Expr,
    pub width: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAliasItem {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplItem {
    pub trait_name: Option<Ident>,
    pub ty: Type,
    pub methods: Vec<FnItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitItem {
    pub name: Ident,
    pub methods: Vec<FnItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PinItem {
    pub name: Ident,
    pub reg: Ident,
    pub bit: Ident,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PeripheralItem {
    pub name: Ident,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Named(Ident),
    Void,
    Primitive(PrimitiveTy),
    Bits {
        signed: bool,
        width: u8,
    },
    Pointer(Box<Type>),
    Ref {
        mutable: bool,
        inner: Box<Type>,
    },
    Array(Box<Type>, Box<Expr>),
    SelfTy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTy {
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
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Local(LocalStmt),
    Expr(Expr),
    Assign(AssignStmt),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Loop(Block),
    Match(MatchStmt),
    When(WhenStmt),
    Await(Expr, Span),
    Asm(AsmStmt),
    Return(Option<Expr>, Span),
    Break(Span),
    Continue(Span),
    Block(Block),
}

#[derive(Debug, Clone)]
pub struct LocalStmt {
    pub is_const: bool,
    pub ty: Type,
    pub name: Ident,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub lhs: Expr,
    pub op: AssignOp,
    pub rhs: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Eq,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub cond: Expr,
    pub then_branch: Box<Stmt>,
    pub else_branch: Option<Box<Stmt>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub cond: Expr,
    pub body: Box<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<Stmt>>,
    pub cond: Option<Expr>,
    pub step: Option<Expr>,
    pub body: Box<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchStmt {
    pub scrutinee: Expr,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Box<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard(Span),
    Ident(Ident),
    Enum(Ident, Ident),
    Int(IntLit, Span),
}

#[derive(Debug, Clone)]
pub struct WhenStmt {
    pub cond: Expr,
    pub then_branch: Block,
    pub else_branch: Option<Block>,
    pub taken: Option<bool>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AsmStmt {
    pub template: String,
    pub binds: Vec<AsmBind>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AsmBind {
    pub dir: AsmDir,
    pub name: Ident,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsmDir {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(IntLit),
    Float(String),
    Bool(bool),
    Char(char),
    String(String),
    Ident(Ident),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, Ident),
    Cast(Box<Expr>, Type),
    Paren(Box<Expr>),
    Await(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct IntLit {
    pub value: i128,
    pub suffix: Option<PrimitiveTy>,
    pub raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
    Deref,
    AddrOf,
    AddrOfMut,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    WrapAdd,
    WrapSub,
    WrapMul,
    SatAdd,
    SatSub,
    SatMul,
}

impl AssignOp {
    pub fn from_token(kind: crate::token::TokenKind) -> Option<Self> {
        use crate::token::TokenKind::*;
        Some(match kind {
            Eq => Self::Eq,
            PlusEq => Self::Add,
            MinusEq => Self::Sub,
            StarEq => Self::Mul,
            SlashEq => Self::Div,
            PercentEq => Self::Rem,
            AmpEq => Self::BitAnd,
            PipeEq => Self::BitOr,
            CaretEq => Self::BitXor,
            LtLtEq => Self::Shl,
            GtGtEq => Self::Shr,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Add => "+=",
            Self::Sub => "-=",
            Self::Mul => "*=",
            Self::Div => "/=",
            Self::Rem => "%=",
            Self::BitAnd => "&=",
            Self::BitOr => "|=",
            Self::BitXor => "^=",
            Self::Shl => "<<=",
            Self::Shr => ">>=",
        }
    }
}

impl BinOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::And => "&&",
            Self::Or => "||",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::WrapAdd => "+%",
            Self::WrapSub => "-%",
            Self::WrapMul => "*%",
            Self::SatAdd => "+|",
            Self::SatSub => "-|",
            Self::SatMul => "*|",
        }
    }
}

impl UnOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Not => "!",
            Self::BitNot => "~",
            Self::Deref => "*",
            Self::AddrOf => "&",
            Self::AddrOfMut => "&mut ",
            Self::PreInc | Self::PostInc => "++",
            Self::PreDec | Self::PostDec => "--",
        }
    }
}

impl PrimitiveTy {
    pub fn name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Bool => "bool",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::Usize => "usize",
            Self::Isize => "isize",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }

    pub fn from_suffix(s: &str) -> Option<Self> {
        Some(match s {
            "int" => Self::Int,
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
}

impl Type {
    pub fn void(span: Span) -> Self {
        Self {
            kind: TypeKind::Void,
            span,
        }
    }
}
