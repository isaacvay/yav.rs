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
            let repr = lit.to_string();

            if repr.starts_with('"') && repr.ends_with('"') && repr.len() >= 2 {
                let inner = unescape_string_literal(&repr[1..repr.len() - 1]);
                Ok(RsxNode::Text(inner, span))
            } else {
                Ok(RsxNode::Text(repr, span))
            }
        }

        // Raw text (words, identifiers, punctuation e.g. `Voir détails` or `Compteur :`)
        Some(_) => parse_raw_text(reader),

        None => Err(syn::Error::new(
            Span::call_site(),
            "unexpected end of rsx! input",
        )),
    }
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
    let is_component = tag.chars().next().is_some_and(|c| c.is_uppercase());

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

    // Parse children until closing tag `</tag>`
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
    let mut name = ident.to_string();

    // Allow hyphenated and colon-separated names: `data-id`, `aria-label`, `client:load`, `class:active`
    loop {
        match reader.peek() {
            Some(TokenTree::Punct(p)) if p.as_char() == '-' || p.as_char() == ':' => {
                let sep = p.as_char();
                // Lookahead: is this `- ident` or `: ident`?
                match reader.peek_nth(1) {
                    Some(TokenTree::Ident(_)) => {
                        reader.next(); // consume separator
                        let next = reader.expect_ident()?;
                        name.push(sep);
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

fn parse_attr_value(reader: &mut TokenReader) -> Result<RsxAttrValue> {
    match reader.peek() {
        Some(TokenTree::Literal(_)) => {
            let lit = match reader.next().unwrap() {
                TokenTree::Literal(l) => l,
                _ => unreachable!(),
            };
            let span = lit.span();
            let repr = lit.to_string();
            if repr.starts_with('"') && repr.ends_with('"') && repr.len() >= 2 {
                let inner = unescape_string_literal(&repr[1..repr.len() - 1]);
                Ok(RsxAttrValue::Str(inner, span))
            } else {
                let stream = quote::quote!(#lit);
                Ok(RsxAttrValue::Expr(stream, span))
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

    // Allow hyphenated tag names: `my-component`
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
            _ => break,
        }
    }

    Ok((name, span))
}

fn unescape_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                Some('0') => out.push('\0'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}
