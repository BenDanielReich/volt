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
    Ref {
        mutable: bool,
        inner: Box<Ty>,
    },
    Peripheral(String),
    Pin {
        reg: String,
        bit: String,
    },
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
    Bits { signed: bool, width: u8 },
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
            Ty::Int(i) => i.name(),
            Ty::Float(FloatTy::F32) => "f32".into(),
            Ty::Float(FloatTy::F64) => "f64".into(),
            Ty::Pointer(inner) => format!("{}*", inner.name()),
            Ty::Array(inner, n) => format!("{}[{n}]", inner.name()),
            Ty::Struct(n) | Ty::Enum(n) | Ty::Peripheral(n) => n.clone(),
            Ty::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", inner.name())
                } else {
                    format!("&{}", inner.name())
                }
            }
            Ty::Pin { reg, bit } => format!("pin({reg}.{bit})"),
            Ty::IntLit => "{{integer}}".into(),
            Ty::FloatLit => "{{float}}".into(),
            Ty::Error => "{{error}}".into(),
        }
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Ty::Int(_) | Ty::IntLit)
    }

    pub fn int_width(&self) -> Option<u8> {
        match self {
            Ty::Int(i) => Some(i.width()),
            _ => None,
        }
    }

    pub fn is_signed_int(&self) -> bool {
        match self {
            Ty::Int(i) => i.is_signed(),
            _ => false,
        }
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
    pub fn name(self) -> String {
        match self {
            Self::Int => "int".into(),
            Self::I8 => "i8".into(),
            Self::I16 => "i16".into(),
            Self::I32 => "i32".into(),
            Self::I64 => "i64".into(),
            Self::Isize => "isize".into(),
            Self::U8 => "u8".into(),
            Self::U16 => "u16".into(),
            Self::U32 => "u32".into(),
            Self::U64 => "u64".into(),
            Self::Usize => "usize".into(),
            Self::Bits {
                signed: false,
                width,
            } => format!("u{width}"),
            Self::Bits {
                signed: true,
                width,
            } => format!("i{width}"),
        }
    }

    pub fn width(self) -> u8 {
        match self {
            Self::Int => 32,
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 => 32,
            Self::I64 | Self::U64 => 64,
            Self::Isize | Self::Usize => 64,
            Self::Bits { width, .. } => width,
        }
    }

    pub fn is_signed(self) -> bool {
        matches!(
            self,
            Self::Int
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::Isize
                | Self::Bits { signed: true, .. }
        )
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
    pub c_name: String,
    pub params: Vec<(String, Ty)>,
    pub return_ty: Ty,
    pub is_extern: bool,
    pub is_internal: bool,
    pub is_comptime: bool,
    pub is_async: bool,
    pub method_of: Option<String>,
    pub effects: Effects,
    pub attrs: Vec<Attribute>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Effects {
    pub blocking: bool,
    pub alloc: bool,
    pub isr: bool,
}

#[derive(Debug, Clone)]
pub struct CallRewrite {
    pub span: Span,
    pub callee: String,
    pub addr_of_recv: bool,
}

#[derive(Debug, Clone)]
pub struct BitFieldDef {
    pub name: String,
    pub start: u8,
    pub width: u8,
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
    pub bits: Vec<BitFieldDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PinDef {
    pub name: String,
    pub reg: String,
    pub bit: String,
    pub start: u8,
    pub width: u8,
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
    pub pins: Vec<PinDef>,
    pub aliases: Vec<(String, Ty)>,
    pub peripherals: Vec<String>,
    pub c_includes: Vec<CInclude>,
    pub call_rewrites: Vec<CallRewrite>,
    pub target_name: String,
    pub target_has_fpu: bool,
    pub target_pointer_width: u8,
    pub target_ram: u32,
    pub target_f_cpu: u64,
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
    Pin,
    Peripheral {
        moved: bool,
    },
    Fn,
    Struct,
    Enum,
    Trait,
    TypeAlias,
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
    aliases: HashMap<String, Ty>,
    regs: HashMap<String, RegDef>,
    fn_sigs: HashMap<String, Vec<Ty>>,
    fn_effects: HashMap<String, Effects>,
    methods: HashMap<(String, String), String>,
    traits: HashMap<String, Vec<String>>,
    trait_impls: HashMap<(String, String), ()>,
    call_rewrites: Vec<CallRewrite>,
    current_effects: Effects,
    current_async: bool,
    impl_self: Option<Ty>,
    mut_borrows: Vec<String>,
    target: crate::target::Target,
    pub diagnostics: Vec<Diagnostic>,
}

impl Checker {
    pub fn new() -> Self {
        Self::with_target(crate::target::HOST)
    }

    pub fn with_target(target: crate::target::Target) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
            enums: HashMap::new(),
            consts: HashMap::new(),
            aliases: HashMap::new(),
            regs: HashMap::new(),
            fn_sigs: HashMap::new(),
            fn_effects: HashMap::new(),
            methods: HashMap::new(),
            traits: HashMap::new(),
            trait_impls: HashMap::new(),
            call_rewrites: Vec::new(),
            current_effects: Effects::default(),
            current_async: false,
            impl_self: None,
            mut_borrows: Vec::new(),
            target,
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
            pins: Vec::new(),
            aliases: Vec::new(),
            peripherals: Vec::new(),
            c_includes: Vec::new(),
            call_rewrites: Vec::new(),
            target_name: self.target.name.to_string(),
            target_has_fpu: self.target.has_fpu,
            target_pointer_width: self.target.pointer_width,
            target_ram: self.target.ram_bytes,
            target_f_cpu: self.target.f_cpu,
        };

        for module in modules {
            for item in &module.items {
                match item {
                    Item::Use(_) | Item::Include(_) => {}
                    Item::Struct(s) => self.collect_struct(s),
                    Item::Enum(e) => self.collect_enum(e),
                    Item::TypeAlias(t) => self.collect_alias(t),
                    Item::Trait(t) => self.collect_trait(t),
                    Item::Peripheral(p) => {
                        self.collect_peripheral(p);
                        program.peripherals.push(p.name.name.clone());
                    }
                    _ => {}
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
                            program.regs.push(def.clone());
                            self.regs.insert(def.name.clone(), def);
                        }
                    }
                    Item::Fn(f) => {
                        if let Some(def) = self.collect_fn_sig(f, None) {
                            program.funcs.push(def);
                        }
                    }
                    Item::Impl(i) => self.collect_impl(i, &mut program),
                    Item::Pin(p) => {
                        if let Some(def) = self.collect_pin(p) {
                            program.pins.push(def);
                        }
                    }
                    _ => {}
                }
            }
        }

        self.inject_board_consts();

        for func in &mut program.funcs {
            if let Some(mut body) = func.body.take() {
                self.push();
                self.current_effects = func.effects;
                self.current_async = func.is_async;
                self.impl_self = func
                    .method_of
                    .as_ref()
                    .map(|n| Ty::Struct(n.clone()));
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
                self.check_block_mut(&mut body, &func.return_ty);
                self.pop();
                func.body = Some(body);
            }
        }

        program.structs = self.structs.values().cloned().collect();
        program.enums = self.enums.values().cloned().collect();
        program.consts = self.consts.values().cloned().collect();
        program.aliases = self
            .aliases
            .iter()
            .map(|(n, t)| (n.clone(), t.clone()))
            .collect();
        program.call_rewrites = self.call_rewrites.clone();
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

    fn collect_alias(&mut self, t: &TypeAliasItem) {
        let ty = self.resolve_type(&t.ty);
        self.aliases.insert(t.name.name.clone(), ty.clone());
        self.define(
            &t.name.name,
            Symbol {
                ty,
                kind: SymbolKind::TypeAlias,
            },
            t.name.span,
        );
    }

    fn collect_trait(&mut self, t: &TraitItem) {
        let methods: Vec<String> = t.methods.iter().map(|m| m.name.name.clone()).collect();
        self.traits.insert(t.name.name.clone(), methods);
        self.define(
            &t.name.name,
            Symbol {
                ty: Ty::Void,
                kind: SymbolKind::Trait,
            },
            t.name.span,
        );
    }

    fn collect_peripheral(&mut self, p: &PeripheralItem) {
        self.define(
            &p.name.name,
            Symbol {
                ty: Ty::Peripheral(p.name.name.clone()),
                kind: SymbolKind::Peripheral { moved: false },
            },
            p.name.span,
        );
        // stored on Program in check_modules
    }

    fn collect_impl(&mut self, i: &ImplItem, program: &mut Program) {
        let ty = self.resolve_type(&i.ty);
        let type_name = match &ty {
            Ty::Struct(n) | Ty::Enum(n) | Ty::Peripheral(n) => n.clone(),
            other => {
                self.error(i.span, format!("cannot impl for `{}`", other.name()));
                return;
            }
        };
        if let Some(tr) = &i.trait_name {
            self.trait_impls
                .insert((tr.name.clone(), type_name.clone()), ());
            if let Some(required) = self.traits.get(&tr.name).cloned() {
                for req in &required {
                    if !i.methods.iter().any(|m| &m.name.name == req) {
                        self.error(
                            i.span,
                            format!("impl of `{tr}` for `{type_name}` missing method `{req}`"),
                        );
                    }
                }
            } else {
                self.error(tr.span, format!("unknown trait `{}`", tr.name));
            }
        }
        let prev = self.impl_self.replace(ty.clone());
        for method in &i.methods {
            if let Some(mut def) = self.collect_fn_sig(method, Some(type_name.clone())) {
                let mangled = format!("{type_name}_{}", method.name.name);
                def.c_name = mangled.clone();
                self.fn_sigs.insert(
                    mangled.clone(),
                    def.params.iter().map(|(_, t)| t.clone()).collect(),
                );
                self.fn_effects.insert(mangled.clone(), def.effects);
                self.methods
                    .insert((type_name.clone(), method.name.name.clone()), mangled);
                program.funcs.push(def);
            }
        }
        self.impl_self = prev;
    }

    fn collect_pin(&mut self, p: &PinItem) -> Option<PinDef> {
        let Some(reg) = self.regs.get(&p.reg.name).cloned() else {
            self.error(
                p.reg.span,
                format!("unknown register `{}` in pin", p.reg.name),
            );
            return None;
        };
        let Some(bit) = reg.bits.iter().find(|b| b.name == p.bit.name) else {
            self.error(
                p.bit.span,
                format!("no bit `{}` on register `{}`", p.bit.name, p.reg.name),
            );
            return None;
        };
        let def = PinDef {
            name: p.name.name.clone(),
            reg: p.reg.name.clone(),
            bit: p.bit.name.clone(),
            start: bit.start,
            width: bit.width,
            span: p.span,
        };
        self.define(
            &p.name.name,
            Symbol {
                ty: Ty::Pin {
                    reg: def.reg.clone(),
                    bit: def.bit.clone(),
                },
                kind: SymbolKind::Pin,
            },
            p.name.span,
        );
        Some(def)
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
        let mut bits = Vec::new();
        for b in &r.bits {
            let start = self.eval_const(&b.start).unwrap_or(0) as u8;
            let width = if let Some(end) = &b.width {
                let end = self.eval_const(end).unwrap_or(start as i128) as u8;
                end.saturating_sub(start).saturating_add(1).max(1)
            } else {
                1
            };
            bits.push(BitFieldDef {
                name: b.name.name.clone(),
                start,
                width,
            });
        }
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
            bits,
            span: r.span,
        })
    }

    fn collect_fn_sig(&mut self, f: &FnItem, method_of: Option<String>) -> Option<FuncDef> {
        let prev_self = self.impl_self.clone();
        if let Some(ref n) = method_of {
            self.impl_self = Some(Ty::Struct(n.clone()));
        }
        let return_ty = self.resolve_type(&f.return_ty);
        let params = f
            .params
            .iter()
            .map(|p| {
                let ty = self.resolve_type(&p.ty);
                (p.name.name.clone(), ty)
            })
            .collect::<Vec<_>>();
        self.impl_self = prev_self;
        let mut effects = effects_from_attrs(&f.attrs);
        if f.attrs.iter().any(|a| a.name.name == "interrupt") {
            effects.isr = true;
        }
        let c_name = f.name.name.clone();
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
        self.fn_effects.insert(f.name.name.clone(), effects);
        Some(FuncDef {
            name: f.name.name.clone(),
            c_name,
            params,
            return_ty,
            is_extern: f.is_extern,
            is_internal: f.is_internal,
            is_comptime: f.is_comptime,
            is_async: f.is_async,
            method_of,
            effects,
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

    fn check_block_mut(&mut self, block: &mut Block, return_ty: &Ty) {
        self.push();
        for stmt in &mut block.stmts {
            self.check_stmt_mut(stmt, return_ty);
        }
        self.pop();
    }

    fn check_stmt_mut(&mut self, stmt: &mut Stmt, return_ty: &Ty) {
        match stmt {
            Stmt::When(w) => {
                let cond = self.check_expr(&w.cond);
                let val = self.eval_const(&w.cond);
                if !cond.is_bool_like() && !cond.is_int() && cond != Ty::Error {
                    self.error(w.cond.span, format!("when condition has type `{}`", cond.name()));
                }
                let taken = val.map(|v| v != 0).unwrap_or(true);
                w.taken = Some(taken);
                if taken {
                    self.check_block_mut(&mut w.then_branch, return_ty);
                } else if let Some(e) = &mut w.else_branch {
                    self.check_block_mut(e, return_ty);
                }
            }
            Stmt::If(i) => {
                let cond = self.check_expr(&i.cond);
                if !cond.is_bool_like() && cond != Ty::Error {
                    self.error(i.cond.span, format!("condition has type `{}`", cond.name()));
                }
                self.check_stmt_mut(&mut i.then_branch, return_ty);
                if let Some(e) = &mut i.else_branch {
                    self.check_stmt_mut(e, return_ty);
                }
            }
            Stmt::While(w) => {
                let cond = self.check_expr(&w.cond);
                if !cond.is_bool_like() && cond != Ty::Error {
                    self.error(w.cond.span, format!("condition has type `{}`", cond.name()));
                }
                self.check_stmt_mut(&mut w.body, return_ty);
            }
            Stmt::For(f) => {
                self.push();
                if let Some(init) = &mut f.init {
                    self.check_stmt_mut(init, return_ty);
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
                self.check_stmt_mut(&mut f.body, return_ty);
                self.pop();
            }
            Stmt::Loop(body) => self.check_block_mut(body, return_ty),
            Stmt::Match(m) => {
                self.check_match(m, return_ty);
            }
            Stmt::Block(b) => self.check_block_mut(b, return_ty),
            other => self.check_stmt(other, return_ty),
        }
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
            Stmt::Match(m) => self.check_match(m, return_ty),
            Stmt::When(w) => {
                let _ = self.check_expr(&w.cond);
                self.check_block(&w.then_branch, return_ty);
                if let Some(e) = &w.else_branch {
                    self.check_block(e, return_ty);
                }
            }
            Stmt::Await(expr, span) => {
                if !self.current_async {
                    self.error(*span, "`await` is only allowed in `async` functions");
                }
                let _ = self.check_expr(expr);
            }
            Stmt::Asm(a) => {
                for b in &a.binds {
                    let _ = self.check_expr(&b.value);
                }
            }
        }
    }

    fn check_match(&mut self, m: &MatchStmt, return_ty: &Ty) {
        let scrut = self.check_expr(&m.scrutinee);
        let mut saw_wild = false;
        for arm in &m.arms {
            match &arm.pattern {
                Pattern::Wildcard(_) => saw_wild = true,
                Pattern::Ident(id) => {
                    let _ = self.lookup(id);
                }
                Pattern::Enum(en, var) => {
                    if let Some(def) = self.enums.get(&en.name) {
                        if !def.variants.iter().any(|(n, _)| n == &var.name) {
                            self.error(
                                var.span,
                                format!("`{}` has no choice called `{}`", en.name, var.name),
                            );
                        }
                        if let Ty::Enum(got) = &scrut {
                            if got != &en.name && scrut != Ty::Error {
                                self.error(
                                    en.span,
                                    format!("this branch is for `{en}`, but you're matching a `{got}`"),
                                );
                            }
                        }
                    } else {
                        self.error(en.span, format!("`{}` isn't a known list of names", en.name));
                    }
                }
                Pattern::Int(_, span) => {
                    if !scrut.is_int() && scrut != Ty::Error && !matches!(scrut, Ty::Enum(_)) {
                        self.error(*span, "integer pattern requires an integer scrutinee");
                    }
                }
            }
            self.check_stmt(&arm.body, return_ty);
        }
        if !saw_wild && matches!(scrut, Ty::Enum(_)) {
            self.diagnostics.push(Diagnostic::warning(
                "match on enum has no `_` arm; add one or cover every variant",
                m.span,
            ));
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Ty {
        match &expr.kind {
            ExprKind::Int(lit) => {
                if let Some((signed, width)) = crate::parser::int_bit_suffix(&lit.raw) {
                    Ty::Int(IntTy::Bits { signed, width })
                } else if let Some(prim) = lit.suffix {
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
            ExprKind::Await(inner) => {
                if !self.current_async {
                    self.error(expr.span, "`await` is only allowed in `async` functions");
                }
                self.check_expr(inner)
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
                Ty::Pointer(inner) | Ty::Ref { inner, .. } => *inner,
                Ty::Error => Ty::Error,
                other => {
                    self.error(inner.span, format!("cannot dereference `{}`", other.name()));
                    Ty::Error
                }
            },
            UnOp::AddrOf => Ty::Ref {
                mutable: false,
                inner: Box::new(ty),
            },
            UnOp::AddrOfMut => {
                self.check_assignable(inner);
                if let ExprKind::Ident(id) = &inner.kind {
                    if self.mut_borrows.iter().any(|n| n == &id.name) {
                        self.error(
                            inner.span,
                            format!("cannot borrow `{id}` as mut more than once"),
                        );
                    } else {
                        self.mut_borrows.push(id.name.clone());
                    }
                }
                Ty::Ref {
                    mutable: true,
                    inner: Box::new(ty),
                }
            }
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
        if let ExprKind::Field(base, method) = &callee.kind {
            return self.check_method_call(base, method, args, span);
        }
        let ExprKind::Ident(name) = &callee.kind else {
            self.error(callee.span, "can only call a named function or method");
            for a in args {
                self.check_expr(a);
            }
            return Ty::Error;
        };
        if name.name == "target" {
            self.error(name.span, "`target` is not a function");
            return Ty::Error;
        }
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
        self.check_effects(&name.name, span);
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
                self.check_arg_move(arg, expected);
            }
        } else {
            for a in args {
                self.check_expr(a);
            }
        }
        sym.ty
    }

    fn check_method_call(
        &mut self,
        base: &Expr,
        method: &Ident,
        args: &[Expr],
        span: Span,
    ) -> Ty {
        let recv = self.check_expr(base);
        let type_name = match &recv {
            Ty::Struct(n) | Ty::Enum(n) | Ty::Peripheral(n) => n.clone(),
            Ty::Pointer(inner) | Ty::Ref { inner, .. } => match inner.as_ref() {
                Ty::Struct(n) | Ty::Enum(n) | Ty::Peripheral(n) => n.clone(),
                other => {
                    self.error(base.span, format!("`{}` has no methods", other.name()));
                    return Ty::Error;
                }
            },
            other => {
                self.error(base.span, format!("`{}` has no methods", other.name()));
                return Ty::Error;
            }
        };
        let Some(mangled) = self.methods.get(&(type_name.clone(), method.name.clone())).cloned() else {
            self.error(
                method.span,
                format!("no method `{}` on `{type_name}`", method.name),
            );
            for a in args {
                self.check_expr(a);
            }
            return Ty::Error;
        };
        let addr_of_recv = matches!(recv, Ty::Struct(_) | Ty::Enum(_) | Ty::Peripheral(_));
        self.call_rewrites.push(CallRewrite {
            span,
            callee: mangled.clone(),
            addr_of_recv,
        });
        self.check_effects(&mangled, span);
        if let Some(sig) = self.fn_params(&method.name).or_else(|| self.fn_sigs.get(&mangled).cloned())
        {
            let expected_args = sig.len().saturating_sub(1);
            if expected_args != args.len() {
                self.error(
                    span,
                    format!(
                        "method `{}` expects {expected_args} argument(s), found {}",
                        method.name,
                        args.len()
                    ),
                );
            }
            for (arg, expected) in args.iter().zip(sig.iter().skip(1)) {
                let got = self.check_expr(arg);
                self.expect_type(expected, &got, arg.span);
            }
        } else {
            for a in args {
                self.check_expr(a);
            }
        }
        self.get(&method.name)
            .or_else(|| {
                self.fn_sigs.get(&mangled).and_then(|_| {
                    self.scopes[0].get(&method.name).cloned()
                })
            })
            .map(|s| s.ty)
            .unwrap_or(Ty::Void)
    }

    fn check_effects(&mut self, name: &str, span: Span) {
        let Some(callee) = self.fn_effects.get(name).copied() else {
            return;
        };
        if self.current_effects.isr && callee.blocking {
            self.error(
                span,
                format!("cannot call blocking function `{name}` from an ISR"),
            );
        }
        if self.current_effects.isr && callee.alloc {
            self.error(span, format!("cannot allocate from ISR (`{name}`)"));
        }
    }

    fn check_arg_move(&mut self, arg: &Expr, expected: &Ty) {
        if !matches!(expected, Ty::Peripheral(_)) {
            return;
        }
        if let ExprKind::Ident(id) = &arg.kind {
            if let Some(sym) = self.get(&id.name) {
                if let SymbolKind::Peripheral { moved: true } = sym.kind {
                    self.error(id.span, format!("peripheral `{id}` has already been moved"));
                }
            }
            self.mark_moved(&id.name);
        }
    }

    fn mark_moved(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(sym) = scope.get_mut(name) {
                if let SymbolKind::Peripheral { moved } = &mut sym.kind {
                    *moved = true;
                }
                return;
            }
        }
    }

    fn fn_params(&self, name: &str) -> Option<Vec<Ty>> {
        // Stored on the root scope as a synthetic symbol? We stash params in consts-like map.
        self.fn_sigs.get(name).cloned()
    }

    fn check_field(&mut self, base: &Expr, field: &Ident) -> Ty {
        if let ExprKind::Ident(name) = &base.kind {
            if name.name == "target" {
                return match field.name.as_str() {
                    "has_fpu" | "avr" | "arm" | "rp2040" | "stm32" | "esp32" | "esp8266" | "esp" => {
                        Ty::Bool
                    }
                    "pointer_width" | "ram" | "f_cpu" | "led" => Ty::IntLit,
                    other => {
                        self.error(
                            field.span,
                            format!("unknown target field `{other}`"),
                        );
                        Ty::Error
                    }
                };
            }
            if let Some(en) = self.enums.get(&name.name).cloned() {
                if en.variants.iter().any(|(n, _)| n == &field.name) {
                    return Ty::Enum(en.name);
                }
                self.error(
                    field.span,
                    format!("`{}` has no choice called `{}`", name.name, field.name),
                );
                return Ty::Error;
            }
            if let Some(reg) = self.regs.get(&name.name) {
                if let Some(bit) = reg.bits.iter().find(|b| b.name == field.name) {
                    return Ty::Int(IntTy::Bits {
                        signed: false,
                        width: bit.width,
                    });
                }
            }
        }
        let mut base_ty = self.check_expr(base);
        if let Ty::Pointer(inner) | Ty::Ref { inner, .. } = base_ty {
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
            TypeKind::Bits { signed, width } => Ty::Int(IntTy::Bits {
                signed: *signed,
                width: *width,
            }),
            TypeKind::SelfTy => self.impl_self.clone().unwrap_or_else(|| {
                self.error(ty.span, "`self` type used outside an impl");
                Ty::Error
            }),
            TypeKind::Named(name) => {
                if let Some(aliased) = self.aliases.get(&name.name).cloned() {
                    return aliased;
                }
                if self.structs.contains_key(&name.name) {
                    Ty::Struct(name.name.clone())
                } else if self.enums.contains_key(&name.name) {
                    Ty::Enum(name.name.clone())
                } else if let Some(sym) = self.get(&name.name) {
                    match sym.kind {
                        SymbolKind::Struct => Ty::Struct(name.name.clone()),
                        SymbolKind::Enum => Ty::Enum(name.name.clone()),
                        SymbolKind::Peripheral { .. } => Ty::Peripheral(name.name.clone()),
                        SymbolKind::TypeAlias => self
                            .aliases
                            .get(&name.name)
                            .cloned()
                            .unwrap_or(Ty::Error),
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
            TypeKind::Ref { mutable, inner } => Ty::Ref {
                mutable: *mutable,
                inner: Box::new(self.resolve_type(inner)),
            },
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
        if expected.is_int() && matches!(got, Ty::Enum(_) | Ty::Pin { .. }) {
            return;
        }
        if got.is_int() && matches!(expected, Ty::Enum(_) | Ty::Pin { .. }) {
            return;
        }
        if let (Ty::Ref { inner: a, .. }, Ty::Pointer(b)) = (expected, got) {
            if a == b {
                return;
            }
        }
        if let (Ty::Pointer(a), Ty::Ref { inner: b, .. }) = (expected, got) {
            if a == b {
                return;
            }
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
                    BinOp::Add | BinOp::WrapAdd => a.wrapping_add(b),
                    BinOp::Sub | BinOp::WrapSub => a.wrapping_sub(b),
                    BinOp::Mul | BinOp::WrapMul => a.wrapping_mul(b),
                    BinOp::SatAdd => a.saturating_add(b),
                    BinOp::SatSub => a.saturating_sub(b),
                    BinOp::SatMul => a.saturating_mul(b),
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
                    if name.name == "target" {
                        return Some(match field.name.as_str() {
                            "has_fpu" => i128::from(self.target.has_fpu),
                            "avr" => i128::from(self.target.family.avr()),
                            "arm" => i128::from(self.target.family.arm()),
                            "rp2040" => i128::from(self.target.family.rp2040()),
                            "stm32" => i128::from(self.target.family.stm32()),
                            "esp32" => i128::from(self.target.family.esp32()),
                            "esp8266" => i128::from(self.target.family.esp8266()),
                            "esp" => i128::from(self.target.family.esp()),
                            "pointer_width" => i128::from(self.target.pointer_width),
                            "ram" => i128::from(self.target.ram_bytes),
                            "f_cpu" => self.target.f_cpu as i128,
                            "led" => i128::from(self.target.led),
                            _ => {
                                self.error(field.span, format!("unknown target field `{}`", field.name));
                                return None;
                            }
                        });
                    }
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
        match kind {
            ExprKind::Ident(id) => match self.get(&id.name).map(|s| s.kind) {
                Some(SymbolKind::Const { .. }) | Some(SymbolKind::Local { is_const: true }) => {
                    self.error(id.span, format!("cannot assign to const `{}`", id.name));
                }
                Some(SymbolKind::Peripheral { .. }) => {
                    self.error(id.span, format!("cannot assign to peripheral `{}`", id.name));
                }
                _ => {}
            },
            ExprKind::Unary(UnOp::Deref, inner) => {
                let ty = self.check_expr(inner);
                if let Ty::Ref { mutable: false, .. } = ty {
                    self.error(span, "cannot assign through shared reference `&T`");
                }
            }
            _ => {}
        }
    }

    fn lookup(&mut self, id: &Ident) -> Ty {
        if id.name == "target" {
            return Ty::Void;
        }
        match self.get(&id.name) {
            Some(sym) => {
                if let SymbolKind::Peripheral { moved: true } = sym.kind {
                    self.error(id.span, format!("peripheral `{}` has already been moved", id.name));
                }
                sym.ty
            }
            None => {
                self.error(id.span, format!("unknown identifier `{}`", id.name));
                Ty::Error
            }
        }
    }

    fn inject_board_consts(&mut self) {
        let Some(board) = crate::board::find_board(self.target.name) else {
            return;
        };
        self.inject_u8("LED_BUILTIN", board.target.led);
        for i in 0..board.analog_count {
            self.inject_u8(
                &format!("A{i}"),
                board.analog_base.saturating_add(i),
            );
        }
    }

    fn inject_u8(&mut self, name: &str, value: u8) {
        if self.get(name).is_some() {
            return;
        }
        let ty = Ty::Int(IntTy::U8);
        let value = i128::from(value);
        self.define(
            name,
            Symbol {
                ty: ty.clone(),
                kind: SymbolKind::Const { value },
            },
            Span::dummy(),
        );
        self.consts.insert(
            name.to_string(),
            ConstDef {
                name: name.to_string(),
                ty,
                value,
                span: Span::dummy(),
            },
        );
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

fn effects_from_attrs(attrs: &[Attribute]) -> Effects {
    let mut e = Effects::default();
    for a in attrs {
        match a.name.name.as_str() {
            "blocking" => e.blocking = true,
            "alloc" => e.alloc = true,
            "isr" | "interrupt" => e.isr = true,
            _ => {}
        }
    }
    e
}

// We need fn_sigs on Checker. I'll add it properly by editing the struct...
// I referenced self.fn_sigs but didn't add the field. Fix in a patch.

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}
