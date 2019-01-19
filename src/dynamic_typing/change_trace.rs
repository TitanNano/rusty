use super::types::Type;
use ratel::{ ast as Ast };

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct ChangeTrace<TC> {
    changes: Vec<TraceSet<TC>>,
}

impl<TC: PartialEq> ChangeTrace<TC> {
    pub fn change(&mut self, change: TC, new_type: Type, location: Location) {
        self.changes.push(TraceSet { attribute: change, loc: location, current_type: new_type });
    }

    fn query(&self, attribute: TC, location: Location) -> Option<TraceResult> {
        let set = self.changes.iter()
            .find(|set| set.attribute == attribute && set.loc.start < location.start);

        if let Some(TraceSet { ref loc, ref current_type, .. }) = set {
            return Some(TraceResult { loc, current_type });
        }

        None
    }

    pub fn new() -> Self {
        ChangeTrace { changes: vec!() }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
struct TraceSet<T> {
    attribute: T,
    loc: Location,
    current_type: Type
}

struct TraceResult<'s> {
    loc: &'s Location,
    current_type: &'s Type
}

#[derive(PartialEq, Debug, Serialize, Clone)]
pub struct TracedTypeChange;

#[derive(PartialEq, Debug, Serialize, Clone)]
pub enum TracedTypeMuation {
    Add(String),
    Remove(String),
    Update(String),
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct Location {
    start: u32,
    end: u32,
    line: u32,
    column: u32,
}

impl<T> From<Ast::Loc<T>> for Location {
    fn from(value: Ast::Loc<T>) -> Self {
        Location { start: value.start, end: value.end, line: 0, column: 0, }
    }
}
