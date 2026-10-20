use std::cell::RefCell;
use std::fmt::{ Debug, Formatter, Result as FmtResult };
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::collections::{ HashMap, VecDeque };

use crate::lexer::ValueToken;
use crate::type_names::DefinedTypeName;
use crate::types::{ DataType, DataValue, DefinedType, EnumType, EnumValue, FunctionValue, StructType, Type, UnionType, UnionValue, Value };

#[derive(Clone, PartialEq, Eq)]
pub struct Scope {
    enum_types: HashMap<DefinedTypeName, Rc<EnumType>>,
    union_types: HashMap<DefinedTypeName, Rc<UnionType>>,
    struct_types: HashMap<DefinedTypeName, Rc<StructType>>,
    vars: HashMap<String, DataValue>,
    containing_fn: Option<Rc<FunctionValue>>,
}
impl Scope {

    pub fn new(containing_fn: Option<Rc<FunctionValue>>) -> Self {
        Self { enum_types: HashMap::new(), union_types: HashMap::new(), struct_types: HashMap::new(), vars: HashMap::new(), containing_fn }
    }

    pub fn add_enum_type(&mut self, e: EnumType) -> Result<(), String> {
        let name = &e.type_name();
        let previous = self.enum_types.insert(name.clone(), Rc::new(e));
        if previous.is_some() { return Err(format!("enum \"{}\" already exists", name.string())); }
        Ok(())
    }

    pub fn add_union_type(&mut self, u: UnionType) -> Result<(), String> {
        let name = &u.type_name();
        let previous = self.union_types.insert(name.clone(), Rc::new(u));
        if previous.is_some() { return Err(format!("union \"{}\" already exists", name.string())); }
        Ok(())
    }

    pub fn add_struct_type(&mut self, s: StructType) -> Result<(), String> {
        let name = &s.type_name();
        let previous = self.struct_types.insert(name.clone(), Rc::new(s));
        if previous.is_some() { return Err(format!("struct \"{}\" already exists", name.string())); }
        Ok(())
    }

    pub fn containing_fn(&self) -> Option<Rc<FunctionValue>> {
        self.containing_fn.clone()
    }

    pub fn context_str(&self) -> String {
        let mut names = Vec::new();
        self.apply_deep(|s| { if let Some(f) = s.containing_fn() { names.push(f.name.clone()); }; None::<()> });
        if names.is_empty() { String::new() } else { format!(" {}", names.join(" : ")) }
    }

    pub fn apply_deep<R, F: FnMut(&Scope) -> Option<R>>(&self, mut f: F) -> Option<R> {
        if let Some(res) = f(self) { return Some(res); }
        match &self.containing_fn {
            Some(cf) => cf.containing_scope.borrow().apply_deep(f),
            None => None,
        }
    }

    pub fn get_enum_type_from_name(&self, name: &DefinedTypeName) -> Result<Rc<EnumType>, String> {
        self.apply_deep(|scope| scope.enum_types.get(name).cloned()).ok_or(format!("could not find enum type \"{}\"", name.string()))
    }

    pub fn get_union_type_from_name(&self, name: &DefinedTypeName) -> Result<Rc<UnionType>, String> {
        self.apply_deep(|scope| scope.union_types.get(name).cloned()).ok_or(format!("could not find union type \"{}\"", name.string()))
    }

    pub fn get_struct_type_from_name(&self, name: &DefinedTypeName) -> Result<Rc<StructType>, String> {
        self.apply_deep(|scope| scope.struct_types.get(name).cloned()).ok_or(format!("could not find struct type \"{}\"", name.string()))
    }

    pub fn value_string(&self, value_token: &ValueToken) -> Result<DataValue, String> {
        match value_token {
            ValueToken::EnumVariant(v) => self.enum_variant_string(&v).map(|ev| DataValue::E(ev)),
            ValueToken::UnionVariant(v, val) => self.union_variant_string(v, val).map(|uv| DataValue::U(uv)),
            ValueToken::Variable(v) => self.variable_string(&v),
            ValueToken::StructAccess(s, fs) => self.struct_access_string(s, fs.iter().map(|f| f.as_str()).collect::<Box<[_]>>().as_ref()),
        }
    }

    pub fn enum_variant_string(&self, variant_str: &str) -> Result<EnumValue, String> {
        self.apply_deep(|s| {
            for (_, e_type) in &s.enum_types {
                if let Ok(value) = e_type.value_of(variant_str) {
                    return Some(value);
                }
            };
            None
        }).ok_or(format!("could not find variant #\"{variant_str}\""))
    }

    pub fn union_variant_string(&self, variant_str: &str, value_token: &ValueToken) -> Result<UnionValue, String> {
        let value = self.value_string(value_token)?;
        self.apply_deep(|s| {
            for (_, u_type) in &s.union_types {
                if let Ok((variant_i, variant_type)) = u_type.variant(variant_str) {
                    return Some(if variant_type.accepts(value.get_type()) {
                        Ok(UnionValue { variant_index: variant_i, value: Box::new(value.clone()), u_type: u_type.clone() })
                    } else {
                        Err("wrong type in %... literal".to_owned())
                    })
                }
            };
            None
        }).unwrap_or(Err(format!("could not find variant %\"{variant_str}\"")))
    }

    pub fn variable_string(&self, variable_str: &str) -> Result<DataValue, String> {
        self.apply_deep(|s| {
            s.vars.get(variable_str).cloned()
        }).ok_or(format!("no variable called $\"{variable_str}\""))
    }

    pub fn struct_access_string(&self, struct_str: &str, field_list: &[ &str ]) -> Result<DataValue, String> {
        field_list.iter().try_fold(
            self.variable_string(struct_str)?,
            |acc, field| Ok(acc.assume_struct()?.get_field(field)?)
        )
    }

    pub fn add_var(&mut self, name: &str, val: DataValue) -> Result<(), String> {
        if name == "_" { return Ok(()); }
        let shadowed = self.apply_deep(|s| if s.vars.contains_key(name) { Some(s.context_str()) } else { None });
        if let Some(context) = shadowed { return Err(format!("variable \"{name}\" already exists in SCOPE{context}")); }
        self.vars.insert(name.to_owned(), val);
        Ok(())
    }

}
impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "  ENUMS")?;
        for (_, e) in &self.enum_types   { writeln!(f, "    {}", e.debug_type_details())?; }
        writeln!(f, "  UNIONS")?;
        for (_, u) in &self.union_types  { writeln!(f, "    {}", u.debug_type_details())?; }
        writeln!(f, "  STRUCTS")?;
        for (_, s) in &self.struct_types { writeln!(f, "    {}", s.debug_type_details())?; }
        writeln!(f, "  VARIABLES")?;
        let mut vars = self.vars.iter().collect::<Vec<_>>();
        vars.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (name, val) in vars { writeln!(f, "    {name} = {val:?}")?; }
        Ok(())
    }
}

pub struct State {
    base_scope: Rc<RefCell<Scope>>,
    fn_scopes: Vec<Rc<RefCell<Scope>>>,
    ret: Option<DataValue>,
    input: VecDeque<usize>,
    trace_file: File,
}
impl State {

    pub fn new() -> Self {
        Self {
            base_scope: Rc::new(RefCell::new(Scope::new(None))),
            fn_scopes: Vec::new(),
            ret: None,
            input: VecDeque::new(),
            trace_file: File::create("./trace.txt").expect("failed to create trace file"),
        }
    }

    pub fn push_scope<'a>(&mut self, containing_fn: Option<Rc<FunctionValue>>) {
        self.fn_scopes.push(Rc::new(RefCell::new(Scope::new(containing_fn))));
        write!(self.trace_file, "{:04}>{}", self.fn_scopes.len(), " ".repeat(self.fn_scopes.len())).expect("failed to write to trace");
    }

    pub fn pop_scope(&mut self) {
        self.fn_scopes.pop();
    }

    pub fn scope(&self) -> &Rc<RefCell<Scope>> {
        self.fn_scopes.last().unwrap_or(&self.base_scope)
    }

    pub fn set_ret(&mut self, value: DataValue) {
        self.ret = Some(value);
    }

    pub fn move_ret(&mut self, var_name: &str, fallthrough_type: &DataType) -> Result<(), String> {
        let ret = self.ret.clone().ok_or("no return for ~")?;
        if var_name == "~" {
            match fallthrough_type.accepts(ret.get_type()) {
                true => Ok(()),
                false => Err(format!("wrong type for return value, \"{}\" does not accept \"{:?}\"", fallthrough_type.debug_type_name(), ret)),
            }
        } else {
            self.ret = None;
            self.scope().borrow_mut().add_var(var_name, ret)
        }
    }

    pub fn new_input(&mut self, input: String) {
        self.input = input.lines().next().unwrap_or("").chars().map(|c| c as usize).collect();
    }

    pub fn next_input_char(&mut self) -> usize {
        self.input.pop_front().unwrap_or(0)
    }

    pub fn trace_file(&mut self) -> &mut File {
        &mut self.trace_file
    }

}
impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self.scope().borrow())?;
        writeln!(f, "RET: {:?}", self.ret)?;
        Ok(())
    }
}