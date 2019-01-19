use failure::*;

#[derive(Debug, Fail)]
pub enum ScopeError {
    #[fail(display = "variable {} is undefinded in current scope {}!", variable_name, scope_name)]
    UndefinedVariable {
        variable_name: String,
        scope_name: String,
    },
}

#[derive(Debug, Fail)]
pub enum AccessError {
    #[fail(display = "property {} is not defined on {}!", property, object)]
    UndefinedProperty {
        object: String,
        property: String,
    },
}

#[derive(Debug, Fail)]
pub enum TypeError {
    #[fail(display = "{} is not a valid prototype!", prototype)]
    IncompatiblePrototype {
        prototype: String,
    },

    #[fail(display = "{} is not a function!", type_name)]
    NotFunction {
        type_name: String,
    },

    #[fail(display = "Unable to modify property {} of trimitive type {}!", property, type_name)]
    PrimitivePropertyWrite {
        type_name: String,
        property: String,
    },
}
