//! Worst-case stack, ISR, and peripheral report emitted into generated C.

use crate::ast::{ExprKind, Stmt};
use crate::sema::{FuncDef, Program, Ty};
use crate::target::Target;

pub fn render(program: &Program, target: &Target) -> String {
    let mut out = String::new();
    out.push_str(&format!("  target: {}\n", target.name));
    out.push_str(&format!(
        "  pointer_width: {}  fpu: {}  ram: {} B  f_cpu: {}\n",
        target.pointer_width, target.has_fpu, target.ram_bytes, target.f_cpu
    ));

    let static_bytes: u64 = program.statics.iter().map(|s| size_of_ty(&s.ty)).sum();
    out.push_str(&format!("  static data (approx): {static_bytes} bytes\n"));

    out.push_str("  functions:\n");
    for f in &program.funcs {
        if f.body.is_none() {
            continue;
        }
        let stack = stack_of(f);
        let mut tags = Vec::new();
        if f.effects.isr {
            tags.push("isr");
        }
        if f.effects.blocking {
            tags.push("blocking");
        }
        if f.is_async {
            tags.push("async");
        }
        if f.is_internal {
            tags.push("internal");
        }
        let tag = if tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", tags.join(", "))
        };
        out.push_str(&format!(
            "    {}  stack≤{stack} B{tag}\n",
            f.c_name
        ));
    }

    let isrs: Vec<_> = program
        .funcs
        .iter()
        .filter(|f| f.effects.isr)
        .map(|f| f.c_name.as_str())
        .collect();
    if !isrs.is_empty() {
        out.push_str(&format!("  ISRs: {}\n", isrs.join(", ")));
    }
    if !program.peripherals.is_empty() {
        out.push_str(&format!(
            "  peripherals: {}\n",
            program.peripherals.join(", ")
        ));
    }
    if !program.regs.is_empty() {
        out.push_str(&format!(
            "  registers: {}\n",
            program
                .regs
                .iter()
                .map(|r| r.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out
}

fn stack_of(f: &FuncDef) -> u64 {
    let Some(body) = &f.body else {
        return 0;
    };
    let mut n = 0u64;
    walk_stmt_stack(&mut n, body.stmts.iter());
    for (_, ty) in &f.params {
        n += size_of_ty(ty);
    }
    n
}

fn walk_stmt_stack<'a>(n: &mut u64, stmts: impl Iterator<Item = &'a Stmt>) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => *n += size_of_ast_local(&l.ty),
            Stmt::Block(b) | Stmt::Loop(b) => walk_stmt_stack(n, b.stmts.iter()),
            Stmt::If(i) => {
                walk_one(n, &i.then_branch);
                if let Some(e) = &i.else_branch {
                    walk_one(n, e);
                }
            }
            Stmt::While(w) => walk_one(n, &w.body),
            Stmt::For(f) => {
                if let Some(init) = &f.init {
                    walk_one(n, init);
                }
                walk_one(n, &f.body);
            }
            Stmt::Match(m) => {
                for arm in &m.arms {
                    walk_one(n, &arm.body);
                }
            }
            Stmt::When(w) => {
                walk_stmt_stack(n, w.then_branch.stmts.iter());
                if let Some(e) = &w.else_branch {
                    walk_stmt_stack(n, e.stmts.iter());
                }
            }
            _ => {}
        }
    }
}

fn walk_one(n: &mut u64, stmt: &Stmt) {
    walk_stmt_stack(n, std::iter::once(stmt));
}

fn size_of_ast_local(ty: &crate::ast::Type) -> u64 {
    match &ty.kind {
        crate::ast::TypeKind::Primitive(p) => match p {
            crate::ast::PrimitiveTy::Bool | crate::ast::PrimitiveTy::U8 | crate::ast::PrimitiveTy::I8 => 1,
            crate::ast::PrimitiveTy::U16 | crate::ast::PrimitiveTy::I16 => 2,
            crate::ast::PrimitiveTy::F64 => 8,
            crate::ast::PrimitiveTy::U64 | crate::ast::PrimitiveTy::I64 => 8,
            _ => 4,
        },
        crate::ast::TypeKind::Bits { width, .. } => {
            if *width <= 8 {
                1
            } else if *width <= 16 {
                2
            } else if *width <= 32 {
                4
            } else {
                8
            }
        }
        crate::ast::TypeKind::Pointer(_) | crate::ast::TypeKind::Ref { .. } => 8,
        crate::ast::TypeKind::Array(inner, len) => {
            let count = match &len.kind {
                ExprKind::Int(lit) => lit.value as u64,
                _ => 1,
            };
            size_of_ast_local(inner) * count
        }
        _ => 4,
    }
}

fn size_of_ty(ty: &Ty) -> u64 {
    match ty {
        Ty::Void | Ty::Error => 0,
        Ty::Bool => 1,
        Ty::Int(i) => {
            let w = i.width();
            if w <= 8 {
                1
            } else if w <= 16 {
                2
            } else if w <= 32 {
                4
            } else {
                8
            }
        }
        Ty::Float(crate::sema::FloatTy::F32) => 4,
        Ty::Float(crate::sema::FloatTy::F64) => 8,
        Ty::Pointer(_) | Ty::Ref { .. } => 8,
        Ty::Array(inner, n) => size_of_ty(inner) * n,
        Ty::Struct(_) | Ty::Enum(_) | Ty::Peripheral(_) | Ty::Pin { .. } => 4,
        Ty::IntLit => 4,
        Ty::FloatLit => 8,
    }
}
