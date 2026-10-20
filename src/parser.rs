use std::collections::HashMap;
use std::io::{ self, Write };
use std::rc::Rc;

use crate::type_names::{DefinedTypeName, TypeName};
use crate::types::{ DataType, DataValue, DefinedType, EnumMatchBuilder, EnumType, EnumValue, FunctionType, FunctionValue, QuestionMarkType, StructType, StructValueBuilder, Type, UnionType, Value };
use crate::state::State;
use crate::lexer::{ Token, Tokens, ValueToken };

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    EnumDefLine(EnumDefLine),
    UnionDefLine(UnionDefLine),
    StructDefLine(StructDefLine),
    MatchEnumLine(MatchEnumLine),
    MatchUnionLine(MatchUnionLine),
    ConstructorLine(ConstructorLine),
    PutLine(PutLine),
    AskLine(AskLine),
    GetLine(GetLine),
    FnBlock(FnBlock),
    ScopeBlock(ScopeBlock),
    BreakLine(BreakLine),
    CallLine(CallLine),
}
impl Expr {

    pub fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        if EnumDefLine    ::matches(tokens.peek()?) { return Ok(Self::EnumDefLine    (EnumDefLine    ::from_tokens(tokens)?)); }
        if UnionDefLine   ::matches(tokens.peek()?) { return Ok(Self::UnionDefLine   (UnionDefLine   ::from_tokens(tokens)?)); }
        if StructDefLine  ::matches(tokens.peek()?) { return Ok(Self::StructDefLine  (StructDefLine  ::from_tokens(tokens)?)); }
        if MatchEnumLine  ::matches(tokens.peek()?) { return Ok(Self::MatchEnumLine  (MatchEnumLine  ::from_tokens(tokens)?)); }
        if MatchUnionLine ::matches(tokens.peek()?) { return Ok(Self::MatchUnionLine (MatchUnionLine ::from_tokens(tokens)?)); }
        if ConstructorLine::matches(tokens.peek()?) { return Ok(Self::ConstructorLine(ConstructorLine::from_tokens(tokens)?)); }
        if PutLine        ::matches(tokens.peek()?) { return Ok(Self::PutLine        (PutLine        ::from_tokens(tokens)?)); }
        if AskLine        ::matches(tokens.peek()?) { return Ok(Self::AskLine        (AskLine        ::from_tokens(tokens)?)); }
        if GetLine        ::matches(tokens.peek()?) { return Ok(Self::GetLine        (GetLine        ::from_tokens(tokens)?)); }
        if FnBlock        ::matches(tokens.peek()?) { return Ok(Self::FnBlock        (FnBlock        ::from_tokens(tokens)?)); }
        if ScopeBlock     ::matches(tokens.peek()?) { return Ok(Self::ScopeBlock     (ScopeBlock     ::from_tokens(tokens)?)); }
        if BreakLine      ::matches(tokens.peek()?) { return Ok(Self::BreakLine      (BreakLine      ::from_tokens(tokens)?)); }
        Ok(Self::CallLine(CallLine::from_tokens(tokens)?))
    }

    pub fn run(&self, state: &mut State) -> Result<(), String> {
        match self {
            Self::EnumDefLine    (e) => e.run(state),
            Self::UnionDefLine   (e) => e.run(state),
            Self::StructDefLine  (e) => e.run(state),
            Self::MatchEnumLine  (e) => e.run(state),
            Self::MatchUnionLine (e) => e.run(state),
            Self::ConstructorLine(e) => e.run(state),
            Self::PutLine        (e) => e.run(state),
            Self::AskLine        (e) => e.run(state),
            Self::GetLine        (e) => e.run(state),
            Self::FnBlock        (e) => e.run(state),
            Self::ScopeBlock     (e) => e.run(state),
            Self::BreakLine      (e) => e.run(state),
            Self::CallLine       (e) => e.run(state),
        }
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDefLine {
    name: DefinedTypeName, variants: Vec<String>
}
impl EnumDefLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Enum
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Enum)?;
        let name = tokens.next_defined_type_name()?;
        let variants = tokens.until_end_of_line()?.all_as(Tokens::next_word)?;
        Ok(Self { name, variants })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        EnumType::new(&self.name, self.variants.iter().map(String::as_str).collect::<Box<[_]>>().as_ref())?
            .add_to_scope(state.scope().borrow_mut())
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionDefLine {
    name: DefinedTypeName, variants: Vec<(String, TypeName)>
}
impl UnionDefLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Union
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Union)?;
        let name = tokens.next_defined_type_name()?;
        let variants = tokens.until_end_of_line()?.all_as(Tokens::next_colon)?;
        Ok(Self { name, variants })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        UnionType::new(&self.name, self.variants.iter().map(|(n, t)| (n.as_str(), t)).collect::<Box<[_]>>().as_ref(), state.scope().borrow())?
            .add_to_scope(state.scope().borrow_mut())
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDefLine {
    name: DefinedTypeName, fields: Vec<(String, TypeName)>
}
impl StructDefLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Struct
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Struct)?;
        let name = tokens.next_defined_type_name()?;
        let fields = tokens.until_end_of_line()?.all_as(Tokens::next_colon)?;
        Ok(Self { name, fields })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        StructType::new(&self.name, self.fields.iter().map(|(n, t)| (n.as_str(), t)).collect::<Box<[_]>>().as_ref(), state.scope().borrow())?
            .add_to_scope(state.scope().borrow_mut())

    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchEnumLine {
    ret_name: String, val_str: ValueToken, map: Vec<(String, ValueToken)>
}
impl MatchEnumLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::MatchEnum
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::MatchEnum)?;
        let ret_name = tokens.next_word()?;
        let val_str = tokens.next_value()?;
        let map = tokens.until_end_of_line()?.all_as(Tokens::next_semicolon)?;
        Ok(Self { ret_name, val_str, map })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let val = state.scope().borrow().value_string(&self.val_str)?.assume_enum()?;
        let mut match_builder = EnumMatchBuilder::new(val.get_type());
        for (map_variant_str, map_val_str) in &self.map {
            let map_val = state.scope().borrow().value_string(map_val_str)?;
            match_builder.add_entry(map_variant_str, map_val)?;
        }
        state.set_ret(match_builder.build()?[&val].clone());
        state.move_ret(&self.ret_name, &DataType::Q(QuestionMarkType))
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchUnionLine {
    ret_name: String, val_str: ValueToken, map: Vec<(String, ValueToken)>
}
impl MatchUnionLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::MatchUnion
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::MatchUnion)?;
        let ret_name = tokens.next_word()?;
        let val_str = tokens.next_value()?;
        let map = tokens.until_end_of_line()?.all_as(Tokens::next_semicolon)?;
        Ok(Self { ret_name, val_str, map })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let val = state.scope().borrow().value_string(&self.val_str)?.assume_union()?;
        let mut cases = HashMap::new();
        let mut fn_res_type = None;
        for (variant_str, param_str) in &self.map {
            let (variant_index, variant_type) = val.u_type.variant(variant_str)?;
            let param = state.scope().borrow().value_string(param_str)?.assume_function()?;
            let &[ref arg_type] = param.f_type.arg_types.as_slice() else { return Err(format!("union match function must have only one argument, \"{param_str:?}\" does not")); };
            if !arg_type.accepts(variant_type.clone()) { return Err(format!("\"{param_str:?}\" has wrong argument type for union variant: \"{}\" does not accept \"{}\"", arg_type.debug_type_name(), variant_type.debug_type_name())); }
            if !fn_res_type.get_or_insert(param.f_type.ret_type.clone()).either_accepts((*param.f_type.ret_type).clone()) { return Err(format!("\"{param_str:?}\" has wrong type in union match")); }
            let previous = cases.insert(variant_index, param);
            if previous.is_some() { return Err(format!("variant \"{variant_str}\" appears more an once in union match")); }
        }
        if !val.u_type.all_indices().iter().all(|v| cases.contains_key(v)) { return Err(format!("non-exhaustive union match")); }
        cases[&val.variant_index].run(&[*val.value], state)?;
        state.move_ret(&self.ret_name, &fn_res_type.map_or(DataType::Q(QuestionMarkType), |t| *t))
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructorLine {
    type_name: DefinedTypeName, ret_name: String, fields: Vec<(String, ValueToken)>
}
impl ConstructorLine {

    fn matches(next_token: Token) -> bool {
        matches!(next_token, Token::DefinedTypeName(_))
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        let type_name = tokens.next_defined_type_name()?;
        let ret_name = tokens.next_word()?;
        let fields = tokens.until_end_of_line()?.all_as(Tokens::next_semicolon)?;
        Ok(Self { type_name, ret_name, fields })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let struct_type = state.scope().borrow().get_struct_type_from_name(&self.type_name)?;
        let mut struct_builder = StructValueBuilder::new(struct_type.clone());
        for (init_field_str, init_val_str) in &self.fields {
            let init_val = state.scope().borrow().value_string(init_val_str)?;
            struct_builder.set(init_field_str, init_val)?;
        }
        let res = struct_builder.build()?;
        state.set_ret(DataValue::S(res));
        state.move_ret(&self.ret_name, &DataType::S(struct_type))
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutLine {
    c: ValueToken
}
impl PutLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Put
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Put)?;
        let c = tokens.next_value()?;
        tokens.expect_eq(Token::EndOfLine)?;
        Ok(Self { c })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let c_val = state.scope().borrow().value_string(&self.c)?.assume_enum()?;
        let c_i = c_val.variant_index();
        let c_b = if c_i < 128 { c_i as u8 } else { return Err(format!("\"{c_val:?}\" has index \"{c_i}\" which is not ASCII")); };
        print!("{}", c_b as char);
        io::stdout().flush().expect("failed to flush");
        Ok(())
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskLine;
impl AskLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Ask
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Ask)?;
        tokens.expect_eq(Token::EndOfLine)?;
        Ok(Self)
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok().ok_or("failed to read input".to_owned())?;
        state.new_input(input);
        Ok(())
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLine {
    ret_name: String, type_name: DefinedTypeName
}
impl GetLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Get
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Get)?;
        let ret_name = tokens.next_word()?;
        let type_name = tokens.next_defined_type_name()?;
        tokens.expect_eq(Token::EndOfLine)?;
        Ok(Self { ret_name, type_name })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let variant_index = state.next_input_char();
        let e_type = state.scope().borrow().get_enum_type_from_name(&self.type_name)?;
        state.set_ret(DataValue::E(EnumValue::new(variant_index, e_type.clone())?));
        state.move_ret(&self.ret_name, &DataType::E(e_type))?;
        Ok(())
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnBlock {
    fn_name: String, ret_type_str: TypeName, param_strs: Vec<(String, TypeName)>, exprs: Vec<Expr>
}
impl FnBlock {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Fn
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Fn)?;
        let fn_name = tokens.next_word()?;
        let ret_type_str = tokens.next_type_name()?;
        let arg_strs = tokens.until_end_of_line()?.all_as(Tokens::next_colon)?;
        let mut exprs = Vec::new();
        loop {
            exprs.push(Expr::from_tokens(tokens)?);
            match tokens.peek() {
                Err(_) => break,
                Ok(Token::EndFn) => { tokens.expect_eq(Token::EndFn)?; break; },
                _ => continue,
            }
        }
        Ok(Self { fn_name, ret_type_str, param_strs: arg_strs, exprs })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let ret_type = self.ret_type_str.resolve(&state.scope().borrow())?;
        let args = self.param_strs.iter().map(|(n, t)| t.resolve(&state.scope().borrow()).map(|t| (n, t))).collect::<Result<Vec<_>, _>>()?;
        let arg_names = args.iter().map(|(n, _)| (*n).to_owned()).collect();
        let arg_types = args.into_iter().map(|(_, t)| t).collect();
        let f_type = FunctionType { ret_type: Box::new(ret_type), arg_types };
        let f_val = FunctionValue { name: self.fn_name.clone(), exprs: self.exprs.clone(), param_names: arg_names, containing_scope: state.scope().clone(), f_type: f_type.clone() };
        state.set_ret(DataValue::F(Rc::new(f_val)));
        state.move_ret(&self.fn_name, &DataType::F(f_type))
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeBlock {
    ret_name: String, exprs: Vec<Expr>
}
impl ScopeBlock {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Scope
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Scope)?;
        let ret_name = tokens.next_word()?;
        tokens.expect_eq(Token::EndOfLine)?;
        let mut exprs = Vec::new();
        loop {
            exprs.push(Expr::from_tokens(tokens)?);
            match tokens.peek() {
                Err(_) => break,
                Ok(Token::EndFn) => { tokens.expect_eq(Token::EndFn)?; break; },
                _ => continue,
            }
        }
        Ok(Self { ret_name, exprs })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let f_type = FunctionType { ret_type: Box::new(DataType::Q(QuestionMarkType)), arg_types: Vec::new() };
        let f_val = FunctionValue { name: "*".to_owned(), exprs: self.exprs.clone(), param_names: Vec::new(), containing_scope: state.scope().clone(), f_type: f_type.clone() };
        let f_rc = Rc::new(f_val);
        f_rc.run(&[], state)?;
        state.move_ret(&self.ret_name, &f_type.ret_type)
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakLine;
impl BreakLine {

    fn matches(next_token: Token) -> bool {
        next_token == Token::Break
    }

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        tokens.expect_eq(Token::Break)?;
        tokens.expect_eq(Token::EndOfLine)?;
        Ok(Self)
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let str = state.scope().borrow().context_str();
        print!("BREAK{str}\n{state:?}");
        let _ = io::stdin().read_line(&mut String::new());
        Ok(())
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallLine {
    fn_name: String, ret_name: String, arg_strs: Vec<ValueToken>
}
impl CallLine {

    fn from_tokens(tokens: &mut Tokens) -> Result<Self, String> {
        let fn_name = tokens.next_word()?;
        let ret_name = tokens.next_word()?;
        let arg_strs = tokens.until_end_of_line()?.all_as(Tokens::next_value)?;
        Ok(Self { fn_name, ret_name, arg_strs })
    }

    fn run(&self, state: &mut State) -> Result<(), String> {
        let fn_val = state.scope().borrow().variable_string(&self.fn_name)?.assume_function()?;
        let arg_vals = self.arg_strs.iter().map(|v| state.scope().borrow().value_string(v)).collect::<Result<Vec<_>, _>>()?;
        fn_val.run(&arg_vals, state)?;
        let fallthrough_type = state.scope().borrow().containing_fn().map_or(DataType::Q(QuestionMarkType), |f| (*f.f_type.ret_type).clone());
        state.move_ret(&self.ret_name, &fallthrough_type)
    }

}