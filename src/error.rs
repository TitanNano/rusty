use failure::*;
use dynamic_typing::{ Location };
use std::sync::Arc;
use std::collections::hash_set::HashSet;

pub type ErrorVec = HashSet<Arc<ValidationError>>;

#[derive(Debug, Fail)]
pub enum ScopeError {
    #[fail(display = "variable \"{}\" is undefinded in current scope \"{}\"!", variable_name, scope_name)]
    UndefinedVariable {
        variable_name: String,
        scope_name: String,
    },
}

#[derive(Debug, Fail)]
pub enum AccessError {
    #[fail(display = "property \"{}\" is not defined on \"{}\"!", property, object)]
    UndefinedProperty {
        object: String,
        property: String,
    },
}

#[derive(Debug, Fail)]
pub enum TypeError {
    #[fail(display = "\"{}\" is not a valid prototype!", prototype)]
    IncompatiblePrototype {
        prototype: String,
    },

    #[fail(display = "\"{}\" is not a function!", type_name)]
    NotFunction {
        type_name: String,
    },

    #[fail(display = "Unable to modify property \"{}\" of trimitive type \"{}\"!", property, type_name)]
    PrimitivePropertyWrite {
        type_name: String,
        property: String,
    },
}

#[derive(Debug, Fail, Eq, PartialEq, Hash)]
pub enum ValidationError {

    #[fail(display = "\"{}\" has no property \"{}\"", object, property)]
    UnknownProperty {
        object: String,
        property: String,
        location: Location,
    },

    #[fail(display = "\"{}\" has type \"{}\" but type \"{}\" was assigned", target, own_type, their_type)]
    AssignTypeMissmatch {
        target: String,
        location: Location,
        own_type: String,
        their_type: String,
    },

    #[fail(display = "trying to compare \"{}\" and \"{}\"", left_type, right_type)]
    CompareTypeMissmatch {
        left_type: String,
        right_type: String,
        location: Location,
    },

    #[fail(display = "variable \"{}\" is undefinded in current scope \"{}\"!", variable_name, scope_name)]
    UndefinedVariable {
        variable_name: String,
        scope_name: String,
    },

    #[fail(display = "\"{}\" must be of type \"{}\" but is \"{}\" here", expression, current_type, expected_type)]
    InvalidType {
        expression: String,
        current_type: String,
        expected_type: String,
        location: Location,
    },

    #[fail(display = "\"{}\" is a useless comparison and should be removed", expression)]
    NonsensicalComparison {
        expression: String,
        location: Location,
    },
}

impl ValidationError {
    pub fn location(&self) -> &Location {
        match self {
            ValidationError::UnknownProperty { location, .. } => &location,
            ValidationError::UndefinedVariable { .. } => &Location { column: 0, end: 0, line: 0, start: 0, },
            ValidationError::AssignTypeMissmatch { location, .. } => &location,
            ValidationError::CompareTypeMissmatch { location, .. } => &location,
            ValidationError::InvalidType { location, .. } => &location,
            ValidationError::NonsensicalComparison { location, .. } => &location,
        }
    }
}

impl From<ScopeError> for ValidationError {
    fn from(error: ScopeError) -> Self {
        match error {
            ScopeError::UndefinedVariable { variable_name, scope_name } => {
                ValidationError::UndefinedVariable { variable_name, scope_name }
            }
        }
    }
}
