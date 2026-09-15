use std::collections::HashMap;

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::span::{Ident, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Void,
    Bool,
    Int(IntTy),
    Float(FloatTy),
    Pointer(Box<Ty>),
    Array(Box<Ty>, u64),
    Struct(String),
    Enum(String),
    /// Untyped integer literal; coerces to a concrete integer.
    IntLit,
    /// Untyped float literal; coerces to `f32` or `f64`.
    FloatLit,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntTy {
    Int,
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatTy {
    F32,
    F64,
}

impl Ty {
    pub fn name(&self) -> String {
        match self {
            Ty::Void => "void".into(),
            Ty::Bool => "bool".into(),
            Ty::Int(i) => i.name().into(),
            Ty::Float(FloatTy::F32) => "f32".into(),
            Ty::Float(FloatTy::F64) => "f64".into(),
            Ty::Pointer(inner) => format!("{}*", inner.name()),
            Ty::Array(inner, n) => format!("{}[{n}]", inner.name()),
            Ty::Struct(n) | Ty::Enum(n) => n.clone(),
            Ty::IntLit => "{{integer}}".into(),
            Ty::FloatLit => "{{float}}".into(),
            Ty::Error => "{{error}}".into(),
        }
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Ty::Int(_) | Ty::IntLit)
    }

    pub fn is_signed_int(&self) -> bool {
        matches!(
            self,
            Ty::Int(IntTy::Int | IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::Isize)
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Ty::Float(_) | Ty::FloatLit)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_int() || self.is_float()
    }

    pub fn is_bool_like(&self) -> bool {
        matches!(self, Ty::Bool | Ty::Int(_) | Ty::IntLit)
    }
}

impl IntTy {
    pub fn name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::Isize => "isize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::Usize => "usize",
        }
    }

    pub fn from_prim(p: PrimitiveTy) -> Option<Self> {
        Some(match p {
            PrimitiveTy::Int => Self::Int,
            PrimitiveTy::I8 => Self::I8,
            PrimitiveTy::I16 => Self::I16,
            PrimitiveTy::I32 => Self::I32,
            PrimitiveTy::I64 => Self::I64,
            PrimitiveTy::Isize => Self::Isize,
            PrimitiveTy::U8 => Self::U8,
            PrimitiveTy::U16 => Self::U16,
            PrimitiveTy::U32 => Self::U32,
            PrimitiveTy::U64 => Self::U64,
            PrimitiveTy::Usize => Self::Usize,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub return_ty: Ty,
    pub is_extern: bool,
    pub attrs: Vec<Attribute>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<(String, i128)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstDef {
    pub name: String,
    pub ty: Ty,
    pub value: i128,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StaticDef {
    pub name: String,
    pub ty: Ty,
    pub is_extern: bool,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RegDef {
    pub name: String,
    pub ty: Ty,
    pub address: u64,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub module_name: Option<String>,
    pub funcs: Vec<FuncDef>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub consts: Vec<ConstDef>,
    pub statics: Vec<StaticDef>,
    pub regs: Vec<RegDef>,
    pub c_includes: Vec<CInclude>,
}

#[derive(Debug, Clone)]
pub struct CInclude {
    pub path: String,
    pub angled: bool,
}

#[derive(Clone)]
struct Symbol {
    ty: Ty,
    kind: SymbolKind,
}

#[derive(Clone)]
enum SymbolKind {
    Local {
        is_const: bool,
    },
    Param,
    Const {
        value: i128,
    },
    Static,
    Reg,
    Fn,
    Struct,
    Enum,
    EnumVariant {
        value: i128,
        #[allow(dead_code)]
        enum_name: String,
    },
}

pub struct Checker {
    scopes: Vec<HashMap<String, Symbol>>,
    structs: HashMap<String, StructDef>,
    enums: HashMap<String, EnumDef>,
    consts: HashMap<String, ConstDef>,
    fn_sigs: HashMap<String, Vec<Ty>>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Checker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
            enums: HashMap::new(),
            consts: HashMap::new(),
            fn_sigs: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn check_modules(&mut self, modules: &[Module]) -> Program {
        let mut program = Program {
            module_name: modules
                .first()
                .and_then(|m| m.name.as_ref().map(|n| n.name.clone())),
            funcs: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            consts: Vec::new(),
            statics: Vec::new(),
            regs: Vec::new(),
            c_includes: Vec::new(),
        };

        // Pass 1: collect types and signatures.
        for module in modules {
            for item in &module.items {
                match item {
                    Item::Use(_) | Item::Include(_) => {}
                    Item::Struct(s) => self.collect_struct(s),
                    Item::Enum(e) => self.collect_enum(e),
                    Item::Const(_) | Item::Static(_) | Item::Reg(_) | Item::Fn(_) => {}
                }
            }
        }
        for module in modules {
            for item in &module.items {
                match item {
                    Item::Const(c) => self.collect_const(c),
                    Item::Static(s) => {
                        if let Some(def) = self.collect_static(s) {
                            program.statics.push(def);
                        }
                    }
                    Item::Reg(r) => {
                        if let Some(def) = self.collect_reg(r) {
                            program.regs.push(def);
                        }
                    }
                    Item::Fn(f) => {
                        if let Some(def) = self.collect_fn_sig(f) {
                            program.funcs.push(def);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Pass 2: check function bodies with signatures in scope.
        for func in &mut program.funcs {
            if let Some(body) = func.body.take() {
                self.push();
                for (name, ty) in &func.params {
                    self.define(
                        name,
                        Symbol {
                            ty: ty.clone(),
                            kind: SymbolKind::Param,
                        },
                        func.span,
                    );
                }
                self.check_block(&body, &func.return_ty);
                self.pop();
                func.body = Some(body);
            }
        }

        program.structs = self.structs.values().cloned().collect();
        program.enums = self.enums.values().cloned().collect();
        program.consts = self.consts.values().cloned().collect();
        program
    }

    fn collect_struct(&mut self, s: &StructItem) {
        let mut fields = Vec::new();
        for f in &s.fields {
            fields.push((f.name.name.clone(), self.resolve_type(&f.ty)));
        }
        let def = StructDef {
            name: s.name.name.clone(),
            fields,
            span: s.span,
        };
        self.define(
            &s.name.name,
            Symbol {
                ty: Ty::Struct(s.name.name.clone()),
                kind: SymbolKind::Struct,
            },
            s.name.span,
        );
        self.structs.insert(s.name.name.clone(), def);
    }

    fn collect_enum(&mut self, e: &EnumItem) {
        let mut variants = Vec::new();
        let mut next = 0i128;
        for v in &e.variants {
            let value = if let Some(expr) = &v.value {
                self.eval_const(expr).unwrap_or(next)
            } else {
                next
            };
            next = value + 1;
            variants.push((v.name.name.clone(), value));
            let qname = format!("{}.{}", e.name.name, v.name.name);
            self.define(
                &qname,
                Symbol {
                    ty: Ty::Enum(e.name.name.clone()),
                    kind: SymbolKind::EnumVariant {
                        value,
                        enum_name: e.name.name.clone(),
                    },
                },
                v.name.span,
            );
        }
        let def = EnumDef {
            name: e.name.name.clone(),
            variants,
            span: e.span,
        };
        self.define(
            &e.name.name,
            Symbol {
                ty: Ty::Enum(e.name.name.clone()),
                kind: SymbolKind::Enum,
            },
            e.name.span,
        );
        self.enums.insert(e.name.name.clone(), def);
    }

    fn collect_const(&mut self, c: &ConstItem) {
        let ty = self.resolve_type(&c.ty);
        let value = self.eval_const(&c.value).unwrap_or(0);
        let def = ConstDef {
            name: c.name.name.clone(),
            ty: ty.clone(),
            value,
            span: c.span,
        };
        self.define(
            &c.name.name,
            Symbol {
                ty,
                kind: SymbolKind::Const { value },
            },
            c.name.span,
        );
        self.consts.insert(c.name.name.clone(), def);
    }

    fn collect_static(&mut self, s: &StaticItem) -> Option<StaticDef> {
        let ty = self.resolve_type(&s.ty);
        if let Some(init) = &s.value {
            let got = self.check_expr(init);
            self.expect_type(&ty, &got, init.span);
        }
        self.define(
            &s.name.name,
            Symbol {
                ty: ty.clone(),
                kind: SymbolKind::Static,
            },
            s.name.span,
        );
        Some(StaticDef {
            name: s.name.name.clone(),
            ty,
            is_extern: s.is_extern,
            init: s.value.clone(),
            span: s.span,
        })
    }

    fn collect_reg(&mut self, r: &RegItem) -> Option<RegDef> {
        let ty = self.resolve_type(&r.ty);
        let address = self.eval_const(&r.address).unwrap_or(0) as u64;
        self.define(
            &r.name.name,
            Symbol {
                ty: ty.clone(),
                kind: SymbolKind::Reg,
            },
            r.name.span,
        );
        Some(RegDef {
            name: r.name.name.clone(),
            ty,
            address,
            span: r.span,
        })
    }

    fn collect_fn_sig(&mut self, f: &FnItem) -> Option<FuncDef> {
        let return_ty = self.resolve_type(&f.return_ty);
        let params = f
            .params
            .iter()
            .map(|p| (p.name.name.clone(), self.resolve_type(&p.ty)))
            .collect::<Vec<_>>();
        self.define(
            &f.name.name,
            Symbol {
                ty: return_ty.clone(),
                kind: SymbolKind::Fn,
            },
            f.name.span,
        );
        self.remember_fn(
            &f.name.name,
            params.iter().map(|(_, ty)| ty.clone()).collect(),
        );
        Some(FuncDef {
            name: f.name.name.clone(),
            params,
            return_ty,
            is_extern: f.is_extern,
            attrs: f.attrs.clone(),
            body: f.body.clone(),
            span: f.span,
        })
    }

    fn check_block(&mut self, block: &Block, return_ty: &Ty) {
        self.push();
        for stmt in &block.stmts {
            self.check_stmt(stmt, return_ty);
        }
        self.pop();
    }

    fn check_stmt(&mut self, stmt: &Stmt, return_ty: &Ty) {
        match stmt {
            Stmt::Local(local) => {
                let ty = self.resolve_type(&local.ty);
                if let Some(init) = &local.init {
                    let got = self.check_expr(init);
                    self.expect_type(&ty, &got, init.span);
                }
                self.define(
                    &local.name.name,
                    Symbol {
                        ty,
                        kind: SymbolKind::Local {
                            is_const: local.is_const,
                        },
                    },
                    local.name.span,
                );
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
            Stmt::Assign(a) => {
                self.check_assignable(&a.lhs);
                let lhs = self.check_expr(&a.lhs);
                let rhs = self.check_expr(&a.rhs);
                self.expect_type(&lhs, &rhs, a.rhs.span);
            }
            Stmt::If(i) => {
                let cond = self.check_expr(&i.cond);
                if !cond.is_bool_like() && cond != Ty::Error {
                    self.error(i.cond.span, format!("condition has type `{}`", cond.name()));
                }
                self.check_stmt(&i.then_branch, return_ty);
                if let Some(e) = &i.else_branch {
                    self.check_stmt(e, return_ty);
                }
            }
            Stmt::While(w) => {
                let cond = self.check_expr(&w.cond);
                if !cond.is_bool_like() && cond != Ty::Error {
                    self.error(w.cond.span, format!("condition has type `{}`", cond.name()));
                }
                self.check_stmt(&w.body, return_ty);
            }
            Stmt::For(f) => {
                self.push();
                if let Some(init) = &f.init {
                    self.check_stmt(init, return_ty);
                }
                if let Some(cond) = &f.cond {
                    let ty = self.check_expr(cond);
                    if !ty.is_bool_like() && ty != Ty::Error {
                        self.error(cond.span, format!("condition has type `{}`", ty.name()));
                    }
                }
                if let Some(step) = &f.step {
                    self.check_expr(step);
                }
                self.check_stmt(&f.body, return_ty);
                self.pop();
            }
            Stmt::Loop(body) => self.check_block(body, return_ty),
            Stmt::Return(value, span) => match (value, return_ty) {
                (None, Ty::Void) => {}
                (None, _) => self.error(*span, "missing return value"),
                (Some(expr), Ty::Void) => {
                    let _ = self.check_expr(expr);
                    self.error(*span, "cannot return a value from void function");
                }
                (Some(expr), expected) => {
                    let got = self.check_expr(expr);
                    self.expect_type(expected, &got, expr.span);
                }
            },
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Block(b) => self.check_block(b, return_ty),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Ty {
        match &expr.kind {
            ExprKind::Int(lit) => {
                if let Some(prim) = lit.suffix {
                    if let Some(int) = IntTy::from_prim(prim) {
                        Ty::Int(int)
                    } else {
                        Ty::IntLit
                    }
                } else {
                    Ty::IntLit
                }
            }
            ExprKind::Float(_) => Ty::FloatLit,
            ExprKind::Bool(_) => Ty::Bool,
            ExprKind::Char(_) => Ty::Int(IntTy::U8),
            ExprKind::String(_) => Ty::Pointer(Box::new(Ty::Int(IntTy::U8))),
            ExprKind::Ident(id) => self.lookup(id),
            ExprKind::Paren(inner) => self.check_expr(inner),
            ExprKind::Unary(op, inner) => self.check_unary(*op, inner),
            ExprKind::Binary(op, lhs, rhs) => self.check_binary(*op, lhs, rhs),
            ExprKind::Call(callee, args) => self.check_call(callee, args, expr.span),
            ExprKind::Index(base, index) => {
                let base_ty = self.check_expr(base);
                let idx_ty = self.check_expr(index);
                if !idx_ty.is_int() && idx_ty != Ty::Error {
                    self.error(index.span, "array index must be an integer");
                }
                match base_ty {
                    Ty::Array(elem, _) | Ty::Pointer(elem) => *elem,
                    Ty::Error => Ty::Error,
                    other => {
                        self.error(base.span, format!("cannot index `{}`", other.name()));
                        Ty::Error
                    }
                }
            }
            ExprKind::Field(base, field) => self.check_field(base, field),
            ExprKind::Cast(inner, ty) => {
                let _ = self.check_expr(inner);
                self.resolve_type(ty)
            }
        }
    }

    fn check_unary(&mut self, op: UnOp, inner: &Expr) -> Ty {
        if matches!(
            op,
            UnOp::PreInc | UnOp::PreDec | UnOp::PostInc | UnOp::PostDec
        ) {
            self.check_assignable(inner);
        }
        let ty = self.check_expr(inner);
        match op {
            UnOp::Neg
            | UnOp::BitNot
            | UnOp::PreInc
            | UnOp::PreDec
            | UnOp::PostInc
            | UnOp::PostDec => {
                if ty.is_numeric() || ty == Ty::Error {
                    if ty == Ty::IntLit {
                        Ty::Int(IntTy::I32)
                    } else {
                        ty
                    }
                } else {
                    self.error(
                        inner.span,
                        format!("cannot apply `{op:?}` to `{}`", ty.name()),
                    );
                    Ty::Error
                }
            }
            UnOp::Not => {
                if ty.is_bool_like() || ty == Ty::Error {
                    Ty::Bool
                } else {
                    self.error(inner.span, format!("cannot apply `!` to `{}`", ty.name()));
                    Ty::Error
                }
            }
            UnOp::Deref => match ty {
                Ty::Pointer(inner) => *inner,
                Ty::Error => Ty::Error,
                other => {
                    self.error(inner.span, format!("cannot dereference `{}`", other.name()));
                    Ty::Error
                }
            },
            UnOp::AddrOf => Ty::Pointer(Box::new(ty)),
        }
    }

    fn check_binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr) -> Ty {
        let left = self.check_expr(lhs);
        let right = self.check_expr(rhs);
        match op {
            BinOp::And | BinOp::Or => {
                if !left.is_bool_like() {
                    self.error(lhs.span, format!("expected bool, found `{}`", left.name()));
                }
                if !right.is_bool_like() {
                    self.error(rhs.span, format!("expected bool, found `{}`", right.name()));
                }
                Ty::Bool
            }
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.unify_numeric(&left, &right, rhs.span);
                Ty::Bool
            }
            _ => self.unify_numeric(&left, &right, rhs.span),
        }
    }

    fn unify_numeric(&mut self, left: &Ty, right: &Ty, span: Span) -> Ty {
        if *left == Ty::Error || *right == Ty::Error {
            return Ty::Error;
        }
        if left == right {
            return left.clone();
        }
        if *left == Ty::IntLit && right.is_int() {
            return right.clone();
        }
        if *right == Ty::IntLit && left.is_int() {
            return left.clone();
        }
        if *left == Ty::IntLit && *right == Ty::IntLit {
            return Ty::IntLit;
        }
        if *left == Ty::FloatLit && matches!(right, Ty::Float(_)) {
            return right.clone();
        }
        if *right == Ty::FloatLit && matches!(left, Ty::Float(_)) {
            return left.clone();
        }
        if *left == Ty::FloatLit && *right == Ty::FloatLit {
            return Ty::FloatLit;
        }
        if *left == Ty::IntLit && right.is_float() {
            return right.clone();
        }
        if *right == Ty::IntLit && left.is_float() {
            return left.clone();
        }
        if *left == Ty::FloatLit && right.is_int() {
            return Ty::Float(FloatTy::F64);
        }
        if *right == Ty::FloatLit && left.is_int() {
            return Ty::Float(FloatTy::F64);
        }
        if left.is_int() && matches!(right, Ty::Enum(_)) {
            return left.clone();
        }
        if right.is_int() && matches!(left, Ty::Enum(_)) {
            return right.clone();
        }
        if matches!((left, right), (Ty::Enum(_), Ty::Enum(_))) && left == right {
            return left.clone();
        }
        // pointer + int
        if let Ty::Pointer(elem) = left {
            if right.is_int() {
                return Ty::Pointer(elem.clone());
            }
        }
        self.error(
            span,
            format!("mismatched types `{}` and `{}`", left.name(), right.name()),
        );
        Ty::Error
    }

    fn check_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Ty {
        let ExprKind::Ident(name) = &callee.kind else {
            self.error(callee.span, "can only call a named function in v0");
            for a in args {
                self.check_expr(a);
            }
            return Ty::Error;
        };
        let Some(sym) = self.get(&name.name) else {
            self.error(name.span, format!("unknown function `{}`", name.name));
            for a in args {
                self.check_expr(a);
            }
            return Ty::Error;
        };
        if !matches!(sym.kind, SymbolKind::Fn) {
            self.error(name.span, format!("`{}` is not a function", name.name));
        }
        // Look up the full signature from the collected fn symbol's type (return only).
        // Re-walk params from a side table: we stored only return type on the symbol.
        // Recover via a second lookup of collected function list on the global scope
        // by checking args loosely if we cannot find arity. For v0, scan funcs...
        // We keep signatures in a dedicated map.
        if let Some(sig) = self.fn_params(&name.name) {
            if sig.len() != args.len() {
                self.error(
                    span,
                    format!(
                        "function `{}` expects {} argument(s), found {}",
                        name.name,
                        sig.len(),
                        args.len()
                    ),
                );
            }
            for (arg, expected) in args.iter().zip(sig.iter()) {
                let got = self.check_expr(arg);
                self.expect_type(expected, &got, arg.span);
            }
        } else {
            for a in args {
                self.check_expr(a);
            }
        }
        sym.ty
    }

    fn fn_params(&self, name: &str) -> Option<Vec<Ty>> {
        // Stored on the root scope as a synthetic symbol? We stash params in consts-like map.
        self.fn_sigs.get(name).cloned()
    }

    fn check_field(&mut self, base: &Expr, field: &Ident) -> Ty {
        // Enum variant: Ident.Field where Ident is an enum type.
        if let ExprKind::Ident(name) = &base.kind {
            if let Some(en) = self.enums.get(&name.name).cloned() {
                if let Some((_, value)) = en.variants.iter().find(|(n, _)| n == &field.name) {
                    let _ = value;
                    return Ty::Enum(en.name);
                }
                self.error(
                    field.span,
                    format!("no variant `{}` on enum `{}`", field.name, name.name),
                );
                return Ty::Error;
            }
        }
        let mut base_ty = self.check_expr(base);
        if let Ty::Pointer(inner) = base_ty {
            base_ty = *inner;
        }
        match base_ty {
            Ty::Struct(name) => {
                if let Some(st) = self.structs.get(&name) {
                    if let Some((_, ty)) = st.fields.iter().find(|(n, _)| n == &field.name) {
                        return ty.clone();
                    }
                    self.error(
                        field.span,
                        format!("no field `{}` on struct `{name}`", field.name),
                    );
                    Ty::Error
                } else {
                    Ty::Error
                }
            }
            Ty::Error => Ty::Error,
            other => {
                self.error(
                    field.span,
                    format!("`{}` has no field `{}`", other.name(), field.name),
                );
                Ty::Error
            }
        }
    }

    fn resolve_type(&mut self, ty: &Type) -> Ty {
        match &ty.kind {
            TypeKind::Void => Ty::Void,
            TypeKind::Primitive(p) => match p {
                PrimitiveTy::Bool => Ty::Bool,
                PrimitiveTy::F32 => Ty::Float(FloatTy::F32),
                PrimitiveTy::F64 => Ty::Float(FloatTy::F64),
                other => Ty::Int(IntTy::from_prim(*other).expect("int prim")),
            },
            TypeKind::Named(name) => {
                if self.structs.contains_key(&name.name) {
                    Ty::Struct(name.name.clone())
                } else if self.enums.contains_key(&name.name) {
                    Ty::Enum(name.name.clone())
                } else if let Some(sym) = self.get(&name.name) {
                    match sym.kind {
                        SymbolKind::Struct => Ty::Struct(name.name.clone()),
                        SymbolKind::Enum => Ty::Enum(name.name.clone()),
                        _ => {
                            self.error(name.span, format!("`{}` is not a type", name.name));
                            Ty::Error
                        }
                    }
                } else {
                    self.error(name.span, format!("unknown type `{}`", name.name));
                    Ty::Error
                }
            }
            TypeKind::Pointer(inner) => Ty::Pointer(Box::new(self.resolve_type(inner))),
            TypeKind::Array(inner, len) => {
                let n = self.eval_const(len).unwrap_or(0) as u64;
                Ty::Array(Box::new(self.resolve_type(inner)), n)
            }
        }
    }

    fn expect_type(&mut self, expected: &Ty, got: &Ty, span: Span) {
        if *got == Ty::Error || *expected == Ty::Error {
            return;
        }
        if expected == got {
            return;
        }
        if *got == Ty::IntLit && expected.is_int() {
            return;
        }
        if *expected == Ty::IntLit && got.is_int() {
            return;
        }
        if *got == Ty::FloatLit && expected.is_float() {
            return;
        }
        if *expected == Ty::FloatLit && got.is_float() {
            return;
        }
        if *got == Ty::IntLit && expected.is_float() {
            return;
        }
        // enum used as its integer value in C-ish contexts
        if expected.is_int() && matches!(got, Ty::Enum(_)) {
            return;
        }
        if got.is_int() && matches!(expected, Ty::Enum(_)) {
            return;
        }
        self.error(
            span,
            format!("expected `{}`, found `{}`", expected.name(), got.name()),
        );
    }

    fn eval_const(&mut self, expr: &Expr) -> Option<i128> {
        match &expr.kind {
            ExprKind::Int(lit) => Some(lit.value),
            ExprKind::Char(c) => Some(*c as i128),
            ExprKind::Bool(b) => Some(i128::from(*b)),
            ExprKind::Ident(id) => match self.get(&id.name) {
                Some(Symbol {
                    kind: SymbolKind::Const { value },
                    ..
                }) => Some(value),
                Some(Symbol {
                    kind: SymbolKind::EnumVariant { value, .. },
                    ..
                }) => Some(value),
                _ => {
                    self.error(id.span, format!("`{}` is not a constant", id.name));
                    None
                }
            },
            ExprKind::Unary(UnOp::Neg, inner) => self.eval_const(inner).map(|v| -v),
            ExprKind::Unary(UnOp::BitNot, inner) => self.eval_const(inner).map(|v| !v),
            ExprKind::Binary(op, lhs, rhs) => {
                let a = self.eval_const(lhs)?;
                let b = self.eval_const(rhs)?;
                Some(match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div if b != 0 => a / b,
                    BinOp::Rem if b != 0 => a % b,
                    BinOp::BitAnd => a & b,
                    BinOp::BitOr => a | b,
                    BinOp::BitXor => a ^ b,
                    BinOp::Shl => a << b,
                    BinOp::Shr => a >> b,
                    _ => {
                        self.error(expr.span, "expression is not a constant");
                        return None;
                    }
                })
            }
            ExprKind::Cast(inner, _) => self.eval_const(inner),
            ExprKind::Paren(inner) => self.eval_const(inner),
            ExprKind::Field(base, field) => {
                if let ExprKind::Ident(name) = &base.kind {
                    if let Some(en) = self.enums.get(&name.name) {
                        if let Some((_, v)) = en.variants.iter().find(|(n, _)| n == &field.name) {
                            return Some(*v);
                        }
                    }
                }
                self.error(expr.span, "expression is not a constant");
                None
            }
            _ => {
                self.error(expr.span, "expression is not a constant");
                None
            }
        }
    }

    fn check_assignable(&mut self, expr: &Expr) {
        let inner = match &expr.kind {
            ExprKind::Paren(e) => e,
            other => {
                return self.check_assignable_inner(other, expr.span);
            }
        };
        self.check_assignable(inner);
    }

    fn check_assignable_inner(&mut self, kind: &ExprKind, span: crate::span::Span) {
        if let ExprKind::Ident(id) = kind {
            match self.get(&id.name).map(|s| s.kind) {
                Some(SymbolKind::Const { .. }) | Some(SymbolKind::Local { is_const: true }) => {
                    self.error(id.span, format!("cannot assign to const `{}`", id.name));
                }
                _ => {}
            }
        }
        let _ = span;
    }

    fn lookup(&mut self, id: &Ident) -> Ty {
        match self.get(&id.name) {
            Some(sym) => sym.ty,
            None => {
                self.error(id.span, format!("unknown identifier `{}`", id.name));
                Ty::Error
            }
        }
    }

    fn define(&mut self, name: &str, sym: Symbol, span: Span) {
        let duplicate = self
            .scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false);
        if duplicate {
            self.error(span, format!("`{name}` is already defined in this scope"));
        }
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), sym);
    }

    fn get(&self, name: &str) -> Option<Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym.clone());
            }
        }
        None
    }

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }
}

// Store function parameter types so calls can be checked.
impl Checker {
    fn remember_fn(&mut self, name: &str, params: Vec<Ty>) {
        self.fn_sigs.insert(name.to_string(), params);
    }
}

// We need fn_sigs on Checker. I'll add it properly by editing the struct...
// I referenced self.fn_sigs but didn't add the field. Fix in a patch.

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}
