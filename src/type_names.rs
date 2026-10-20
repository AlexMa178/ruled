use std::collections::HashMap;
use std::sync::LazyLock;

use crate::types::{ DataType, DefinedType, EnumType, FunctionType, QuestionMarkType, StructType, UnionType };
use crate::state::Scope;

#[derive(Clone)]
enum TypeNameToken {
    Name(String),
    Comma,
    OpenBracket,
    CloseBracket,
}

static SYMBOLS: LazyLock<HashMap<char, TypeNameToken>> = LazyLock::new(|| HashMap::from([
    (',', TypeNameToken::Comma),
    ('<', TypeNameToken::OpenBracket),
    ('>', TypeNameToken::CloseBracket),
]));

fn type_name_lexer(s: &str) -> Vec<TypeNameToken> {
    
    let mut tokens = Vec::new();
    let mut maybe_curr_name = None;
    for c in s.chars() {
        if let Some(token) = SYMBOLS.get(&c) {
            if let Some(curr_name) = maybe_curr_name.take() {
                tokens.push(TypeNameToken::Name(curr_name));
            }
            tokens.push(token.clone());
        } else {
            let curr_name = maybe_curr_name.get_or_insert(String::new());
            curr_name.push(c);
        }
    }
    if let Some(curr_name) = maybe_curr_name.take() {
        tokens.push(TypeNameToken::Name(curr_name));
    }
    tokens

}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeNameTree {
    Flat(FlatTypeName),
    Fn(Vec<TypeNameTree>),
}

fn type_name_parser<I: Iterator<Item = TypeNameToken>>(tokens: &mut I) -> Result<TypeNameTree, String> {

    Ok(match tokens.next().ok_or("unexpected end of type name")? {
        TypeNameToken::Name(s) => TypeNameTree::Flat(FlatTypeName::new(&s)?),
        TypeNameToken::OpenBracket => {
            let mut list = Vec::new();
            loop {
                list.push(type_name_parser(tokens)?);
                match tokens.next().ok_or("unexpected end of function type list")? {
                    TypeNameToken::Comma => continue,
                    TypeNameToken::CloseBracket => break,
                    TypeNameToken::Name(_) | TypeNameToken::OpenBracket => Err("unexpected value in function type list")?,
                }
            }
            TypeNameTree::Fn(list)
        },
        TypeNameToken::Comma | TypeNameToken::CloseBracket => Err("unexpected value in type name")?,
    })

}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefinedTypeName {
    string: String
}
impl DefinedTypeName {

    pub fn new(s: &str) -> Result<Self, String> {
        let chars = s.chars().collect::<Vec<_>>();
        if !chars.get(0).map_or(false, |c| c.is_ascii_uppercase()) { return Err(format!("defined type name \"{s}\" does not start with an uppercase letter")); }
        if !chars.iter().all(|c| c.is_ascii_alphanumeric() || *c == '_') { return Err(format!("defined type name \"{s}\" contains illegal characters")); }
        Ok(Self { string: s.to_owned() })
    }

    pub fn string(&self) -> String {
        self.string.clone()
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FlatTypeName {
    Question,
    Defined(DefinedTypeName),
}
impl FlatTypeName {

    pub fn new(s: &str) -> Result<Self, String> {
        Ok(match s {
            "?" => Self::Question,
            _ => Self::Defined(DefinedTypeName::new(s)?)
        })
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeName {
    tree: TypeNameTree,
}
impl TypeName {

    pub fn new(s: &str) -> Result<Self, String> {
        let tokens = type_name_lexer(s);
        let mut token_iter = tokens.into_iter();
        let tree = type_name_parser(&mut token_iter)?;
        Ok(Self { tree })
    }

    pub fn resolve(&self, scope: &Scope) -> Result<DataType, String> {

        fn tree_to_type(t: &TypeNameTree, scope: &Scope) -> Result<DataType, String> {
            match t {
                TypeNameTree::Flat(flat) => {
                    match flat {
                        FlatTypeName::Question => Ok(DataType::Q(QuestionMarkType)),
                        FlatTypeName::Defined(name) =>
                            if let Ok(e) = EnumType::get_from_scope(name, scope) {
                                Ok(DataType::E(e))
                            } else if let Ok(s) = StructType::get_from_scope(name, scope) {
                                Ok(DataType::S(s))
                            } else if let Ok(u) = UnionType::get_from_scope(name, scope) {
                                Ok(DataType::U(u))
                            } else {
                                Err(format!("could not find type \"{}\"", name.string()))
                            },
                    }
                },
                TypeNameTree::Fn(type_name_trees) => {
                    let types = type_name_trees.iter().map(|t| tree_to_type(t, scope)).collect::<Result<Vec<_>, _>>()?;
                    let (ret, args) = types.split_first().ok_or("function type must have at least one type")?;
                    Ok(DataType::F(FunctionType { ret_type: Box::new(ret.clone()), arg_types: args.to_vec() }))
                },
            }
        }

        tree_to_type(&self.tree, scope)

    }

}
impl From<DefinedTypeName> for TypeName {
    fn from(value: DefinedTypeName) -> Self {
        Self { tree: TypeNameTree::Flat(FlatTypeName::Defined(value)) }
    }
}