use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::span::{Ident, Span};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn parse_module(mut self) -> (Module, Vec<Diagnostic>) {
        let start = self.peek().span;
        let mut name = None;
        if self.eat(TokenKind::Module) {
            let mut ident = self.expect_ident("module name");
            while self.eat(TokenKind::Dot) {
                let seg = self.expect_ident("module path segment");
                ident.name.push('.');
                ident.name.push_str(&seg.name);
                ident.span = ident.span.merge(seg.span);
            }
            name = Some(ident);
            self.expect(TokenKind::Semicolon, "`;` after module name");
        }
        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            match self.parse_item() {
                Some(item) => items.push(item),
                None => {
                    if self.at(TokenKind::Eof) {
                        break;
                    }
                    self.error_here("expected an item");
                    self.sync_item();
                }
            }
        }
        let end = self.peek().span;
        (
            Module {
                name,
                items,
                span: start.merge(end),
            },
            self.diagnostics,
        )
    }

    fn parse_item(&mut self) -> Option<Item> {
        if self.at(TokenKind::Hash)
            && self.peek_n(1).kind == TokenKind::Ident
            && self.peek_n(1).lexeme == "include"
        {
            return self.parse_include().map(Item::Include);
        }
        let attrs = self.parse_attrs();
        if self.at(TokenKind::Use) {
            return self.parse_use().map(Item::Use);
        }
        if self.at(TokenKind::Struct) {
            return self.parse_struct().map(Item::Struct);
        }
        if self.at(TokenKind::Enum) {
            return self.parse_enum().map(Item::Enum);
        }
        if self.at(TokenKind::Const) {
            return self.parse_const().map(Item::Const);
        }
        if self.at(TokenKind::Reg) {
            return self.parse_reg().map(Item::Reg);
        }

        let is_extern = self.eat(TokenKind::Extern);
        let is_static = self.eat(TokenKind::Static);

        let start = self.peek().span;
        if self.at(TokenKind::Function) {
            let fn_span = self.bump().span;
            let name = self.expect_ident("function name");
            if !self.at(TokenKind::LParen) {
                self.error_here("expected `(` after function name");
            }
            let params = self.parse_params();
            let body = if self.at(TokenKind::LBrace) {
                Some(self.parse_block())
            } else {
                self.expect(TokenKind::Semicolon, "`;` after function prototype");
                None
            };
            let span = start.merge(self.prev_span());
            return Some(Item::Fn(FnItem {
                attrs,
                is_extern,
                return_ty: Type::void(fn_span),
                name,
                params,
                body,
                span,
            }));
        }
        let ty = self.parse_type()?;
        let name = self.expect_ident("item name");

        if self.at(TokenKind::LParen) {
            let params = self.parse_params();
            let body = if self.at(TokenKind::LBrace) {
                Some(self.parse_block())
            } else {
                self.expect(TokenKind::Semicolon, "`;` after function prototype");
                None
            };
            let span = start.merge(self.prev_span());
            return Some(Item::Fn(FnItem {
                attrs,
                is_extern,
                return_ty: ty,
                name,
                params,
                body,
                span,
            }));
        }

        if is_static || is_extern || self.at(TokenKind::Eq) || self.at(TokenKind::Semicolon) {
            let value = if self.eat(TokenKind::Eq) {
                Some(self.parse_expr())
            } else {
                None
            };
            self.expect(TokenKind::Semicolon, "`;` after global");
            return Some(Item::Static(StaticItem {
                is_extern,
                ty,
                name,
                value,
                span: start.merge(self.prev_span()),
            }));
        }

        self.error_here("expected `(`, `=`, or `;` after item name");
        None
    }

    fn parse_use(&mut self) -> Option<UseItem> {
        let start = self.peek().span;
        self.bump(); // use
        let mut path = vec![self.expect_ident("module path")];
        while self.eat(TokenKind::Dot) {
            path.push(self.expect_ident("module path segment"));
        }
        self.expect(TokenKind::Semicolon, "`;` after use");
        Some(UseItem {
            path,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_include(&mut self) -> Option<IncludeItem> {
        let start = self.peek().span;
        self.bump(); // #
        self.bump(); // include
        let (path, style) = if self.at(TokenKind::String) {
            let tok = self.bump();
            (unquote_string(&tok.lexeme), IncludeStyle::Quote)
        } else {
            self.expect(TokenKind::Lt, "`<...>` or `\"...\"` after #include");
            let mut path = String::new();
            while !self.at(TokenKind::Gt) && !self.at(TokenKind::Eof) {
                path.push_str(&self.bump().lexeme);
            }
            self.expect(TokenKind::Gt, "`>` after include path");
            (path, IncludeStyle::Angle)
        };
        self.eat(TokenKind::Semicolon);
        if path.is_empty() {
            self.error_here("empty #include path");
            return None;
        }
        Some(IncludeItem {
            path,
            style,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_struct(&mut self) -> Option<StructItem> {
        let start = self.peek().span;
        self.bump(); // struct
        let name = self.expect_ident("struct name");
        self.expect(TokenKind::LBrace, "`{` after struct name");
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let Some(ty) = self.parse_type() else {
                self.sync_item();
                break;
            };
            let fname = self.expect_ident("field name");
            self.expect(TokenKind::Semicolon, "`;` after field");
            fields.push(Field { ty, name: fname });
        }
        self.expect(TokenKind::RBrace, "`}` after struct fields");
        Some(StructItem {
            name,
            fields,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_enum(&mut self) -> Option<EnumItem> {
        let start = self.peek().span;
        self.bump(); // enum
        let name = self.expect_ident("enum name");
        self.expect(TokenKind::LBrace, "`{` after enum name");
        let mut variants = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let vname = self.expect_ident("variant name");
            let value = if self.eat(TokenKind::Eq) {
                Some(self.parse_expr())
            } else {
                None
            };
            variants.push(EnumVariant { name: vname, value });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "`}` after enum variants");
        Some(EnumItem {
            name,
            variants,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_const(&mut self) -> Option<ConstItem> {
        let start = self.peek().span;
        self.bump(); // const
        let ty = self.parse_type()?;
        let name = self.expect_ident("const name");
        self.expect(TokenKind::Eq, "`=` after const name");
        let value = self.parse_expr();
        self.expect(TokenKind::Semicolon, "`;` after const");
        Some(ConstItem {
            ty,
            name,
            value,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_reg(&mut self) -> Option<RegItem> {
        let start = self.peek().span;
        self.bump(); // reg
        let ty = self.parse_type()?;
        let name = self.expect_ident("register name");
        self.expect(TokenKind::At, "`@` and an address after register name");
        let address = self.parse_expr();
        self.expect(TokenKind::Semicolon, "`;` after register");
        Some(RegItem {
            ty,
            name,
            address,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_attrs(&mut self) -> Vec<Attribute> {
        let mut attrs = Vec::new();
        while self.at(TokenKind::Hash) {
            let start = self.peek().span;
            self.bump();
            self.expect(TokenKind::LBracket, "`[` after `#`");
            let name = self.expect_ident("attribute name");
            let value = if self.eat(TokenKind::LParen) {
                let v = if self.at(TokenKind::String) {
                    Some(unquote_string(&self.bump().lexeme))
                } else if self.at(TokenKind::Ident) {
                    Some(self.bump().lexeme)
                } else {
                    None
                };
                self.expect(TokenKind::RParen, "`)` after attribute value");
                v
            } else {
                None
            };
            self.expect(TokenKind::RBracket, "`]` after attribute");
            attrs.push(Attribute {
                name,
                value,
                span: start.merge(self.prev_span()),
            });
        }
        attrs
    }

    fn parse_params(&mut self) -> Vec<Param> {
        self.expect(TokenKind::LParen, "`(`");
        let mut params = Vec::new();
        if self.eat(TokenKind::RParen) {
            return params;
        }
        // C-style empty: (void)
        if (self.at(TokenKind::Void) || self.at(TokenKind::Function))
            && self.peek_n(1).kind == TokenKind::RParen
        {
            self.bump();
            self.bump();
            return params;
        }
        loop {
            let Some(ty) = self.parse_type() else { break };
            let name = self.expect_ident("parameter name");
            params.push(Param { ty, name });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "`)` after parameters");
        params
    }

    fn parse_type(&mut self) -> Option<Type> {
        let start = self.peek().span;
        let mut ty = if self.at(TokenKind::Void) || self.at(TokenKind::Function) {
            let tok = self.bump();
            Type::void(tok.span)
        } else if self.peek().kind.is_type_keyword() {
            let tok = self.bump();
            let prim = primitive_from_kind(tok.kind)?;
            Type {
                kind: TypeKind::Primitive(prim),
                span: tok.span,
            }
        } else if self.at(TokenKind::Ident) {
            let name = self.expect_ident("type name");
            Type {
                span: name.span,
                kind: TypeKind::Named(name),
            }
        } else {
            return None;
        };
        loop {
            if self.eat(TokenKind::Star) {
                let span = start.merge(self.prev_span());
                ty = Type {
                    kind: TypeKind::Pointer(Box::new(ty)),
                    span,
                };
            } else if self.eat(TokenKind::LBracket) {
                let len = self.parse_expr();
                self.expect(TokenKind::RBracket, "`]` after array size");
                let span = start.merge(self.prev_span());
                ty = Type {
                    kind: TypeKind::Array(Box::new(ty), Box::new(len)),
                    span,
                };
            } else {
                break;
            }
        }
        Some(ty)
    }

    fn parse_block(&mut self) -> Block {
        let start = self.peek().span;
        self.expect(TokenKind::LBrace, "`{`");
        let mut stmts = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            stmts.push(self.parse_stmt());
        }
        self.expect(TokenKind::RBrace, "`}`");
        Block {
            stmts,
            span: start.merge(self.prev_span()),
        }
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek().kind {
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Return => {
                let start = self.bump().span;
                let value = if self.at(TokenKind::Semicolon) {
                    None
                } else {
                    Some(self.parse_expr())
                };
                self.expect(TokenKind::Semicolon, "`;` after return");
                Stmt::Return(value, start.merge(self.prev_span()))
            }
            TokenKind::Break => {
                let span = self.bump().span;
                self.expect(TokenKind::Semicolon, "`;` after break");
                Stmt::Break(span)
            }
            TokenKind::Continue => {
                let span = self.bump().span;
                self.expect(TokenKind::Semicolon, "`;` after continue");
                Stmt::Continue(span)
            }
            TokenKind::LBrace => Stmt::Block(self.parse_block()),
            TokenKind::Const => self.parse_const_local(),
            TokenKind::Ident
                if self.peek().lexeme == "loop" && self.peek_n(1).kind == TokenKind::LBrace =>
            {
                self.bump();
                Stmt::Loop(self.parse_block())
            }
            _ => {
                if let Some((ty, name)) = self.try_decl() {
                    let init = if self.eat(TokenKind::Eq) {
                        Some(self.parse_expr())
                    } else {
                        None
                    };
                    self.expect(TokenKind::Semicolon, "`;` after declaration");
                    return Stmt::Local(LocalStmt {
                        span: ty.span.merge(self.prev_span()),
                        is_const: false,
                        ty,
                        name,
                        init,
                    });
                }
                let expr = self.parse_expr();
                if let Some(op) = AssignOp::from_token(self.peek().kind) {
                    self.bump();
                    let rhs = self.parse_expr();
                    self.expect(TokenKind::Semicolon, "`;` after assignment");
                    return Stmt::Assign(AssignStmt {
                        span: expr.span.merge(self.prev_span()),
                        lhs: expr,
                        op,
                        rhs,
                    });
                }
                self.expect(TokenKind::Semicolon, "`;` after expression");
                Stmt::Expr(expr)
            }
        }
    }

    fn try_decl(&mut self) -> Option<(Type, Ident)> {
        let pos = self.pos;
        let diags = self.diagnostics.len();
        let ty = self.parse_type();
        match ty {
            Some(ty) if self.at(TokenKind::Ident) => {
                let name = self.expect_ident("variable name");
                Some((ty, name))
            }
            _ => {
                self.pos = pos;
                self.diagnostics.truncate(diags);
                None
            }
        }
    }

    fn parse_if(&mut self) -> Stmt {
        let start = self.bump().span;
        self.expect(TokenKind::LParen, "`(` after if");
        let cond = self.parse_expr();
        self.expect(TokenKind::RParen, "`)` after if condition");
        let then_branch = Box::new(self.parse_stmt());
        let else_branch = if self.eat(TokenKind::Else) {
            Some(Box::new(self.parse_stmt()))
        } else {
            None
        };
        Stmt::If(IfStmt {
            cond,
            then_branch,
            else_branch,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_const_local(&mut self) -> Stmt {
        let start = self.bump().span;
        let Some(ty) = self.parse_type() else {
            self.error_here("expected type after `const`");
            return Stmt::Block(Block {
                stmts: Vec::new(),
                span: start,
            });
        };
        let name = self.expect_ident("const name");
        self.expect(TokenKind::Eq, "`=` after const name");
        let init = Some(self.parse_expr());
        self.expect(TokenKind::Semicolon, "`;` after const");
        Stmt::Local(LocalStmt {
            span: start.merge(self.prev_span()),
            is_const: true,
            ty,
            name,
            init,
        })
    }

    fn parse_while(&mut self) -> Stmt {
        let start = self.bump().span;
        self.expect(TokenKind::LParen, "`(` after while");
        let cond = self.parse_expr();
        self.expect(TokenKind::RParen, "`)` after while condition");
        let body = Box::new(self.parse_stmt());
        Stmt::While(WhileStmt {
            cond,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_for(&mut self) -> Stmt {
        let start = self.bump().span;
        self.expect(TokenKind::LParen, "`(` after for");
        let init = if self.at(TokenKind::Semicolon) {
            self.bump();
            None
        } else {
            Some(Box::new(self.parse_stmt()))
        };
        let cond = if self.at(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::Semicolon, "`;` after for condition");
        let step = if self.at(TokenKind::RParen) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::RParen, "`)` after for header");
        let body = Box::new(self.parse_stmt());
        Stmt::For(ForStmt {
            init,
            cond,
            step,
            body,
            span: start.merge(self.prev_span()),
        })
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Expr {
        self.parse_bin(Self::parse_and, &[TokenKind::PipePipe], &[BinOp::Or])
    }

    fn parse_and(&mut self) -> Expr {
        self.parse_bin(Self::parse_bit_or, &[TokenKind::AmpAmp], &[BinOp::And])
    }

    fn parse_bit_or(&mut self) -> Expr {
        self.parse_bin(Self::parse_bit_xor, &[TokenKind::Pipe], &[BinOp::BitOr])
    }

    fn parse_bit_xor(&mut self) -> Expr {
        self.parse_bin(Self::parse_bit_and, &[TokenKind::Caret], &[BinOp::BitXor])
    }

    fn parse_bit_and(&mut self) -> Expr {
        self.parse_bin(Self::parse_eq, &[TokenKind::Amp], &[BinOp::BitAnd])
    }

    fn parse_eq(&mut self) -> Expr {
        self.parse_bin(
            Self::parse_rel,
            &[TokenKind::EqEq, TokenKind::NotEq],
            &[BinOp::Eq, BinOp::Ne],
        )
    }

    fn parse_rel(&mut self) -> Expr {
        self.parse_bin(
            Self::parse_shift,
            &[TokenKind::Lt, TokenKind::Le, TokenKind::Gt, TokenKind::Ge],
            &[BinOp::Lt, BinOp::Le, BinOp::Gt, BinOp::Ge],
        )
    }

    fn parse_shift(&mut self) -> Expr {
        self.parse_bin(
            Self::parse_add,
            &[TokenKind::LtLt, TokenKind::GtGt],
            &[BinOp::Shl, BinOp::Shr],
        )
    }

    fn parse_add(&mut self) -> Expr {
        self.parse_bin(
            Self::parse_mul,
            &[TokenKind::Plus, TokenKind::Minus],
            &[BinOp::Add, BinOp::Sub],
        )
    }

    fn parse_mul(&mut self) -> Expr {
        self.parse_bin(
            Self::parse_as,
            &[TokenKind::Star, TokenKind::Slash, TokenKind::Percent],
            &[BinOp::Mul, BinOp::Div, BinOp::Rem],
        )
    }

    fn parse_bin(
        &mut self,
        next: fn(&mut Self) -> Expr,
        kinds: &[TokenKind],
        ops: &[BinOp],
    ) -> Expr {
        let mut left = next(self);
        while let Some(idx) = kinds.iter().position(|&k| self.at(k)) {
            self.bump();
            let right = next(self);
            let span = left.span.merge(right.span);
            left = Expr {
                kind: ExprKind::Binary(ops[idx], Box::new(left), Box::new(right)),
                span,
            };
        }
        left
    }

    fn parse_as(&mut self) -> Expr {
        let mut expr = self.parse_unary();
        while self.eat(TokenKind::As) {
            if let Some(ty) = self.parse_type() {
                let span = expr.span.merge(ty.span);
                expr = Expr {
                    kind: ExprKind::Cast(Box::new(expr), ty),
                    span,
                };
            } else {
                self.error_here("expected type after `as`");
                break;
            }
        }
        expr
    }

    fn parse_unary(&mut self) -> Expr {
        let start = self.peek().span;
        let op = match self.peek().kind {
            TokenKind::Minus => Some(UnOp::Neg),
            TokenKind::Bang => Some(UnOp::Not),
            TokenKind::Tilde => Some(UnOp::BitNot),
            TokenKind::Star => Some(UnOp::Deref),
            TokenKind::Amp => Some(UnOp::AddrOf),
            TokenKind::PlusPlus => Some(UnOp::PreInc),
            TokenKind::MinusMinus => Some(UnOp::PreDec),
            TokenKind::Plus => {
                self.bump();
                return self.parse_unary();
            }
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let expr = self.parse_unary();
            return Expr {
                span: start.merge(expr.span),
                kind: ExprKind::Unary(op, Box::new(expr)),
            };
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if self.eat(TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.at(TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr());
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen, "`)` after call arguments");
                let span = expr.span.merge(self.prev_span());
                expr = Expr {
                    kind: ExprKind::Call(Box::new(expr), args),
                    span,
                };
            } else if self.eat(TokenKind::LBracket) {
                let index = self.parse_expr();
                self.expect(TokenKind::RBracket, "`]` after index");
                let span = expr.span.merge(self.prev_span());
                expr = Expr {
                    kind: ExprKind::Index(Box::new(expr), Box::new(index)),
                    span,
                };
            } else if self.eat(TokenKind::Dot) || self.eat(TokenKind::Arrow) {
                let field = self.expect_ident("field name");
                let span = expr.span.merge(field.span);
                expr = Expr {
                    kind: ExprKind::Field(Box::new(expr), field),
                    span,
                };
            } else if self.at(TokenKind::PlusPlus) {
                let span = expr.span.merge(self.bump().span);
                expr = Expr {
                    kind: ExprKind::Unary(UnOp::PostInc, Box::new(expr)),
                    span,
                };
            } else if self.at(TokenKind::MinusMinus) {
                let span = expr.span.merge(self.bump().span);
                expr = Expr {
                    kind: ExprKind::Unary(UnOp::PostDec, Box::new(expr)),
                    span,
                };
            } else {
                break;
            }
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Integer => {
                self.bump();
                let (value, suffix) = parse_int_literal(&tok.lexeme);
                Expr {
                    kind: ExprKind::Int(IntLit {
                        value,
                        suffix,
                        raw: tok.lexeme,
                    }),
                    span: tok.span,
                }
            }
            TokenKind::Float => {
                self.bump();
                Expr {
                    kind: ExprKind::Float(tok.lexeme),
                    span: tok.span,
                }
            }
            TokenKind::True => {
                self.bump();
                Expr {
                    kind: ExprKind::Bool(true),
                    span: tok.span,
                }
            }
            TokenKind::False => {
                self.bump();
                Expr {
                    kind: ExprKind::Bool(false),
                    span: tok.span,
                }
            }
            TokenKind::Char => {
                self.bump();
                let ch = parse_char_literal(&tok.lexeme);
                Expr {
                    kind: ExprKind::Char(ch),
                    span: tok.span,
                }
            }
            TokenKind::String => {
                self.bump();
                Expr {
                    kind: ExprKind::String(unquote_string(&tok.lexeme)),
                    span: tok.span,
                }
            }
            TokenKind::Ident => {
                let name = self.expect_ident("identifier");
                Expr {
                    span: name.span,
                    kind: ExprKind::Ident(name),
                }
            }
            TokenKind::LParen => {
                self.bump();
                let inner = self.parse_expr();
                self.expect(TokenKind::RParen, "`)`");
                Expr {
                    span: tok.span.merge(self.prev_span()),
                    kind: ExprKind::Paren(Box::new(inner)),
                }
            }
            k if k.is_type_keyword() => {
                // allow `u8` as a type name in expressions (e.g. Level-style? no)
                // Treat as identifier-like for enum-ish? Better: error.
                let name = self.expect_ident_or_type_as_ident();
                Expr {
                    span: name.span,
                    kind: ExprKind::Ident(name),
                }
            }
            _ => {
                self.error_here("expected expression");
                self.bump();
                Expr {
                    kind: ExprKind::Int(IntLit {
                        value: 0,
                        suffix: None,
                        raw: "0".into(),
                    }),
                    span: tok.span,
                }
            }
        }
    }

    fn expect_ident_or_type_as_ident(&mut self) -> Ident {
        let tok = self.bump();
        Ident::new(tok.lexeme, tok.span)
    }

    fn expect_ident(&mut self, what: &str) -> Ident {
        if self.at(TokenKind::Ident) {
            let tok = self.bump();
            Ident::new(tok.lexeme, tok.span)
        } else {
            self.error_here(format!("expected {what}"));
            Ident::new("_", self.peek().span)
        }
    }

    fn expect(&mut self, kind: TokenKind, what: &str) {
        if self.eat(kind) {
            return;
        }
        self.error_here(format!("expected {what}"));
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or(self.tokens.last().expect("token stream"))
    }

    fn peek_n(&self, n: usize) -> &Token {
        self.tokens
            .get(self.pos + n)
            .unwrap_or(self.tokens.last().expect("token stream"))
    }

    fn bump(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() && self.tokens[self.pos].kind != TokenKind::Eof {
            self.pos += 1;
        }
        tok
    }

    fn prev_span(&self) -> Span {
        if self.pos == 0 {
            self.peek().span
        } else {
            self.tokens[self.pos - 1].span
        }
    }

    fn error_here(&mut self, message: impl Into<String>) {
        let span = self.peek().span;
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    fn sync_item(&mut self) {
        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Semicolon) {
                self.bump();
                break;
            }
            if self.at(TokenKind::RBrace) {
                self.bump();
                break;
            }
            if matches!(
                self.peek().kind,
                TokenKind::Module
                    | TokenKind::Use
                    | TokenKind::Struct
                    | TokenKind::Enum
                    | TokenKind::Const
                    | TokenKind::Static
                    | TokenKind::Reg
                    | TokenKind::Extern
                    | TokenKind::Hash
            ) {
                break;
            }
            self.bump();
        }
    }
}

fn primitive_from_kind(kind: TokenKind) -> Option<PrimitiveTy> {
    Some(match kind {
        TokenKind::Int => PrimitiveTy::Int,
        TokenKind::Bool => PrimitiveTy::Bool,
        TokenKind::U8 => PrimitiveTy::U8,
        TokenKind::U16 => PrimitiveTy::U16,
        TokenKind::U32 => PrimitiveTy::U32,
        TokenKind::U64 => PrimitiveTy::U64,
        TokenKind::I8 => PrimitiveTy::I8,
        TokenKind::I16 => PrimitiveTy::I16,
        TokenKind::I32 => PrimitiveTy::I32,
        TokenKind::I64 => PrimitiveTy::I64,
        TokenKind::Usize => PrimitiveTy::Usize,
        TokenKind::Isize => PrimitiveTy::Isize,
        TokenKind::F32 => PrimitiveTy::F32,
        TokenKind::F64 => PrimitiveTy::F64,
        _ => return None,
    })
}

pub fn parse_int_literal(raw: &str) -> (i128, Option<PrimitiveTy>) {
    let mut body = raw;
    let mut suffix = None;
    for s in [
        "int", "usize", "isize", "u16", "u32", "u64", "i16", "i32", "i64", "u8", "i8", "f32", "f64",
    ] {
        if let Some(stripped) = raw.strip_suffix(s) {
            if stripped.ends_with(|c: char| c.is_ascii_digit() || c == '_') {
                body = stripped;
                suffix = PrimitiveTy::from_suffix(s);
                break;
            }
        }
    }
    let cleaned: String = body.chars().filter(|&c| c != '_').collect();
    let value = if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        i128::from_str_radix(hex, 16).unwrap_or(0)
    } else if let Some(bin) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        i128::from_str_radix(bin, 2).unwrap_or(0)
    } else {
        cleaned.parse().unwrap_or(0)
    };
    (value, suffix)
}

fn parse_char_literal(raw: &str) -> char {
    let inner = raw.trim_matches('\'');
    match inner {
        "\\n" => '\n',
        "\\r" => '\r',
        "\\t" => '\t',
        "\\\\" => '\\',
        "\\0" => '\0',
        "\\'" => '\'',
        s if !s.is_empty() => s.chars().next().unwrap_or('?'),
        _ => '?',
    }
}

fn unquote_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('0') => out.push('\0'),
                Some(c) => out.push(c),
                None => {}
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> Module {
        let (toks, lex_diags) = Lexer::new(0, src).tokenize();
        assert!(lex_diags.is_empty(), "{lex_diags:?}");
        let (module, diags) = Parser::new(toks).parse_module();
        assert!(diags.is_empty(), "{diags:?}");
        module
    }

    #[test]
    fn parses_function() {
        let m = parse("i32 add(i32 a, i32 b) { return a + b; }");
        assert_eq!(m.items.len(), 1);
        match &m.items[0] {
            Item::Fn(f) => assert_eq!(f.name.name, "add"),
            _ => panic!("expected fn"),
        }
    }

    #[test]
    fn parses_function_keyword() {
        let m = parse("function setup() { }");
        match &m.items[0] {
            Item::Fn(f) => {
                assert_eq!(f.name.name, "setup");
                assert!(matches!(f.return_ty.kind, TypeKind::Void));
            }
            _ => panic!("expected fn"),
        }
    }

    #[test]
    fn parses_int_function() {
        let m = parse("int add(int a, int b) { return a + b; }");
        match &m.items[0] {
            Item::Fn(f) => {
                assert_eq!(f.name.name, "add");
                assert!(matches!(
                    f.return_ty.kind,
                    TypeKind::Primitive(PrimitiveTy::Int)
                ));
            }
            _ => panic!("expected fn"),
        }
    }

    #[test]
    fn parses_register_and_use() {
        let m = parse("module blink;\nuse volt.delay;\nreg u8 PORTB @ 0x25;\n");
        assert_eq!(m.items.len(), 2);
    }

    #[test]
    fn parses_include() {
        let m = parse("#include <volt/gpio>\n#include \"local.volt\"\n");
        assert_eq!(m.items.len(), 2);
        match &m.items[0] {
            Item::Include(i) => {
                assert_eq!(i.path, "volt/gpio");
                assert_eq!(i.style, IncludeStyle::Angle);
            }
            _ => panic!("expected include"),
        }
        match &m.items[1] {
            Item::Include(i) => {
                assert_eq!(i.path, "local.volt");
                assert_eq!(i.style, IncludeStyle::Quote);
            }
            _ => panic!("expected include"),
        }
    }

    #[test]
    fn parses_arithmetic_and_comparisons() {
        let m = parse("int main() { return (1 + 2 - 3) * 4 / 5; }");
        match &m.items[0] {
            Item::Fn(f) => assert!(matches!(
                &f.body.as_ref().unwrap().stmts[0],
                Stmt::Return(Some(_), _)
            )),
            _ => panic!("expected fn"),
        }
        let m = parse("int main() { return 1 < 2; }");
        match &m.items[0] {
            Item::Fn(f) => match &f.body.as_ref().unwrap().stmts[0] {
                Stmt::Return(Some(e), _) => match &e.kind {
                    ExprKind::Binary(BinOp::Lt, _, _) => {}
                    other => panic!("expected <, got {other:?}"),
                },
                _ => panic!("expected return"),
            },
            _ => panic!("expected fn"),
        }
        let m = parse("int main() { return 2 > 1; }");
        match &m.items[0] {
            Item::Fn(f) => match &f.body.as_ref().unwrap().stmts[0] {
                Stmt::Return(Some(e), _) => match &e.kind {
                    ExprKind::Binary(BinOp::Gt, _, _) => {}
                    other => panic!("expected >, got {other:?}"),
                },
                _ => panic!("expected return"),
            },
            _ => panic!("expected fn"),
        }
    }

    #[test]
    fn parses_if_else() {
        let m = parse("function f() { if (x) { return 1; } else { return 0; } }");
        match &m.items[0] {
            Item::Fn(f) => match &f.body.as_ref().unwrap().stmts[0] {
                Stmt::If(i) => assert!(i.else_branch.is_some()),
                _ => panic!("expected if"),
            },
            _ => panic!("expected fn"),
        }
    }

    #[test]
    fn parses_const_local() {
        let m = parse("function f() { const int n = 4; }");
        match &m.items[0] {
            Item::Fn(f) => match &f.body.as_ref().unwrap().stmts[0] {
                Stmt::Local(l) => {
                    assert!(l.is_const);
                    assert_eq!(l.name.name, "n");
                }
                _ => panic!("expected const local"),
            },
            _ => panic!("expected fn"),
        }
    }
}
