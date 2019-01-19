extern crate ratel;
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate erased_serde;
extern crate uuid;

mod dynamic_typing;
mod statics;
mod error;

use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use std::sync::{ Mutex };
use ratel::{ parse, ast as Ast, Module };
use failure::*;

use dynamic_typing::{
    Type, FunctionType, Scope, Variable, ObjectType, VariableKind, CustomType, CustomTypeObject,
    TracedTypeChange, Location, TracedChange, SafeBorrow, TracedTypeMuation, new_mutex_ref,
};
use statics::{ OBJECT, OBJECT_PROTOTYPE, ARRAY_PROTOTYPE };
use error::{ AccessError, TypeError };

fn main() {
    let global_object: Variable = Variable::new(String::from("Object"), (&*OBJECT).clone(), VariableKind::Const);

    let mut static_root_scope: Scope = Scope::new(String::from("StaticRoot"), None);

    static_root_scope.variables = vec!(Mutex::new(global_object));

    // read foo.js
    let mut file = File::open("/Users/Jovan/rusty/test.js").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    // parse it
    let module = match parse(&contents) {
        Ok(ast) => ast,
        Err(e) => { println!("{:#?}", e); return; }
    };

    let (scope, errors, tracing_errors) = analyze_ast(module, &static_root_scope);

    println!("{}", serde_json::to_string_pretty(&scope).unwrap());

    for error in errors {
        println!("Error while analyzing scope <{}>: {:?}", scope.name(), error);
    }

    for error in tracing_errors {
        println!("Error while tracing scope <{}> for type changes: {:?}", scope.name(), error);
    }
}

fn analyze_ast<'a, 'b>(module: Module, static_root_scope: &'a Scope<'b>) -> (Scope<'a>, Vec<Error>, Vec<Error>) {
    let body = module.body();

    let mut module_scope = Scope::new(String::from("ModuleScope"), Some(static_root_scope));
    let mut scope_errors = vec!();

    for statement in body {
        let statement = **statement;

        if let Ast::Statement::Declaration(declaration_statement) = statement.item {
            let Ast::statement::DeclarationStatement { declarators: declarations, kind } = declaration_statement;

            for declaration in declarations {
                let variable = analyze_declaration(declaration.item, kind, &module_scope);

                match variable {
                    Ok(variable) => {

                        match variable.current_type() {
                            Type::Object(data) => module_scope.add_type(CustomTypeObject::from(data)),
                            Type::Function(data) => module_scope.add_type(CustomTypeObject::from(data)),
                            _ => ()
                        };

                        module_scope.add(variable)
                    },
                    Err(e) => scope_errors.push(e),
                }
            }
        }
    }

    let (module_scope, tracing_errors) = tracing_pass(body, module_scope);

    (module_scope, scope_errors, tracing_errors)
}

fn analyze_declaration(declaration: Ast::Declarator, kind: Ast::DeclarationKind, scope: &Scope) -> Result<Variable, Error> {
    let variable_name = match declaration.id.item {
        Ast::Pattern::Identifier(name) => name.to_string(),
        Ast::Pattern::RestElement { argument } => argument.item.to_string(),

        Ast::Pattern::ObjectPattern { ref properties, .. } => {
            let properties: Vec<Ast::Property> = properties.into_iter().map(|prop| prop.item).collect();

            return analyze_object_destructure(&properties[..], &declaration, kind);
        },

        Ast::Pattern::ArrayPattern { ref elements, .. } => {
            let elements: Vec<Ast::Pattern> = elements.into_iter().map(|element| { element.item }).collect();

            return analyze_array_destructure(&elements[..], &declaration, kind);
        },

        Ast::Pattern::AssignmentPattern { left: ref pattern, right: ref default } => {
            return analyze_assignment_pattern(pattern, default, &declaration, kind);
        },

        Ast::Pattern::Void => unreachable!("void pattern should only appear inside of array patterns!"),
    };

    let mut variable_type = match declaration.init {
        Some(value) => determine_expression_type(&value, scope)?,
        None => Type::Undefined,
    };

    variable_type.assign_name(&variable_name[..]);

    let variable = Variable::new(variable_name, variable_type, VariableKind::from(kind));

    Ok(variable)
}

fn analyze_object_destructure(_properties: &[Ast::Property], _declaration: &Ast::Declarator, _kind: Ast::DeclarationKind) -> Result<Variable, Error> {
    panic!("Object Destructuring is not implemented!");
}

fn analyze_array_destructure(_elements: &[Ast::Pattern], _declaration: &Ast::Declarator, _kind: Ast::DeclarationKind) -> Result<Variable, Error> {
    panic!("Array Destructuring is not implemented!");
}

fn analyze_assignment_pattern(_pattern: &Ast::Pattern, _default: &Ast::Expression, _declaration: &Ast::Declarator, _kind: Ast::DeclarationKind) -> Result<Variable, Error> {
    panic!("Assignment Patterns are not implemented!");
}

fn type_from_properties(properties: &[Ast::Property], scope: &Scope) -> Result<Type, Error> {
    let properties: Result<HashMap<String, Type>, Error> = properties.iter().map(|property| {
        let transformed = match property {
            Ast::Property::Literal { key, value } => {
                (property_to_string(&key.item), determine_expression_type(&value, &scope)?)
            },

            Ast::Property::Shorthand(property) => {
                (property.to_string(), determine_expression_type(&Ast::Expression::Identifier(property), &scope)?)
            },

            Ast::Property::Method { key, value: _value } => {
                (property_to_string(&key.item), Type::from(FunctionType::new(vec!())))
            },

            Ast::Property::Spread { argument } => panic!("Property spread for Object literals is not implement!, {:#?}", argument),
        };

        Ok(transformed)
    }).collect();

    let mut properties = properties?;
    let prototype: Option<CustomTypeObject> = {
        let prototype = properties.get("__proto__");

        match prototype {
            Some(prototype) => {
                match prototype {
                    Type::Object(type_) => Some(CustomTypeObject::from(type_)),
                    Type::Function(type_) => Some(CustomTypeObject::from(type_)),
                    Type::Undefined => None,
                    _ => Some(CustomTypeObject::from(&*OBJECT_PROTOTYPE))
                }
            },
            None => Some(CustomTypeObject::from(&*OBJECT_PROTOTYPE)),
        }
    };

    properties.remove("__proto__");

    let new_type = Type::Object(new_mutex_ref(ObjectType::new(None, properties, prototype)));

    Ok(new_type)
}

fn determine_expression_type(expression: &Ast::Expression, scope: &Scope) -> Result<Type, Error> {
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

            type_from_properties(&properties, &scope)?
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

            let left_type = determine_expression_type(&alternate.item, &scope)?;
            let right_type = determine_expression_type(&consequent.item, &scope)?;

            if left_type == right_type {
                left_type
            } else {
                Type::Mixed(vec!(left_type, right_type))
            }
        },

        //this is not right because we actually need the return type not the function type
        Ast::Expression::Call(call_expression) => {
            let function_type = determine_expression_type(&call_expression.callee.item, scope)?;

            match function_type {
                Type::Function(function_type) => function_type.borrow_safe(|function_type| function_type.return_type()),
                _ => panic!("unable to call {}, it's not a function!", expression_to_string(&call_expression.callee.item))
            }
        },
        Ast::Expression::Arrow(_) => {
            Type::Function(new_mutex_ref(FunctionType::new(vec!())))
        },

        Ast::Expression::Sequence(list) => {
            match list.body.into_iter().last() {
                Some(expression) => determine_expression_type(&expression, &scope)?,
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
        Ast::Expression::TaggedTemplate(expression) => determine_expression_type(&expression.tag.item, &scope)?,

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
                    _ => Err(TypeError::IncompatiblePrototype { prototype: proto.to_string() })?
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

fn literal_to_string(literal: &Ast::Literal) -> String {
    match literal {
        Ast::Literal::String(literal) => literal.to_string(),
        Ast::Literal::Null => "null".to_string(),
        Ast::Literal::Number(literal) => literal.to_string(),
        Ast::Literal::Binary(literal) => literal.to_string(),
        Ast::Literal::RegEx(literal) => literal.to_string(),
        Ast::Literal::Undefined => "undefined".to_string(),
        Ast::Literal::True => "true".to_string(),
        Ast::Literal::False => "false".to_string(),
    }
}

fn expression_to_string(expression: &Ast::Expression) -> String {
    match expression {
        Ast::Expression::Literal(value) => literal_to_string(value),
        Ast::Expression::Identifier(name) => name.to_string(),
        _ => "NotRepresentable".to_string(),
    }
}

fn property_to_string(property_key: &Ast::PropertyKey) -> String {
    match property_key {
        Ast::PropertyKey::Literal (value) => value.to_string(),
        Ast::PropertyKey::Binary (value) => value.to_string(),
        Ast::PropertyKey::Computed (expression_node) => expression_to_string(&expression_node.item),
    }
}

fn determine_member_type(expression: &Ast::Expression, property: Ast::Node<'_, &str>, scope: &Scope) -> Result<Type, Error> {
    let object = determine_expression_type(expression, scope)?;
    let member_type = object.properties(|properties| {
        let mut member_type: Option<Type> = None;

        for (name, type_) in properties {
            if name != property.item {
                continue;
            }

            member_type = Some(type_.clone());
        }

        member_type
    });

    if let Some(type_) = member_type {
        Ok(type_)
    } else {
        Err(AccessError::UndefinedProperty { property: property.item.to_string(), object: object.to_string() })?
    }
}

fn tracing_pass<'a>(ast: Ast::StatementList, mut scope: Scope<'a>) -> (Scope<'a>, Vec<Error>) {
    let mut error_collection = vec!();

    for statement in ast {
        match statement.item {
            Ast::Statement::Expression(expression) => {
                trace_expression(expression, &mut scope, &mut error_collection);
            },

            _ => ()
        }
    }

    (scope, error_collection)
}

fn trace_expression(expression: Ast::ExpressionNode, scope: &mut Scope, error_collection: &mut Vec<Error>) {

    match expression.item {
        Ast::Expression::Binary(binary_expression) => {
            let operator = binary_expression.operator;

            match operator {
                Ast::OperatorKind::Assign => (),
                _ => return,
            };

            let receiver = binary_expression.left;
            let type_source = binary_expression.right;
            let assigned_type = {
                let result = determine_expression_type(&type_source, &scope);

                match result {
                    Ok(type_) => type_,
                    Err(error) => { error_collection.push(error); return; }
                }
            };

            if let Ast::Expression::Identifier(identifier) = receiver.item {
                let variable = match scope.locate(identifier) {
                    Ok(variable) => variable,
                    Err(error) => {
                        error_collection.push(Error::from(error));
                        return;
                    }
                };

                variable.borrow_mut_safe(|variable| {
                    variable.change(TracedTypeChange, assigned_type.clone(), Location::from(*expression));
                });
            }

            println!("receiver: {:#?}", receiver.item);

            if let Ast::Expression::Member(member_expression) = receiver.item {
                let object_type = match determine_expression_type(&member_expression.object.item, scope) {
                    Ok(object_type) => object_type,
                    Err(error) => { error_collection.push(error); return; },
                };

                let property = member_expression.property.item.to_string();

                match object_type {
                    Type::Object(mut object_data) => {
                        let has_property = object_data.borrow_safe(|object_data| object_data.properties.contains_key(&property));

                        let change = if has_property {
                            TracedTypeMuation::Update(property)
                        } else {
                            TracedTypeMuation::Add(property)
                        };

                        object_data.borrow_mut_safe(|object_data| {
                            object_data.change(change, assigned_type, Location::from(*expression));
                        });
                    },

                    Type::Function(mut object_data) => {
                        let has_property = object_data.borrow_safe(|object_data| object_data.properties.contains_key(&property));

                        let change = if has_property {
                            TracedTypeMuation::Update(property)
                        } else {
                            TracedTypeMuation::Add(property)
                        };

                        object_data.borrow_mut_safe(|object_data| {
                            object_data.change(change, assigned_type, Location::from(*expression));
                        });
                    }

                    _ => error_collection.push(Error::from(TypeError::PrimitivePropertyWrite { type_name: object_type.to_string(), property })),
                }
            }
        },

        Ast::Expression::Call(call_expression) => {
            let arguments_result: Result<Vec<Type>, Error> = call_expression.arguments.into_iter()
                .map(|expression| determine_expression_type(expression, scope)).collect();

            let arguments = match arguments_result {
                Ok(arguments) => arguments,
                Err(error) => {
                    error_collection.push(error);
                    return;
                }
            };

            let callee_type = match determine_expression_type(&call_expression.callee.item, scope) {
                Ok(callee_type) => callee_type,
                Err(error) => {
                    error_collection.push(error);
                    return;
                }
            };

            match callee_type {
                Type::Function(function_type) => {
                    function_type.borrow_mut_safe(|function_type| {
                        function_type.trace_invocation(arguments, Location::from(*expression));
                    });
                },

                _ => {
                    error_collection.push(Error::from(TypeError::NotFunction { type_name: expression_to_string(&expression.item) }));
                    return;
                }
            }
        },

        _ => (),
    }
}
