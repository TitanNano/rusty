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

    pub fn find(&self, callback: impl Fn(&TraceSet<TC>) -> bool) -> Option<&TraceSet<TC>> {
        for change in self.changes.iter().rev() {
            if !callback(&change) {
                continue;
            }

            return Some(&change);
        }

        None
    }

    pub fn new() -> Self {
        ChangeTrace { changes: vec!() }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct TraceSet<T> {
    pub attribute: T,
    pub loc: Location,
    pub current_type: Type
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
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub column: u32,
}

impl Location {
    pub fn collapse(mut self, after: bool) -> Self {
        if after {
            self.end += 1;
            self.start = self.end;

            return self;
        }

        self.end = self.start;

        self
    }
}

impl<T> From<Ast::Loc<T>> for Location {
    fn from(value: Ast::Loc<T>) -> Self {
        Location { start: value.start, end: value.end, line: 0, column: 0, }
    }
}
