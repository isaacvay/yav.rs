//! RSX parser — parses JSX-like token streams into an AST.
//!
//! This parser consumes a `proc_macro2::TokenStream` containing HTML-like
//! markup and produces an [`RsxNode`] AST that the codegen module transforms
//! into Rust code constructing `yav_core::Node` trees.
//!
//! ## Supported Grammar
//!
//! ```text
//! rsx        := node*
//! node       := element | expr_block | string_literal
//! element    := '<' tag attributes ( '/>' | '>' node* '</' tag '>' )
//! attributes := attribute*
//! attribute  := name '=' value | name
//! name       := IDENT ('-' IDENT)*
//! value      := STRING_LITERAL | '{' EXPR '}'
//! expr_block := '{' EXPR '}'
//! tag        := IDENT ('-' IDENT)*
//! ```
//!
//! - Lowercase tags (e.g., `div`, `span`) are HTML elements
//! - PascalCase tags (e.g., `Button`, `NavBar`) are component references
//! - Hyphenated tags (e.g., `my-component`) are supported

use proc_macro2::{Delimiter, Ident, Span, TokenStream, TokenTree};
use std::collections::VecDeque;
use syn::Result;

/// AST node produced by the RSX parser.
#[derive(Debug)]
pub enum RsxNode {
    /// An HTML element: `<tag attrs>children</tag>` or `<tag attrs />`
    Element(RsxElement),
    /// A text literal: `"hello world"`
    Text(String, Span),
    /// A Rust expression block: `{expr}`
    Expr(TokenStream, Span),
    /// Multiple sibling nodes
    Fragment(Vec<RsxNode>),
    /// Raw unescaped text content (used for `<script>` and `<style>` blocks)
    Raw(String, Span),
    /// Dynamic expression evaluated as raw unescaped content inside `<script>` or `<style>`
    RawExpr(TokenStream, Span),
    /// Native conditional control flow: `if cond { ... } else { ... }`
    If {
        cond: TokenStream,
        then_branch: Box<RsxNode>,
        else_branch: Option<Box<RsxNode>>,
    },
    /// Native loop control flow: `for pat in expr { ... }`
    For {
        header: TokenStream,
        body: Box<RsxNode>,
    },
}

/// An element in the RSX tree.
#[derive(Debug)]
pub struct RsxElement {
    pub tag: String,
    pub tag_span: Span,
    pub attributes: Vec<RsxAttribute>,
    pub children: Vec<RsxNode>,
    pub self_closing: bool,
    /// True if tag starts with uppercase (component reference)
    pub is_component: bool,
}

/// An attribute on an RSX element.
#[derive(Debug)]
pub struct RsxAttribute {
    pub name: String,
    pub name_span: Span,
    pub value: RsxAttrValue,
}

/// The value of an RSX attribute.
#[derive(Debug)]
#[allow(dead_code)]
pub enum RsxAttrValue {
    /// A string literal: `class="foo"`
    Str(String, Span),
    /// A Rust expression: `id={my_id}`
    Expr(TokenStream, Span),
    /// A boolean attribute with no value: `disabled`
    Bool(Span),
}

/// A buffered token reader that supports multi-token lookahead.
struct TokenReader {
    buf: VecDeque<TokenTree>,
}

impl TokenReader {
    fn new(stream: TokenStream) -> Self {
        TokenReader {
            buf: stream.into_iter().collect(),
        }
    }

    fn peek(&self) -> Option<&TokenTree> {
        self.buf.front()
    }

    fn peek_nth(&self, n: usize) -> Option<&TokenTree> {
        self.buf.get(n)
    }

    fn next(&mut self) -> Option<TokenTree> {
        self.buf.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Check if the next tokens form a closing tag `</ tag`
    fn is_closing_tag(&self) -> bool {
        matches!(
            (self.peek_nth(0), self.peek_nth(1)),
            (Some(TokenTree::Punct(p1)), Some(TokenTree::Punct(p2)))
                if p1.as_char() == '<' && p2.as_char() == '/'
        )
    }

    /// Check if the next tokens form a closing fragment `</>`
    fn is_closing_fragment(&self) -> bool {
        matches!(
            (self.peek_nth(0), self.peek_nth(1), self.peek_nth(2)),
            (Some(TokenTree::Punct(p1)), Some(TokenTree::Punct(p2)), Some(TokenTree::Punct(p3)))
                if p1.as_char() == '<' && p2.as_char() == '/' && p3.as_char() == '>'
        )
    }

    fn expect_punct(&mut self, expected: char) -> Result<Span> {
        match self.next() {
            Some(TokenTree::Punct(p)) if p.as_char() == expected => Ok(p.span()),
            Some(tt) => Err(syn::Error::new(
                tt.span(),
                format!("expected `{}`, got `{}`", expected, tt),
            )),
            None => Err(syn::Error::new(
                Span::call_site(),
                format!("expected `{}`, found end of input", expected),
            )),
        }
    }

    fn expect_ident(&mut self) -> Result<Ident> {
        match self.next() {
            Some(TokenTree::Ident(id)) => Ok(id),
            Some(tt) => Err(syn::Error::new(
                tt.span(),
                format!("expected identifier, got `{}`", tt),
            )),
            None => Err(syn::Error::new(
                Span::call_site(),
                "expected identifier, found end of input",
            )),
        }
    }
}

/// Parse a token stream of RSX markup into an AST node.
pub fn parse_rsx(input: TokenStream) -> Result<RsxNode> {
    let mut reader = TokenReader::new(input);
    let mut nodes = Vec::new();

    while !reader.is_empty() {
        nodes.push(parse_node(&mut reader)?);
    }

    Ok(match nodes.len() {
        0 => RsxNode::Fragment(vec![]),
        1 => nodes.into_iter().next().unwrap(),
        _ => RsxNode::Fragment(nodes),
    })
}

fn parse_node(reader: &mut TokenReader) -> Result<RsxNode> {
    match reader.peek() {
        // `<>` — start of an anonymous fragment or `<` element
        Some(TokenTree::Punct(p)) if p.as_char() == '<' => {
            if let Some(TokenTree::Punct(p2)) = reader.peek_nth(1) {
                if p2.as_char() == '>' {
                    reader.next(); // consume `<`
                    reader.next(); // consume `>`
                    let mut children = Vec::new();
                    while !reader.is_empty() {
                        if reader.is_closing_fragment() {
                            reader.expect_punct('<')?;
                            reader.expect_punct('/')?;
                            reader.expect_punct('>')?;
                            break;
                        }
                        children.push(parse_node(reader)?);
                    }
                    return Ok(RsxNode::Fragment(children));
                }
            }
            parse_element(reader)
        }

        // `{ expr }` — expression block
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
            let g = match reader.next().unwrap() {
                TokenTree::Group(g) => g,
                _ => unreachable!(),
            };
            Ok(RsxNode::Expr(g.stream(), g.span()))
        }

        // String/number literal
        Some(TokenTree::Literal(_)) => {
            let lit = match reader.next().unwrap() {
                TokenTree::Literal(l) => l,
                _ => unreachable!(),
            };
            let span = lit.span();
            match syn::Lit::new(lit.clone()) {
                syn::Lit::Str(lit_str) => Ok(RsxNode::Text(lit_str.value(), span)),
                _ => Ok(RsxNode::Text(lit.to_string(), span)),
            }
        }

        // Native `if cond { ... } [else ...]` control flow
        Some(TokenTree::Ident(id)) if id == "if" && can_start_if(reader) => parse_if(reader),

        // Native `for pat in expr { ... }` control flow
        Some(TokenTree::Ident(id)) if id == "for" && can_start_for(reader) => parse_for(reader),

        // Raw text (words, identifiers, punctuation e.g. `Voir détails` or `Compteur :`)
        Some(_) => parse_raw_text(reader),

        None => Err(syn::Error::new(
            Span::call_site(),
            "unexpected end of rsx! input",
        )),
    }
}

fn body_looks_like_rsx(stream: &TokenStream) -> bool {
    let mut iter = stream.clone().into_iter();
    match iter.next() {
        None => true,
        Some(TokenTree::Punct(p)) if p.as_char() == '<' => true,
        Some(TokenTree::Literal(_)) => true,
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => true,
        Some(TokenTree::Ident(id)) if id == "if" || id == "for" => true,
        _ => false,
    }
}

fn can_start_if(reader: &TokenReader) -> bool {
    if !matches!(reader.peek(), Some(TokenTree::Ident(id)) if id == "if") {
        return false;
    }
    let mut idx = 1;
    while let Some(tt) = reader.peek_nth(idx) {
        if matches!(
            (reader.peek_nth(idx), reader.peek_nth(idx + 1)),
            (Some(TokenTree::Punct(p1)), Some(TokenTree::Punct(p2)))
                if p1.as_char() == '<' && p2.as_char() == '/'
        ) {
            return false;
        }
        if let TokenTree::Group(g) = tt {
            if g.delimiter() == Delimiter::Brace {
                return idx > 1 && body_looks_like_rsx(&g.stream());
            }
        }
        idx += 1;
    }
    false
}

fn can_start_for(reader: &TokenReader) -> bool {
    if !matches!(reader.peek(), Some(TokenTree::Ident(id)) if id == "for") {
        return false;
    }
    let mut idx = 1;
    let mut found_in = None;
    while let Some(tt) = reader.peek_nth(idx) {
        if matches!(
            (reader.peek_nth(idx), reader.peek_nth(idx + 1)),
            (Some(TokenTree::Punct(p1)), Some(TokenTree::Punct(p2)))
                if p1.as_char() == '<' && p2.as_char() == '/'
        ) {
            return false;
        }
        if found_in.is_none() {
            if let TokenTree::Ident(id) = tt {
                if id == "in" && idx > 1 {
                    found_in = Some(idx);
                    idx += 1;
                    continue;
                }
            }
            if matches!(tt, TokenTree::Group(g) if g.delimiter() == Delimiter::Brace) {
                return false;
            }
        } else if let TokenTree::Group(g) = tt {
            if g.delimiter() == Delimiter::Brace {
                if let Some(in_idx) = found_in {
                    return idx > in_idx + 1 && body_looks_like_rsx(&g.stream());
                }
            }
        }
        idx += 1;
    }
    false
}

fn parse_if(reader: &mut TokenReader) -> Result<RsxNode> {
    let if_ident = reader.expect_ident()?;
    let mut cond_tokens = Vec::new();
    let then_stream = loop {
        match reader.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                let group = match reader.next().unwrap() {
                    TokenTree::Group(g) => g,
                    _ => unreachable!(),
                };
                break group.stream();
            }
            Some(_) => {
                cond_tokens.push(reader.next().unwrap());
            }
            None => {
                return Err(syn::Error::new(
                    if_ident.span(),
                    "expected `{ ... }` body after `if` condition in rsx!",
                ));
            }
        }
    };

    let cond: TokenStream = cond_tokens.into_iter().collect();
    let then_branch = Box::new(parse_rsx(then_stream)?);

    let else_branch = if matches!(reader.peek(), Some(TokenTree::Ident(id)) if id == "else") {
        reader.next(); // consume `else`
        if matches!(reader.peek(), Some(TokenTree::Ident(id)) if id == "if") {
            Some(Box::new(parse_if(reader)?))
        } else {
            match reader.next() {
                Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                    Some(Box::new(parse_rsx(g.stream())?))
                }
                Some(tt) => {
                    return Err(syn::Error::new(
                        tt.span(),
                        format!("expected `{{ ... }}` or `if` after `else`, got `{}`", tt),
                    ));
                }
                None => {
                    return Err(syn::Error::new(
                        if_ident.span(),
                        "expected `{ ... }` after `else` in rsx!",
                    ));
                }
            }
        }
    } else {
        None
    };

    Ok(RsxNode::If {
        cond,
        then_branch,
        else_branch,
    })
}

fn parse_for(reader: &mut TokenReader) -> Result<RsxNode> {
    let for_ident = reader.expect_ident()?;
    let mut header_tokens = Vec::new();
    let body_stream = loop {
        match reader.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                let group = match reader.next().unwrap() {
                    TokenTree::Group(g) => g,
                    _ => unreachable!(),
                };
                break group.stream();
            }
            Some(_) => {
                header_tokens.push(reader.next().unwrap());
            }
            None => {
                return Err(syn::Error::new(
                    for_ident.span(),
                    "expected `{ ... }` body after `for ... in ...` in rsx!",
                ));
            }
        }
    };

    let header: TokenStream = header_tokens.into_iter().collect();
    let body = Box::new(parse_rsx(body_stream)?);

    Ok(RsxNode::For { header, body })
}

fn parse_raw_text(reader: &mut TokenReader) -> Result<RsxNode> {
    let mut words = Vec::new();
    let mut first_span = None;

    while let Some(tt) = reader.peek() {
        match tt {
            // Stop if start of tag `<` or closing tag `</`
            TokenTree::Punct(p) if p.as_char() == '<' => break,
            // Stop if expression block `{`
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => break,
            // Stop if native control flow `if ... { ... }` or `for ... in ... { ... }` starts after words
            TokenTree::Ident(id)
                if !words.is_empty()
                    && ((id == "if" && can_start_if(reader))
                        || (id == "for" && can_start_for(reader))) =>
            {
                break;
            }
            _ => {
                let token = reader.next().unwrap();
                if first_span.is_none() {
                    first_span = Some(token.span());
                }
                words.push(token);
            }
        }
    }

    if words.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "unexpected end of text in rsx!",
        ));
    }

    let span = first_span.unwrap();
    let mut text = String::new();
    for token in words {
        let token_str = token.to_string();
        let is_punct = matches!(&token, TokenTree::Punct(p) if matches!(p.as_char(), ',' | '.' | '!' | '?' | ';' | ':'));
        if !text.is_empty() && !is_punct && !text.ends_with(' ') {
            text.push(' ');
        }
        text.push_str(&token_str);
    }

    let followed_by_expr =
        matches!(reader.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace);
    if followed_by_expr && !text.is_empty() && !text.ends_with(' ') {
        text.push(' ');
    }

    Ok(RsxNode::Text(text, span))
}

fn parse_element(reader: &mut TokenReader) -> Result<RsxNode> {
    // Consume `<`
    reader.expect_punct('<')?;

    // Parse tag name
    let (tag, tag_span) = parse_tag_name(reader)?;
    let last_segment = tag.rsplit("::").next().unwrap_or(&tag);
    let is_component = tag.contains("::")
        || last_segment
            .chars()
            .next()
            .is_some_and(|c| c.is_uppercase());

    // Parse attributes until `>` or `/>`
    let mut attributes = Vec::new();
    loop {
        match reader.peek() {
            // `>` — end of opening tag
            Some(TokenTree::Punct(p)) if p.as_char() == '>' => {
                reader.next();
                break;
            }
            // `/` `>` — self-closing
            Some(TokenTree::Punct(p)) if p.as_char() == '/' => {
                reader.next(); // consume `/`
                reader.expect_punct('>')?;
                return Ok(RsxNode::Element(RsxElement {
                    tag,
                    tag_span,
                    attributes,
                    children: vec![],
                    self_closing: true,
                    is_component,
                }));
            }
            // Attribute name (identifier)
            Some(TokenTree::Ident(_)) => {
                attributes.push(parse_attribute(reader)?);
            }
            Some(tt) => {
                return Err(syn::Error::new(
                    tt.span(),
                    format!("unexpected token in <{}> attributes: `{}`", tag, tt),
                ));
            }
            None => {
                return Err(syn::Error::new(
                    tag_span,
                    format!("unexpected end of input while parsing <{}>", tag),
                ));
            }
        }
    }

    // Parse children: if `<script>` or `<style>`, parse raw tokens verbatim without evaluating as Rust expressions
    if !is_component && (tag == "script" || tag == "style") {
        let raw_child = parse_raw_body(reader, &tag)?;
        reader.expect_punct('<')?;
        reader.expect_punct('/')?;
        let (close_tag, close_span) = parse_tag_name(reader)?;
        if close_tag != tag {
            return Err(syn::Error::new(
                close_span,
                format!(
                    "mismatched closing tag: expected `</{}>`, got `</{}>`",
                    tag, close_tag
                ),
            ));
        }
        reader.expect_punct('>')?;

        return Ok(RsxNode::Element(RsxElement {
            tag,
            tag_span,
            attributes,
            children: vec![raw_child],
            self_closing: false,
            is_component,
        }));
    }

    // Parse standard children until closing tag `</tag>`
    let mut children = Vec::new();
    loop {
        // Check for closing tag `</`
        if reader.is_closing_tag() {
            // Consume `<`, `/`, tag, `>`
            reader.expect_punct('<')?;
            reader.expect_punct('/')?;
            let (close_tag, close_span) = parse_tag_name(reader)?;
            if close_tag != tag {
                return Err(syn::Error::new(
                    close_span,
                    format!(
                        "mismatched closing tag: expected `</{}>`, got `</{}>`",
                        tag, close_tag
                    ),
                ));
            }
            reader.expect_punct('>')?;
            break;
        }

        if reader.is_empty() {
            return Err(syn::Error::new(
                tag_span,
                format!("unclosed element `<{}>`: expected `</{}>`", tag, tag),
            ));
        }

        children.push(parse_node(reader)?);
    }

    Ok(RsxNode::Element(RsxElement {
        tag,
        tag_span,
        attributes,
        children,
        self_closing: false,
        is_component,
    }))
}

fn parse_attribute(reader: &mut TokenReader) -> Result<RsxAttribute> {
    let (name, name_span) = parse_attr_name(reader)?;

    // Check for `=` (valued attribute) vs. no `=` (boolean)
    match reader.peek() {
        Some(TokenTree::Punct(p)) if p.as_char() == '=' => {
            reader.next(); // consume `=`
            let value = parse_attr_value(reader)?;
            Ok(RsxAttribute {
                name,
                name_span,
                value,
            })
        }
        _ => Ok(RsxAttribute {
            name,
            name_span,
            value: RsxAttrValue::Bool(name_span),
        }),
    }
}

fn parse_attr_name(reader: &mut TokenReader) -> Result<(String, Span)> {
    let ident = reader.expect_ident()?;
    let span = ident.span();
    let ident_str = ident.to_string();
    let mut name = ident_str
        .strip_prefix("r#")
        .unwrap_or(&ident_str)
        .to_string();

    // Allow hyphenated and colon-separated names: `data-id`, `aria-label`, `client:load`, `class:active`, `class:opacity-50`
    loop {
        match reader.peek() {
            Some(TokenTree::Punct(p)) if p.as_char() == '-' || p.as_char() == ':' => {
                let sep = p.as_char();
                // Lookahead: is this `- ident/lit` or `: ident`?
                match reader.peek_nth(1) {
                    Some(TokenTree::Ident(_)) => {
                        reader.next(); // consume separator
                        let next = reader.expect_ident()?;
                        let next_str = next.to_string();
                        name.push(sep);
                        name.push_str(next_str.strip_prefix("r#").unwrap_or(&next_str));
                    }
                    Some(TokenTree::Literal(_)) if sep == '-' => {
                        reader.next(); // consume `-`
                        let lit = reader.next().unwrap();
                        name.push(sep);
                        name.push_str(&lit.to_string());
                    }
                    _ => break,
                }
            }
            _ => break,
        }
    }

    Ok((name, span))
}

fn parse_attr_value(reader: &mut TokenReader) -> Result<RsxAttrValue> {
    match reader.peek() {
        Some(TokenTree::Literal(_)) => {
            let lit = match reader.next().unwrap() {
                TokenTree::Literal(l) => l,
                _ => unreachable!(),
            };
            let span = lit.span();
            match syn::Lit::new(lit.clone()) {
                syn::Lit::Str(lit_str) => Ok(RsxAttrValue::Str(lit_str.value(), span)),
                _ => {
                    let stream = quote::quote!(#lit);
                    Ok(RsxAttrValue::Expr(stream, span))
                }
            }
        }
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
            let g = match reader.next().unwrap() {
                TokenTree::Group(g) => g,
                _ => unreachable!(),
            };
            Ok(RsxAttrValue::Expr(g.stream(), g.span()))
        }
        Some(TokenTree::Ident(_)) => {
            let id = reader.expect_ident()?;
            let span = id.span();
            let id_stream = quote::quote!(#id);
            Ok(RsxAttrValue::Expr(id_stream, span))
        }
        Some(tt) => Err(syn::Error::new(
            tt.span(),
            format!(
                "expected attribute value (\"string\", {{expr}}, or identifier), got `{}`",
                tt
            ),
        )),
        None => Err(syn::Error::new(
            Span::call_site(),
            "expected attribute value, found end of input",
        )),
    }
}

fn parse_tag_name(reader: &mut TokenReader) -> Result<(String, Span)> {
    let ident = reader.expect_ident()?;
    let span = ident.span();
    let mut name = ident.to_string();

    // Allow hyphenated tag names (`my-component`) and module-qualified component paths (`ui::Button`)
    loop {
        match reader.peek() {
            Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
                match reader.peek_nth(1) {
                    Some(TokenTree::Ident(_)) => {
                        reader.next(); // consume `-`
                        let next = reader.expect_ident()?;
                        name.push('-');
                        name.push_str(&next.to_string());
                    }
                    _ => break,
                }
            }
            Some(TokenTree::Punct(p)) if p.as_char() == ':' => {
                match (reader.peek_nth(1), reader.peek_nth(2)) {
                    (Some(TokenTree::Punct(p2)), Some(TokenTree::Ident(_)))
                        if p2.as_char() == ':' =>
                    {
                        reader.next(); // consume first `:`
                        reader.next(); // consume second `:`
                        let next = reader.expect_ident()?;
                        name.push_str("::");
                        name.push_str(&next.to_string());
                    }
                    _ => break,
                }
            }
            _ => break,
        }
    }

    Ok((name, span))
}

fn parse_raw_body(reader: &mut TokenReader, expected_tag: &str) -> Result<RsxNode> {
    let mut tokens = Vec::new();
    let span = reader.peek().map(|t| t.span()).unwrap_or_else(Span::call_site);

    while !reader.is_empty() {
        if reader.is_closing_tag() {
            if let Some(TokenTree::Ident(next_id)) = reader.peek_nth(2) {
                if next_id == expected_tag {
                    break;
                }
            }
        }
        tokens.push(reader.next().unwrap());
    }

    if tokens.len() == 1 {
        if let TokenTree::Group(g) = &tokens[0] {
            if g.delimiter() == Delimiter::Brace {
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                if inner.len() == 1 {
                    if let TokenTree::Literal(lit) = &inner[0] {
                        if let Ok(s) = syn::parse2::<syn::LitStr>(quote::quote!(#lit)) {
                            return Ok(RsxNode::Raw(s.value(), span));
                        }
                    }
                }
                if let Ok(expr) = syn::parse2::<syn::Expr>(g.stream()) {
                    if !matches!(expr, syn::Expr::Block(_)) {
                        return Ok(RsxNode::RawExpr(g.stream(), g.span()));
                    }
                }
            }
        }
    }

    let raw_string = raw_tokens_to_string(&tokens);
    Ok(RsxNode::Raw(raw_string, span))
}

fn raw_tokens_to_string(tokens: &[TokenTree]) -> String {
    if !tokens.is_empty() {
        let mut all_lits = Vec::new();
        let mut only_lits = true;
        for tt in tokens {
            match tt {
                TokenTree::Literal(lit) => {
                    if let Ok(s) = syn::parse2::<syn::LitStr>(quote::quote!(#lit)) {
                        all_lits.push(s.value());
                    } else {
                        only_lits = false;
                        break;
                    }
                }
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                    let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                    if inner.len() == 1 {
                        if let TokenTree::Literal(lit) = &inner[0] {
                            if let Ok(s) = syn::parse2::<syn::LitStr>(quote::quote!(#lit)) {
                                all_lits.push(s.value());
                                continue;
                            }
                        }
                    }
                    only_lits = false;
                    break;
                }
                _ => {
                    only_lits = false;
                    break;
                }
            }
        }
        if only_lits && !all_lits.is_empty() {
            return all_lits.join("\n");
        }
    }

    let mut out = String::new();
    format_token_trees(tokens, &mut out, 0);
    out
}

fn format_token_trees(tokens: &[TokenTree], out: &mut String, brace_depth: usize) {
    let mut prev_char = None;
    for tt in tokens {
        match tt {
            TokenTree::Group(g) => {
                let (open, close, next_depth) = match g.delimiter() {
                    Delimiter::Brace => ('{', '}', brace_depth + 1),
                    Delimiter::Parenthesis => ('(', ')', brace_depth),
                    Delimiter::Bracket => ('[', ']', brace_depth),
                    Delimiter::None => (' ', ' ', brace_depth),
                };
                if open != ' ' {
                    if let Some(pc) = prev_char {
                        if pc != '(' && pc != '[' && pc != '{' && pc != '.' && pc != '#' && pc != ':' {
                            out.push(' ');
                        }
                    }
                    out.push(open);
                    prev_char = Some(open);
                }
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                format_token_trees(&inner, out, next_depth);
                if close != ' ' {
                    out.push(close);
                    prev_char = Some(close);
                }
            }
            TokenTree::Ident(id) => {
                let s = id.to_string();
                let clean = s.strip_prefix("r#").unwrap_or(&s);
                if let Some(pc) = prev_char {
                    if pc.is_alphanumeric()
                        || pc == '_'
                        || pc == ')'
                        || pc == '}'
                        || pc == ']'
                        || pc == '"'
                        || pc == '\''
                        || (pc == ':' && brace_depth > 0)
                    {
                        out.push(' ');
                    }
                }
                out.push_str(clean);
                prev_char = clean.chars().last();
            }
            TokenTree::Punct(p) => {
                let ch = p.as_char();
                if let Some(pc) = prev_char {
                    if ((ch == '=' || ch == '+' || ch == '*' || ch == '&' || ch == '|') && pc.is_alphanumeric())
                        || pc == ','
                        || pc == ';'
                        || (pc == ':' && ch != ':' && brace_depth > 0)
                    {
                        out.push(' ');
                    }
                }
                out.push(ch);
                prev_char = Some(ch);
            }
            TokenTree::Literal(lit) => {
                if let Some(pc) = prev_char {
                    if pc.is_alphanumeric() || pc == '_' || pc == '=' || pc == ':' || pc == ',' {
                        out.push(' ');
                    }
                }
                let s = lit.to_string();
                out.push_str(&s);
                prev_char = s.chars().last();
            }
        }
    }
}
