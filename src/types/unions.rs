use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::fmt::{ Debug, Formatter, Result as FmtResult };

use crate::state::Scope;
use crate::type_names::{DefinedTypeName, TypeName};
use crate::types::{ DataType, DataValue, DefinedType, Type, Value };

#[derive(Clone, PartialEq, Eq)]
pub struct UnionType {
    pub name: DefinedTypeName,
    pub variants: Vec<(String, DataType)>,
}
impl UnionType {

    pub fn new(name: &DefinedTypeName, variants: &[(&str, &TypeName)], scope: impl Deref<Target = Scope>) -> Result<Self, String> {
        Ok(Self {
            name: name.clone(),
            variants: variants.iter().map(|&(name, type_name)| Ok((
                name.to_owned(),
                type_name.resolve(&scope)?,
            ))).collect::<Result<Vec<_>, String>>()?,
        })
    }

    pub fn all_indices(&self) -> Vec<usize> {
        (0..self.variants.len()).collect()
    }

    pub fn variant(self: &Rc<Self>, variant: &str) -> Result<(usize, DataType), String> {
        let i = self.variants.iter().position(|(v, _)| v == variant).ok_or(format!("could not find variant %\"{variant}\""))?;
        Ok((i, self.variants[i].1.clone()))
    }

}
impl DefinedType for UnionType {
    fn type_name(&self) -> DefinedTypeName {
        self.name.clone()
    }
    fn debug_type_details(&self) -> String {
        format!("{}{{{}}}", self.name.string(), self.variants.iter().map(|(v_n, v_t)| format!("{v_n}:{}", v_t.debug_type_name())).collect::<Vec<_>>().join("|"))
    }
    fn add_to_scope(self, mut scope: impl DerefMut<Target = Scope>) -> Result<(), String> {
        (*scope).add_union_type(self)
    }
    fn get_from_scope(name: &DefinedTypeName, scope: impl Deref<Target = Scope>) -> Result<Rc<Self>, String> {
        scope.get_union_type_from_name(name)
    }
}
impl Type for UnionType {
    fn debug_type_name(&self) -> String {
        self.type_name().string()
    }
    fn accepts(&self, other: DataType) -> bool {
        other.assume_union().map_or(false, |other_u| *self == *other_u)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct UnionValue {
    pub variant_index: usize,
    pub value: Box<DataValue>,
    pub u_type: Rc<UnionType>,
}
impl Value for UnionValue {
    type Type = Rc<UnionType>;
    fn get_type(&self) -> Self::Type {
        self.u_type.clone()
    }
}
impl Debug for UnionValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "U[{}/{}={:?}]::{}", self.variant_index, self.u_type.variants[self.variant_index].0, self.value, self.u_type.type_name().string())
    }
}