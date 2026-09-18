//! C-style `#define` and `#change` before parse.
//!
//! Object-like: `#define LED 13`
//! Function-like (no space before `(`): `#define MAX(a, b) ((a) > (b) ? (a) : (b))`
//! Short names: `#change enum to em` then write `em Level { Low = 0, High = 1 }`

use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::span::Span;
use crate::token::{Token, TokenKind};

#[derive(Clone)]
struct Macro {
    params: Option<Vec<String>>,
    body: Vec<Token>,
}

#[derive(Clone)]
struct Change {
    kind: TokenKind,
    lexeme: String,
}

pub fn preprocess(src: &str, tokens: Vec<Token>) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut macros: HashMap<String, Macro> = HashMap::new();
    let mut changes: HashMap<String, Change> = HashMap::new();
    let mut diagnostics = Vec::new();
    let mut out = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        let tok = &tokens[i];
        if tok.kind == TokenKind::Eof {
            out.push(tok.clone());
            break;
        }

        if tok.kind == TokenKind::Hash && at_line_start(src, tok.span) {
            if matches!(tokens.get(i + 1), Some(t) if t.kind == TokenKind::Ident && t.lexeme == "define")
            {
                i += 2; // `#` `define`
                match parse_define(src, &tokens, &mut i) {
                    Ok((name, mac)) => {
                        macros.insert(name, mac);
                    }
                    Err(diag) => {
                        diagnostics.push(diag);
                        skip_rest_of_line(src, &tokens, &mut i);
                    }
                }
                continue;
            }
            if matches!(tokens.get(i + 1), Some(t) if t.kind == TokenKind::Ident && t.lexeme == "change")
            {
                i += 2; // `#` `change`
                match parse_change(&tokens, &mut i) {
                    Ok((short, change)) => {
                        changes.insert(short, change);
                        skip_rest_of_line(src, &tokens, &mut i);
                    }
                    Err(diag) => {
                        diagnostics.push(diag);
                        skip_rest_of_line(src, &tokens, &mut i);
                    }
                }
                continue;
            }
        }

        expand_into(
            &tokens,
            &mut i,
            &macros,
            &changes,
            &mut out,
            &mut diagnostics,
            0,
        );
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

fn parse_change(tokens: &[Token], i: &mut usize) -> Result<(String, Change), Diagnostic> {
    let from = tokens.get(*i).ok_or_else(|| {
        Diagnostic::error(
            "expected a name after `#change` — try `#change enum to em`",
            Span::dummy(),
        )
    })?;
    if !is_word(from) {
        return Err(Diagnostic::error(
            "expected a name after `#change` — try `#change enum to em`",
            from.span,
        ));
    }
    let from_span = from.span;
    let change = Change {
        kind: from.kind,
        lexeme: from.lexeme.clone(),
    };
    *i += 1;

    let arrow = tokens.get(*i).ok_or_else(|| {
        Diagnostic::error("expected `to` — write `#change enum to em`", from_span)
    })?;
    if arrow.kind != TokenKind::Ident || arrow.lexeme != "to" {
        return Err(Diagnostic::error(
            "expected `to` — write `#change enum to em`",
            arrow.span,
        ));
    }
    *i += 1;

    let short = tokens.get(*i).ok_or_else(|| {
        Diagnostic::error(
            "expected the short name after `to` — try `#change enum to em`",
            Span::dummy(),
        )
    })?;
    if short.kind != TokenKind::Ident {
        return Err(Diagnostic::error(
            "expected the short name after `to` — try `#change enum to em`",
            short.span,
        ));
    }
    if TokenKind::keyword(&short.lexeme).is_some() {
        return Err(Diagnostic::error(
            format!(
                "short name `{}` is already a Volt word — pick something else",
                short.lexeme
            ),
            short.span,
        ));
    }
    let short_name = short.lexeme.clone();
    *i += 1;
    Ok((short_name, change))
}

fn expand_into(
    tokens: &[Token],
    i: &mut usize,
    macros: &HashMap<String, Macro>,
    changes: &HashMap<String, Change>,
    out: &mut Vec<Token>,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) {
    let mut tok = tokens[*i].clone();
    apply_change(&mut tok, changes);
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
                                changes,
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
                    changes,
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
    changes: &HashMap<String, Change>,
    out: &mut Vec<Token>,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
    hide: &str,
) {
    let mut j = 0;
    let mut hidden = macros.clone();
    hidden.remove(hide);
    while j < body.len() {
        expand_into(body, &mut j, &hidden, changes, out, diagnostics, depth);
    }
}

fn apply_change(tok: &mut Token, changes: &HashMap<String, Change>) {
    if !is_word(tok) {
        return;
    }
    if let Some(change) = changes.get(&tok.lexeme) {
        tok.kind = change.kind;
        tok.lexeme = change.lexeme.clone();
    }
}

fn is_word(tok: &Token) -> bool {
    tok.kind == TokenKind::Ident || TokenKind::keyword(&tok.lexeme).is_some()
}

fn skip_rest_of_line(src: &str, tokens: &[Token], i: &mut usize) {
    while *i < tokens.len()
        && tokens[*i].kind != TokenKind::Eof
        && !at_line_start(src, tokens[*i].span)
    {
        *i += 1;
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

    fn expand_kinds(src: &str) -> Vec<(TokenKind, String)> {
        let (toks, lex_diags) = Lexer::new(0, src).tokenize();
        assert!(lex_diags.is_empty(), "{lex_diags:?}");
        let (toks, pre_diags) = preprocess(src, toks);
        assert!(pre_diags.is_empty(), "{pre_diags:?}");
        toks.into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| (t.kind, t.lexeme))
            .collect()
    }

    #[test]
    fn change_rewrites_enum_keyword() {
        let kinds = expand_kinds("#change enum to em\nem Foo { A = 1 }");
        assert_eq!(kinds[0].0, TokenKind::Enum);
        assert_eq!(kinds[0].1, "enum");
        assert!(!kinds.iter().any(|(_, lex)| lex == "em"));
        assert!(!kinds.iter().any(|(_, lex)| lex == "change"));
    }

    #[test]
    fn change_rewrites_ident() {
        let kinds = expand_kinds("#change pin_mode to pm\npm(13, 1)");
        assert_eq!(kinds[0], (TokenKind::Ident, "pin_mode".into()));
        assert!(!kinds.iter().any(|(_, lex)| lex == "pm"));
    }

    #[test]
    fn change_then_define_expands() {
        let lexemes = expand("#define LED 13\n#change LED to L\nL");
        assert_eq!(lexemes, vec!["13"]);
    }
}
