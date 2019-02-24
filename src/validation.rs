use ratel::{ ast as Ast };
use traveler::{ travel_ast, HookType, PiggybackCapable };
use failure::*;
use dynamic_typing::{ Scope, Location };
use expressions::{ expression_to_string, determine_expression_type };
use error::{ ValidationError };

struct MetaCarry {
    error: Option<Error>,
}

impl MetaCarry {
    fn new() -> Self {
        Self { error: None }
    }
}

impl PiggybackCapable for MetaCarry {
    fn new() -> Self {
        Self::new()
    }
}

pub fn validation_pass(ast: Ast::StatementList, scope: &Scope) -> Vec<ValidationError> {
    let mut errors = vec!();

    travel_ast(ast, |data: &HookType<MetaCarry>| {
        match data {
            HookType::ConsequentBody { test, consequent } => {
            },

            HookType::PropertyAccess { object, property, .. } => {
                let object_type = determine_expression_type(&object.item, scope).expect("variable has to exist at this location!");

                let property_result = object_type.query_property(property.item, **property);

                if property_result.is_some() {
                    return;
                }

                let validation_error = ValidationError::UnknownProperty {
                    object: expression_to_string(&object.item),
                    property: property.item.to_owned(),
                    location: Location::from(***object),
                };

                errors.push(validation_error);
            },

            _ => (),
        };
    });

    errors
}
