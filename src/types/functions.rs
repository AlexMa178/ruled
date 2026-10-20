use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::fmt::{ Debug, Formatter, Result as FmtResult };

use crate::parser::Expr;
use crate::state::{ Scope, State };
use crate::types::{ DataType, DataValue, Type, Value };

#[derive(Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub ret_type: Box<DataType>,
    pub arg_types: Vec<DataType>,
}
impl Type for FunctionType {
    fn debug_type_name(&self) -> String {
        format!("<{}{}>", self.ret_type.debug_type_name(), self.arg_types.iter().map(|t| format!(",{}", Type::debug_type_name(t))).collect::<String>())
    }
    fn accepts(&self, other: DataType) -> bool {
        other.assume_function().map_or(false, |other_f|
            self.ret_type.accepts((*other_f.ret_type).clone()) &&
            self.arg_types.len() == other_f.arg_types.len() &&
            self.arg_types.iter().zip(&other_f.arg_types).all(|(s, o)| s.accepts(o.clone()))
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FunctionValue {
    pub name: String,
    pub exprs: Vec<Expr>,
    pub param_names: Vec<String>,
    pub containing_scope: Rc<RefCell<Scope>>,
    pub f_type: FunctionType,
}
impl FunctionValue {

    pub fn run(self: &Rc<Self>, arg_vals: &[DataValue], state: &mut State) -> Result<(), String> {
        state.push_scope(Some(self.clone()));
        writeln!(state.trace_file(), "{}", self.name).expect("failed to write to trace");
        if self.param_names.len() != arg_vals.len() { return Err(format!("function has {} parameter(s) {:?}, but {} argument(s) were supplied", self.param_names.len(), self.param_names, arg_vals.len())); }
        for ((param_name, arg_val), expected_arg_type) in self.param_names.iter().zip(arg_vals).zip(self.f_type.arg_types.clone()) {
            if !expected_arg_type.either_accepts(arg_val.get_type()) { return Err(format!("the argument for parameter \"{param_name}\" has the wrong type in \"{:?}\": neither \"{}\" nor \"{}\" accepts the other", self, expected_arg_type.debug_type_name(), arg_val.get_type().debug_type_name())); }
            state.set_ret(arg_val.clone());
            state.move_ret(param_name, &self.f_type.ret_type)?;
        }
        for expr in &self.exprs {
            expr.run(state)?;
        }
        state.pop_scope();
        Ok(())
    }

}
impl Value for FunctionValue {
    type Type = FunctionType;
    fn get_type(&self) -> Self::Type {
        self.f_type.clone()
    }
}
impl Debug for FunctionValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "F{{{}}}({})::{}", self.name, self.param_names.join(","), self.f_type.debug_type_name())
    }
}