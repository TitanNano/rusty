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
