use std::iter;

use crate::type_names::{DefinedTypeName, TypeName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    tokens: Vec<Token>
}
impl Tokens {

    pub fn new(tokens: &[ Token ]) -> Self {
        let mut tokens = tokens.to_vec();
        tokens.reverse();
        Self { tokens }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn peek(&self) -> Result<Token, String> {
        self.tokens.last().cloned().ok_or("unexpected end of tokens".to_owned())
    }

    pub fn next(&mut self) -> Result<Token, String> {
        self.tokens.pop().ok_or("unexpected end of tokens".to_owned())
    }

    pub fn expect_eq(&mut self, token: Token) -> Result<(), String> {
        let next = self.next()?;
        if next == token { Ok(()) } else { Err(format!("expected token \"{token:?}\", found token \"{next:?}\"")) }
    }

    pub fn next_word(&mut self) -> Result<String, String> {
        let next = self.next()?;
        match next { Token::Word(s) => Ok(s), _ => Err(format!("expected word, found \"{next:?}\"")) }
    }

    pub fn next_defined_type_name(&mut self) -> Result<DefinedTypeName, String> {
        let next = self.next()?;
        match next { Token::DefinedTypeName(t) => Ok(t), _ => Err(format!("expected defined type name, found \"{next:?}\"")) }
    }

    pub fn next_type_name(&mut self) -> Result<TypeName, String> {
        let next = self.next()?;
        match next {
            Token::DefinedTypeName(n) => Ok(n.into()),
            Token::TypeName(n) => Ok(n),
            _ => Err(format!("expected type name, found \"{next:?}\"")),
        }
    }

    pub fn next_value(&mut self) -> Result<ValueToken, String> {
        let next = self.next()?;
        match next { Token::Value(v) => Ok(v), _ => Err(format!("expected value, found \"{next:?}\"")) }
    }

    pub fn next_colon(&mut self) -> Result<(String, TypeName), String> {
        let next = self.next()?;
        match next { Token::Colon(a, b) => Ok((a, b)), _ => Err(format!("expected colon, found \"{next:?}\"")) }
    }

    pub fn next_semicolon(&mut self) -> Result<(String, ValueToken), String> {
        let next = self.next()?;
        match next { Token::Semicolon(a, b) => Ok((a, b)), _ => Err(format!("expected semicolon, found \"{next:?}\"")) }
    }

    pub fn all_as<T, F: Fn(&mut Tokens) -> Result<T, String>>(mut self, f: F) -> Result<Vec<T>, String> {
        let mut tokens = Vec::new();
        while !self.is_empty() {
            tokens.push(f(&mut self)?);
        }
        Ok(tokens)
    }

    pub fn until_end_of_line(&mut self) -> Result<Tokens, String> {
        let mut tokens = Vec::new();
        while let Some(next) = match self.next()? { Token::EndOfLine => None, e => Some(e) } {
            tokens.push(next);
        }
        Ok(Tokens::new(&tokens))
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Enum,
    Union,
    Struct,
    Fn,
    Scope,
    Break,
    MatchEnum,
    MatchUnion,
    Put,
    Ask,
    Get,
    Colon(String, TypeName),
    Semicolon(String, ValueToken),
    Word(String),
    DefinedTypeName(DefinedTypeName),
    TypeName(TypeName),
    Value(ValueToken),
    EndFn,
    EndOfLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueToken {
    EnumVariant(String),
    UnionVariant(String, Box<ValueToken>),
    Variable(String),
    StructAccess(String, Vec<String>),
}

const VALUE_SYMBOL: [char; 3] = [ '$', '#', '%' ];

pub fn lexer(s: &str) -> Result<Tokens, String> {

    let mut tokens = Vec::new();
    let mut maybe_curr_indent = None;
    let preprocessed = s.lines().map(|line| if let Some(contents) = line.strip_suffix('\\') { contents.to_owned() } else { format!("{line}\n") }).collect::<String>();
    for line in preprocessed.lines() {

        if line.is_empty() || line.trim_start().starts_with("--") { continue; }
        
        let line_indent = determine_indent(line);
        if let Some(curr_indent) = maybe_curr_indent {
            tokens.push(Token::EndOfLine);
            if line_indent < curr_indent {
                for _ in 0..(curr_indent - line_indent) { tokens.push(Token::EndFn); }
            }
        }
        maybe_curr_indent = Some(line_indent);
        
        tokens.extend(line.split_ascii_whitespace().map(lex_word).collect::<Result<Vec<_>, _>>()?);

    }
    tokens.push(Token::EndOfLine);
    Ok(Tokens::new(&tokens))

}

fn determine_indent(line: &str) -> u32 {
    fn inner(indent: u32, line: &str) -> u32 {
        line.strip_prefix("    ").map_or(indent, |new_line| inner(indent + 1, new_line))
    }
    inner(0, line)
}

fn lex_word(word: &str) -> Result<Token, String> {
    Ok(match word {
        "enum" => Token::Enum,
        "union" => Token::Union,
        "struct" => Token::Struct,
        "fn" => Token::Fn,
        "scope" => Token::Scope,
        "break" => Token::Break,
        "#" => Token::MatchEnum,
        "%" => Token::MatchUnion,
        ">" => Token::Put,
        "<" => Token::Get,
        "^" => Token::Ask,
        w if let Some((a, b)) = w.split_once(':') => Token::Colon(a.to_owned(), TypeName::new(b)?),
        w if let Some((a, b)) = w.split_once(';') => Token::Semicolon(a.to_owned(), lex_value_token(b)?),
        w if let Ok(t) = DefinedTypeName::new(w) => Token::DefinedTypeName(t),
        w if let Ok(t) = TypeName::new(w) => Token::TypeName(t),
        w if VALUE_SYMBOL.into_iter().any(|sym| w.starts_with(sym)) => Token::Value(lex_value_token(w)?),
        s => Token::Word(s.to_owned()),
    })
}

fn lex_value_token(val_str: &str) -> Result<ValueToken, String> {
    Ok(match val_str {
        w if let Some(v) = w.strip_prefix('#') => ValueToken::EnumVariant(v.to_owned()),
        w if let Some(v) = w.strip_prefix('%') => {
            let mut split = v.split_inclusive(VALUE_SYMBOL);
            let name_and_sym = split.next().ok_or(format!("no name for % in %\"{v}\""))?;
            let mut rev_name_and_sym = name_and_sym.chars().rev();
            let sym = rev_name_and_sym.next().ok_or(format!("no symbol for % in %\"{v}\""))?;
            let name = rev_name_and_sym.rev().collect::<String>();
            let val = iter::once(String::from(sym).as_str()).chain(split).collect::<String>();
            ValueToken::UnionVariant(name, Box::new(lex_value_token(&val)?))
        }
        w if let Some(v) = w.strip_prefix('$') =>
            if v.contains('.') {
                let parts = v.split('.').map(|s| s.to_owned()).collect::<Vec<_>>();
                let (var, fields) = parts.split_first().ok_or("empty struct access")?;
                ValueToken::StructAccess(var.to_owned(), fields.to_owned())
            } else {
                ValueToken::Variable(v.to_owned())
            },
        _ => return Err(format!("could not lex \"{val_str}\" as a value token")),
    })
}