use ratel::ast as Ast;
use dynamic_typing::{
    Type, SafeBorrow, CustomTypeObject, FunctionType, ObjectType,
    new_mutex_ref, CustomType, Scoped, ScopeRef
};
use failure::*;
use statics::ARRAY_PROTOTYPE;
use std::collections::HashMap;
use error::TypeError;
use literals::literal_to_string;
use objects::{ type_from_properties, determine_member_type };

pub fn determine_expression_type(expression: &Ast::Expression, scope: &ScopeRef) -> Result<Type, Error> {
    let var_type: Type = match expression {
        Ast::Expression::Void => Type::Undefined,

        Ast::Expression::Literal(literal) => literal.into(),
        Ast::Expression::Identifier(name) => {
            let variable = scope.locate(&name)?;

            variable.borrow_safe(|variable| variable.current_type().clone())
        },
        Ast::Expression::This { .. } => {
            Type::Undefined
        },
        Ast::Expression::Array(array_expression) => {
            let mixed: Result<Vec<Type>, Error> = array_expression.body.iter()
                .map(|element| determine_expression_type(&element.item, scope))
                .collect();

            Type::Composed { outer: (*ARRAY_PROTOTYPE).clone(), inner: Box::new(Type::Mixed(mixed?))}
        },

        Ast::Expression::Object(object_expression) => {
            let properties: Vec<Ast::Property> = object_expression.body.into_iter().map(|property| property.item).collect();

            type_from_properties(&properties, scope)?
        },

        Ast::Expression::Function { .. } => {
            panic!("function expressions are not implemented!");
        },

        // no properly implemented yet, this has to determine the type of the operation
        Ast::Expression::Binary(_binary_expression) => Type::Undefined,
        Ast::Expression::Member(member_expression) => {
            let Ast::expression::MemberExpression { object, property } = member_expression;

            determine_member_type(&object.item, *property, scope)?

        },

        Ast::Expression::Conditional(conditional_expression) => {
            let Ast::expression::ConditionalExpression { consequent, alternate, .. } = conditional_expression;

            let left_type = determine_expression_type(&alternate.item, scope)?;
            let right_type = determine_expression_type(&consequent.item, scope)?;

            if left_type == right_type {
                left_type
            } else {
                Type::Mixed(vec!(left_type, right_type))
            }
        },

        //this is not right because we actually need the return type not the function type
        Ast::Expression::Call(call_expression) => {
            let function_type = determine_expression_type(&call_expression.callee.item, scope)?;
            let argument_types: Vec<Type> = call_expression.arguments.iter().map(|expression_node| {
                determine_expression_type(&**expression_node, scope)
            }).collect::<Result<_, Error>>()?;

            match function_type {
                Type::Function(function_type) => function_type.borrow_safe(|function_type| function_type.return_type(&argument_types)),
                _ => panic!("unable to call {}, it's not a function!", expression_to_string(&call_expression.callee.item))
            }
        },
        Ast::Expression::Arrow(_) => {
            Type::Function(new_mutex_ref(FunctionType::new(vec!())))
        },

        Ast::Expression::Sequence(list) => {
            match list.body.into_iter().last() {
                Some(expression) => determine_expression_type(&expression, scope)?,
                None => unreachable!("it's not possible to have an empty expression list!"),
            }
        },

        Ast::Expression::Template(_) => Type::String,
        Ast::Expression::ComputedMember(expression) => {
            let Ast::expression::ComputedMemberExpression { object, property } = expression;
            let expression_string =  expression_to_string(&property.item);
            let location = Ast::Loc::new(property.start, property.end, &expression_string[..]);
            let property_node = Ast::Node::new(&location);

            determine_member_type(&object.item, property_node, scope)?
        }

        //here we also need the return type instead of the function type
        Ast::Expression::TaggedTemplate(expression) => determine_expression_type(&expression.tag.item, scope)?,

        // it's not quite clear what this is
        Ast::Expression::MetaProperty(_expression) => Type::Undefined,

        Ast::Expression::Prefix(expression) => {
            return determine_expression_type(&expression.operand, scope);
        }

        Ast::Expression::Postfix(expression) => {
            return determine_expression_type(&expression.operand, scope);
        }

        Ast::Expression::Spread(expression) => {
            let container_type = determine_expression_type(&expression.argument, scope)?;

            match container_type {
                Type::Composed { outer, inner } => {
                    if !outer.borrow_safe(|outer| outer.is_array()) {
                        return Ok(Type::Composed { outer, inner });
                    }

                    (*inner).clone()
                },

                _ => container_type
            }
        },

        Ast::Expression::Class(expression) => {
            let mut constructor = FunctionType::new(vec!());
            let mut prototype_properties = HashMap::new();

            let Ast::OptionalName(name) = expression.name;

            let name = match name {
                Some(name) => Some(name.to_string()),
                None => None,
            };


            let parent_prototype = match expression.extends {
                Some(expression) => Some(determine_expression_type(&expression.item, scope)?),
                None => None,
            };

            let parent_prototype: Option<CustomTypeObject> = match parent_prototype {
                Some(proto) => match proto {
                    Type::Object(ref object_type) => Some(CustomTypeObject::from(object_type)),
                    Type::Function(ref func_type) => Some(CustomTypeObject::from(func_type)),
                    _ => return Err(TypeError::IncompatiblePrototype { prototype: proto.to_string() }.into())
                },

                None => None,
            };

            if let Some(ref name) = name {
                constructor.assign_name(name.to_string());
            }

            let mut constructor_type = Type::from(constructor);

            prototype_properties.insert(String::from("constructor"), constructor_type.clone());

            let prototype_name = match name {
                Some(name) => Some(format!("{}Prototype", name)),
                None => None
            };

            let prototype = new_mutex_ref(ObjectType::new(prototype_name, prototype_properties, parent_prototype));

            constructor_type.properties_mut(|properties| {
                properties.insert("prototype".to_owned(), Type::from(&prototype));
            });

            constructor_type
        }
    };

    Ok(var_type)
}

pub fn expression_to_string(expression: &Ast::Expression) -> String {
    match expression {
        Ast::Expression::Literal(value) => literal_to_string(value),
        Ast::Expression::Identifier(name) => (*name).to_string(),
        Ast::Expression::Member(member_access) =>
            format!("{}.{}", expression_to_string(&member_access.object), **member_access.property),
        Ast::Expression::ComputedMember(..) => "NotRepresentable(ComputedMember)".to_string(),
        Ast::Expression::Object(..) => "NotRepresentable(Object)".to_string(),
        Ast::Expression::Function(..) => "NotRepresentable(Function)".to_string(),
        Ast::Expression::Binary(..) => "NotRepresentable(Binary)".to_string(),
        Ast::Expression::This(..) => "this".to_string(),
        Ast::Expression::Array(..) => "Arrary".to_string(),
        Ast::Expression::Void => "void".to_string(),
        Ast::Expression::Sequence(..) => "NotRepresentable(Sequence)".to_string(),
        Ast::Expression::Conditional(..) => "NotRepresentable(Conditional)".to_string(),
        Ast::Expression::Call(..) => "NotRepresentable(Call)".to_string(),
        Ast::Expression::Prefix(..) => "NotRepresentable(Prefix)".to_string(),
        Ast::Expression::Postfix(..) => "NotRepresentable(Postfix)".to_string(),
        Ast::Expression::MetaProperty(..) => "NotRepresentable(MetaProperty)".to_string(),
        Ast::Expression::Template(..) => "NotRepresentable(Template)".to_string(),
        Ast::Expression::TaggedTemplate(..) => "NotRepresentable(TaggedTemplate)".to_string(),
        Ast::Expression::Spread(..) => "NotRepresentable(Spread)".to_string(),
        Ast::Expression::Arrow(..) => "NotRepresentable(Arrow)".to_string(),
        Ast::Expression::Class(..) => "NotRepresentable(Class)".to_string(),
    }
}
