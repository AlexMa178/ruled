use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::fmt::{ Debug, Formatter, Result as FmtResult };

use crate::state::Scope;
use crate::type_names::{DefinedTypeName, TypeName};
use crate::types::{ DataType, DataValue, DefinedType, Type, Value };

#[derive(Clone, PartialEq, Eq)]
pub struct StructType {
    name: DefinedTypeName,
    fields: Vec<(String, DataType)>,
}
impl StructType {

    pub fn new(name: &DefinedTypeName, fields: &[(&str, &TypeName)], scope: impl Deref<Target = Scope>) -> Result<Self, String> {
        Ok(Self {
            name: name.clone(),
            fields: fields.iter().map(|&(name, type_name)| Ok((
                name.to_owned(),
                type_name.resolve(&scope)?,
            ))).collect::<Result<Vec<_>, String>>()?,
        })
    }

    fn field_index(&self, field_name: &str) -> Result<usize, String> {
        self.fields.iter().position(|(f_n, _)| f_n == field_name).ok_or(format!("no field \"{field_name}\" in \"{}\"", self.debug_type_details()))
    }

}
impl DefinedType for StructType {
    fn type_name(&self) -> DefinedTypeName {
        self.name.clone()
    }
    fn debug_type_details(&self) -> String {
        format!("{}{{{}}}", self.name.string(), self.fields.iter().map(|(f_n, f_t)| format!("{f_n}:{}", f_t.debug_type_name())).collect::<Vec<_>>().join(","))
    }
    fn add_to_scope(self, mut scope: impl DerefMut<Target = Scope>) -> Result<(), String> {
        (*scope).add_struct_type(self)
    }
    fn get_from_scope(name: &DefinedTypeName, scope: impl Deref<Target = Scope>) -> Result<Rc<Self>, String> {
        scope.get_struct_type_from_name(name)
    }
}
impl Type for StructType {
    fn debug_type_name(&self) -> String {
        self.type_name().string()
    }
    fn accepts(&self, other: DataType) -> bool {
        other.assume_struct().map_or(false, |other_s| *self == *other_s)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct StructValue {
    field_values: Vec<DataValue>,
    s_type: Rc<StructType>,
}
impl StructValue {

    pub fn get_field(&self, field_name: &str) -> Result<DataValue, String> {
        Ok(self.field_values[ self.s_type.field_index(field_name)? ].clone())
    }

}
impl Value for StructValue {
    type Type = Rc<StructType>;
    fn get_type(&self) -> Self::Type {
        self.s_type.clone()
    }
}
impl Debug for StructValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let s = self.s_type.fields.iter().zip(&self.field_values).map(|((f, _), v)| format!("{f}={v:?}")).collect::<Vec<_>>().join(",");
        write!(f, "S[{s}]::{}", self.s_type.type_name().string())
    }
}

pub struct StructValueBuilder {
    field_values: Vec<Option<DataValue>>,
    s_type: Rc<StructType>,
}
impl StructValueBuilder {

    pub fn new(s_type: Rc<StructType>) -> Self {
        Self { field_values: s_type.fields.iter().map(|_| None).collect(), s_type }
    }

    pub fn set(&mut self, field_name: &str, val: DataValue) -> Result<(), String> {
        let i = self.s_type.field_index(field_name)?;
        if self.field_values[i].is_some() { return Err(format!("field \"{field_name}\" already set")); }
        if !self.s_type.fields[i].1.accepts(val.get_type()) { return Err(format!("field \"{field_name}\" has wrong type")); }
        self.field_values[i] = Some(val);
        Ok(())
    }

    pub fn build(self) -> Result<StructValue, String> {
         let field_values = self.field_values.into_iter().collect::<Option<Vec<_>>>().ok_or("not all fields initialized")?;
         Ok(StructValue { field_values, s_type: self.s_type })
    }

}