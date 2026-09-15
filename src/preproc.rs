//! C-style `#define` before parse.
//!
//! Object-like: `#define LED 13`
//! Function-like (no space before `(`): `#define MAX(a, b) ((a) > (b) ? (a) : (b))`

use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::token::{Token, TokenKind};

#[derive(Clone)]
struct Macro {
    params: Option<Vec<String>>,
    body: Vec<Token>,
}

pub fn preprocess(src: &str, tokens: Vec<Token>) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut macros: HashMap<String, Macro> = HashMap::new();
    let mut diagnostics = Vec::new();
    let mut out = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let tok = &tokens[i];
        if tok.kind == TokenKind::Eof {
            out.push(tok.clone());
            break;
        }

        if tok.kind == TokenKind::Hash
            && at_line_start(src, tok.span)
            && matches!(tokens.get(i + 1), Some(t) if t.kind == TokenKind::Ident && t.lexeme == "define")
        {
            i += 2; // `#` `define`
            match parse_define(src, &tokens, &mut i) {
                Ok((name, mac)) => {
                    macros.insert(name, mac);
                }
                Err(diag) => {
                    diagnostics.push(diag);
                    while i < tokens.len()
                        && tokens[i].kind != TokenKind::Eof
                        && !at_line_start(src, tokens[i].span)
                    {
                        i += 1;
                    }
                }
            }
            continue;
        }

        expand_into(&tokens, &mut i, &macros, &mut out, &mut diagnostics, 0);
    }

    (out, diagnostics)
}

fn parse_define(src: &str, tokens: &[Token], i: &mut usize) -> Result<(String, Macro), Diagnostic> {
    let name_tok = tokens
        .get(*i)
        .ok_or_else(|| Diagnostic::error("expected macro name after `#define`", Span::dummy()))?;
    if name_tok.kind != TokenKind::Ident {
        return Err(Diagnostic::error(
            "expected macro name after `#define`",
            name_tok.span,
        ));
    }
    let name = name_tok.lexeme.clone();
    let name_span = name_tok.span;
    *i += 1;

    let params = if matches!(tokens.get(*i), Some(t) if t.kind == TokenKind::LParen)
        && adjacent(name_span, tokens[*i].span)
    {
        *i += 1;
        let mut params = Vec::new();
        if !matches!(tokens.get(*i), Some(t) if t.kind == TokenKind::RParen) {
            loop {
                let p = tokens.get(*i).ok_or_else(|| {
                    Diagnostic::error("unterminated macro parameter list", name_span)
                })?;
                if p.kind != TokenKind::Ident {
                    return Err(Diagnostic::error("expected parameter name", p.span));
                }
                params.push(p.lexeme.clone());
                *i += 1;
                if matches!(tokens.get(*i), Some(t) if t.kind == TokenKind::Comma) {
                    *i += 1;
                    continue;
                }
                break;
            }
        }
        let close = tokens
            .get(*i)
            .ok_or_else(|| Diagnostic::error("expected `)` after macro parameters", name_span))?;
        if close.kind != TokenKind::RParen {
            return Err(Diagnostic::error(
                "expected `)` after macro parameters",
                close.span,
            ));
        }
        *i += 1;
        Some(params)
    } else {
        None
    };

    let mut body = Vec::new();
    while *i < tokens.len()
        && tokens[*i].kind != TokenKind::Eof
        && !at_line_start(src, tokens[*i].span)
    {
        body.push(tokens[*i].clone());
        *i += 1;
    }

    Ok((name, Macro { params, body }))
}

fn expand_into(
    tokens: &[Token],
    i: &mut usize,
    macros: &HashMap<String, Macro>,
    out: &mut Vec<Token>,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) {
    let tok = tokens[*i].clone();
    if tok.kind == TokenKind::Ident {
        if let Some(mac) = macros.get(&tok.lexeme) {
            if depth > 64 {
                diagnostics.push(Diagnostic::error(
                    format!("macro `{}` expansion is too deep", tok.lexeme),
                    tok.span,
                ));
                out.push(tok);
                *i += 1;
                return;
            }
            if let Some(params) = &mac.params {
                if matches!(tokens.get(*i + 1), Some(t) if t.kind == TokenKind::LParen) {
                    *i += 2; // name + (
                    match collect_args(tokens, i, tok.span) {
                        Ok(args) => {
                            if args.len() != params.len() {
                                diagnostics.push(Diagnostic::error(
                                    format!(
                                        "macro `{}` expects {} argument(s), found {}",
                                        tok.lexeme,
                                        params.len(),
                                        args.len()
                                    ),
                                    tok.span,
                                ));
                                return;
                            }
                            let substituted = substitute(mac, params, &args, tok.span);
                            replay(
                                &substituted,
                                macros,
                                out,
                                diagnostics,
                                depth + 1,
                                &tok.lexeme,
                            );
                            return;
                        }
                        Err(diag) => {
                            diagnostics.push(diag);
                            return;
                        }
                    }
                }
                // Function-like name not followed by `(` is left alone (C behavior).
            } else {
                *i += 1;
                let substituted = retarget(&mac.body, tok.span);
                replay(
                    &substituted,
                    macros,
                    out,
                    diagnostics,
                    depth + 1,
                    &tok.lexeme,
                );
                return;
            }
        }
    }
    out.push(tok);
    *i += 1;
}

fn replay(
    body: &[Token],
    macros: &HashMap<String, Macro>,
    out: &mut Vec<Token>,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
    hide: &str,
) {
    let mut j = 0;
    let mut hidden = macros.clone();
    hidden.remove(hide);
    while j < body.len() {
        expand_into(body, &mut j, &hidden, out, diagnostics, depth);
    }
}

fn substitute(mac: &Macro, params: &[String], args: &[Vec<Token>], use_span: Span) -> Vec<Token> {
    let mut out = Vec::new();
    for tok in &mac.body {
        if tok.kind == TokenKind::Ident {
            if let Some(idx) = params.iter().position(|p| p == &tok.lexeme) {
                out.extend(retarget(&args[idx], use_span));
                continue;
            }
        }
        let mut t = tok.clone();
        t.span = use_span;
        out.push(t);
    }
    out
}

fn collect_args(
    tokens: &[Token],
    i: &mut usize,
    span: Span,
) -> Result<Vec<Vec<Token>>, Diagnostic> {
    let mut args = Vec::new();
    if matches!(tokens.get(*i), Some(t) if t.kind == TokenKind::RParen) {
        *i += 1;
        return Ok(args);
    }
    let mut current = Vec::new();
    let mut depth = 0usize;
    loop {
        let tok = tokens
            .get(*i)
            .ok_or_else(|| Diagnostic::error("unterminated macro argument list", span))?;
        match tok.kind {
            TokenKind::Eof => {
                return Err(Diagnostic::error("unterminated macro argument list", span));
            }
            TokenKind::LParen => {
                depth += 1;
                current.push(tok.clone());
                *i += 1;
            }
            TokenKind::RParen if depth == 0 => {
                args.push(current);
                *i += 1;
                return Ok(args);
            }
            TokenKind::RParen => {
                depth -= 1;
                current.push(tok.clone());
                *i += 1;
            }
            TokenKind::Comma if depth == 0 => {
                args.push(std::mem::take(&mut current));
                *i += 1;
            }
            _ => {
                current.push(tok.clone());
                *i += 1;
            }
        }
    }
}

fn retarget(tokens: &[Token], span: Span) -> Vec<Token> {
    tokens
        .iter()
        .map(|t| {
            let mut c = t.clone();
            c.span = span;
            c
        })
        .collect()
}

fn adjacent(a: Span, b: Span) -> bool {
    a.end == b.start
}

fn at_line_start(src: &str, span: Span) -> bool {
    let start = span.start as usize;
    if start == 0 {
        return true;
    }
    let before = &src[..start.min(src.len())];
    let trimmed = before.trim_end_matches([' ', '\t', '\r']);
    trimmed.is_empty() || trimmed.ends_with('\n')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn expand(src: &str) -> Vec<String> {
        let (toks, lex_diags) = Lexer::new(0, src).tokenize();
        assert!(lex_diags.is_empty(), "{lex_diags:?}");
        let (toks, pre_diags) = preprocess(src, toks);
        assert!(pre_diags.is_empty(), "{pre_diags:?}");
        toks.into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.lexeme)
            .collect()
    }

    #[test]
    fn object_like_define() {
        let lexemes = expand("#define LED 13\npin_mode(LED, 1);");
        assert!(lexemes.contains(&"13".to_string()));
        assert!(!lexemes.contains(&"LED".to_string()));
        assert!(!lexemes.contains(&"define".to_string()));
    }

    #[test]
    fn function_like_define() {
        let lexemes = expand("#define MAX(a, b) a\nMAX(1, 2);");
        assert_eq!(lexemes, vec!["1", ";"]);
    }
}
