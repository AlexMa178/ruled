use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::fmt::{ Debug, Formatter, Result as FmtResult };

use crate::state::Scope;
use crate::type_names::DefinedTypeName;
use crate::types::{ DataType, DataValue, DefinedType, Type, Value };

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EnumType {
    name: DefinedTypeName,
    variants: Vec<String>,
}
impl EnumType {

    pub fn new(name: &DefinedTypeName, variants: &[&str]) -> Result<Self, String> {
        Ok(Self { name: name.clone(), variants: variants.iter().map(|&v| v.to_owned()).collect() })
    }

    pub fn all_values(self: &Rc<Self>) -> Vec<EnumValue> {
        (0..self.variants.len()).map(|i| EnumValue { variant_index: i, e_type: Rc::clone(self) }).collect()
    }

    pub fn value_of(self: &Rc<Self>, variant_str: &str) -> Result<EnumValue, String> {
        let i = self.variants.iter().position(|v| v == variant_str).ok_or(format!("could not find variant #\"{variant_str}\""))?;
        Ok(EnumValue { variant_index: i, e_type: Rc::clone(self) })
    }

}
impl DefinedType for EnumType {
    fn type_name(&self) -> DefinedTypeName {
        self.name.clone()
    }
    fn debug_type_details(&self) -> String {
        format!("{}{{{}}}", self.name.string(), self.variants.join("|"))
    }
    fn add_to_scope(self, mut scope: impl DerefMut<Target = Scope>) -> Result<(), String> {
        (*scope).add_enum_type(self)
    }
    fn get_from_scope(name: &DefinedTypeName, scope: impl Deref<Target = Scope>) -> Result<Rc<Self>, String> {
        scope.get_enum_type_from_name(name)
    }
}
impl Type for EnumType {
    fn debug_type_name(&self) -> String {
        self.type_name().string()
    }
    fn accepts(&self, other: DataType) -> bool {
        other.assume_enum().map_or(false, |other_e| *self == *other_e)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EnumValue {
    variant_index: usize,
    e_type: Rc<EnumType>,
}
impl EnumValue {

    pub fn new(variant_index: usize, e_type: Rc<EnumType>) -> Result<Self, String> {
        let Some(_) = e_type.variants.get(variant_index) else { return Err(format!("\"{}\" does not have variant number \"{variant_index}\"", e_type.type_name().string())); };
        Ok(Self { variant_index, e_type })
    }

    pub fn variant_index(&self) -> usize {
        self.variant_index
    }

}
impl Value for EnumValue {
    type Type = Rc<EnumType>;
    fn get_type(&self) -> Self::Type {
        self.e_type.clone()
    }
}
impl Debug for EnumValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "E[{}/{}]::{}", self.variant_index, self.e_type.variants[self.variant_index], self.e_type.type_name().string())
    }
}

pub struct EnumMatchBuilder {
    e_type: Rc<EnumType>,
    maybe_ret_type: Option<DataType>,
    map: HashMap<EnumValue, DataValue>,
}
impl EnumMatchBuilder {

    pub fn new(e_type: Rc<EnumType>) -> Self {
        Self { e_type, maybe_ret_type: None, map: HashMap::new() }
    }

    pub fn add_entry(&mut self, variant_str: &str, val: DataValue) -> Result<(), String> {
        let ret_type = &*self.maybe_ret_type.get_or_insert(val.get_type());
        if !ret_type.accepts(val.get_type()) {
            if val.get_type().accepts(ret_type.clone()) {
                self.maybe_ret_type = Some(val.get_type());
            } else {
                return Err(format!("wrong type in enum match for #\"{variant_str}\", \"{}\" does not accept \"{}\"", ret_type.debug_type_name(), val.get_type().debug_type_name()));
            }
        }
        let variant = self.e_type.value_of(variant_str)?;
        let previous = self.map.insert(variant.clone(), val);
        if previous.is_some() { return Err(format!("variant \"{variant:?}\" appears more an once in enum match")); }
        Ok(())
    }

    pub fn build(self) -> Result<HashMap<EnumValue, DataValue>, String> {
        if !self.e_type.all_values().iter().all(|v| self.map.contains_key(v)) { return Err(format!("non-exhaustive enum match")); }
        Ok(self.map)
    }

}