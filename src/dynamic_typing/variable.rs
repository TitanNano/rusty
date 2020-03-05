use super::types::Type;
use super::{ TracedTypeChange, ChangeTrace, TracedChange, Location };
use ratel::{ ast as Ast };

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct Variable {
    name: String,
    current_type: Type,
    change_trace: ChangeTrace<TracedTypeChange>,
    kind: VariableKind,
}

impl Variable {
    pub fn current_type(&self) -> &Type {
        &self.current_type
    }

    pub fn name(&self)  -> &str {
        &self.name
    }

    pub fn new(name: String, current_type: Type, kind: VariableKind) -> Self {
        Self {
            name,
            current_type,
            kind,
            change_trace: ChangeTrace::new(),
        }
    }
}

impl TracedChange<TracedTypeChange, Type, Location> for Variable {
    fn change(&mut self, change: TracedTypeChange, new_type: Type, location: Location) {
        self.change_trace.change(change, new_type, location)
    }
}

#[derive(PartialEq, Debug, Clone, Serialize)]
pub enum VariableKind {
    Const,
    Let,
    Var,
}

impl From<Ast::DeclarationKind> for VariableKind {
    fn from(kind: Ast::DeclarationKind) -> VariableKind {
        match kind {
            Ast::DeclarationKind::Const => VariableKind::Const,
            Ast::DeclarationKind::Let => VariableKind::Let,
            Ast::DeclarationKind::Var => VariableKind::Var,
        }
    }
}

impl ToString for Variable {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}
