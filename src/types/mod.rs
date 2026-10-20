mod enums;
mod unions;
mod structs;
mod functions;

use std::ops::{ Deref, DerefMut };
use std::rc::Rc;
use std::fmt::{ Debug, Formatter, Result as FmtResult };

use crate::state::Scope;
use crate::type_names::DefinedTypeName;
pub use crate::types::enums::*;
pub use crate::types::unions::*;
pub use crate::types::structs::*;
pub use crate::types::functions::*;

pub trait DefinedType: Sized {
    fn type_name(&self) -> DefinedTypeName;
    fn debug_type_details(&self) -> String;
    fn add_to_scope(self, scope: impl DerefMut<Target = Scope>) -> Result<(), String>;
    fn get_from_scope(name: &DefinedTypeName, scope: impl Deref<Target = Scope>) -> Result<Rc<Self>, String>;
}

pub trait Type {
    fn debug_type_name(&self) -> String;
    fn accepts(&self, other: DataType) -> bool;
}

pub trait Value {
    type Type;
    fn get_type(&self) -> Self::Type;
}

#[derive(Clone, PartialEq, Eq)]
pub enum DataType {
    E(Rc<EnumType>),
    U(Rc<UnionType>),
    S(Rc<StructType>),
    F(FunctionType),
    Q(QuestionMarkType),
}
impl DataType {

    pub fn assume_enum(self) -> Result<Rc<EnumType>, String> {
        match self { Self::E(e) => Ok(e), _ => Err("type must be an enum type".to_owned()) }
    }

    pub fn assume_union(self) -> Result<Rc<UnionType>, String> {
        match self { Self::U(u) => Ok(u), _ => Err("type must be a union type".to_owned()) }
    }

    pub fn assume_struct(self) -> Result<Rc<StructType>, String> {
        match self { Self::S(s) => Ok(s), _ => Err("type must be a struct type".to_owned()) }
    }

    pub fn assume_function(self) -> Result<FunctionType, String> {
        match self { Self::F(f) => Ok(f), _ => Err("type must be a function type".to_owned()) }
    }

    pub fn as_type(&self) -> Box<dyn Type> {
        match self {
            Self::E(e) => Box::from((**e).clone()),
            Self::U(u) => Box::from((**u).clone()),
            Self::S(s) => Box::from((**s).clone()),
            Self::F(f) => Box::from((*f).clone()),
            Self::Q(q) => Box::from(*q),
        }
    }

    pub fn either_accepts(&self, other: DataType) -> bool {
        other.accepts(self.clone()) || self.accepts(other)
    }

}
impl Type for DataType {
    fn debug_type_name(&self) -> String {
        self.as_type().debug_type_name()
    }
    fn accepts(&self, other: DataType) -> bool {
        self.as_type().accepts(other)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum DataValue {
    E(EnumValue),
    U(UnionValue),
    S(StructValue),
    F(Rc<FunctionValue>),
}
impl DataValue {

    pub fn assume_enum(self) -> Result<EnumValue, String> {
        match self { Self::E(e) => Ok(e), _ => Err(format!("value must be an enum value, found {self:?}")) }
    }

    pub fn assume_union(self) -> Result<UnionValue, String> {
        match self { Self::U(u) => Ok(u), _ => Err(format!("value must be a union value, found {self:?}")) }
    }

    pub fn assume_struct(self) -> Result<StructValue, String> {
        match self { Self::S(s) => Ok(s), _ => Err(format!("value must be a struct value, found {self:?}", )) }
    }

    pub fn assume_function(self) -> Result<Rc<FunctionValue>, String> {
        match self { Self::F(f) => Ok(f), _ => Err(format!("value must be a function value, found {self:?}")) }
    }

}
impl Value for DataValue {
    type Type = DataType;
    fn get_type(&self) -> Self::Type {
        match self {
            Self::E(e) => DataType::E(e.get_type()),
            Self::U(u) => DataType::U(u.get_type()),
            Self::S(s) => DataType::S(s.get_type()),
            Self::F(f) => DataType::F(f.get_type()),
        }
    }
}
impl Debug for DataValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", match self {
            Self::E(e) => format!("{e:?}"),
            Self::U(u) => format!("{u:?}"),
            Self::S(s) => format!("{s:?}"),
            Self::F(f) => format!("{f:?}"),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct QuestionMarkType;
impl Type for QuestionMarkType {
    fn debug_type_name(&self) -> String {
        "?".to_owned()
    }
    fn accepts(&self, _other: DataType) -> bool {
        true
    }
}