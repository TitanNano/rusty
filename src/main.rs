extern crate ecmascript;
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
use ecmascript::ast as Ast;
use failure::*;

use dynamic_typing::{ Type, FunctionType, Scope, Variable, ObjectType };
use statics::{ OBJECT, OBJECT_PROTOTYPE, ARRAY_PROTOTYPE };
use error::{ AccessError };

fn main() {
    let global_object: Variable = Variable::new(String::from("Object"), (&*OBJECT).clone());

    let mut static_root_scope: Scope = Scope::new(String::from("StaticRoot"), None);

    static_root_scope.variables = vec!(global_object);

    // read foo.js
    let mut file = File::open("/Users/Jovan/rusty/test.js").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    // parse it
    let ast = match ecmascript::parse(&contents) {
        Ok(ast) => ast,
        Err(e) => { println!("{}", e); return; }
    };

    let (scope, errors) = analyze_ast(ast, &static_root_scope);

    println!("{}", serde_json::to_string_pretty(&scope).unwrap());

    for error in errors {
        println!("Error while analyzing scope <{}>: {:?}", scope.name(), error);
    }
}

fn analyze_ast<'a, 'b>(ast: Ast::Program, static_root_scope: &'a Scope<'b>) -> (Scope<'a>, Vec<Error>) {
    let body = match ast {
        Ast::Program::Program { body, .. } => body
    };

    let mut module_scope = Scope::new(String::from("ModuleScope"), Some(static_root_scope));
    let mut scope_errors = vec!();

    for statement in body {
        match statement {
            Ast::Statement::VariableDeclaration { declarations, kind, .. } => {
                for declaration in declarations {
                    let variable = analyze_declaration(declaration, &kind, &module_scope);

                    match variable {
                        Ok(variable) => {

                            match variable.current_type() {
                                Type::Object(data) => module_scope.add_type(data.clone()),
                                Type::Function(data) => module_scope.add_type(data.clone()),
                                _ => ()
                            };

                            module_scope.add(variable)
                        },
                        Err(e) => scope_errors.push(e),
                    }
                }
            }

            Ast::Statement::ExpressionStatement { expression, .. } => {
                println!("ExpressionStatement is currently not handled! {:?}", expression);
            }
        }
    }

    (module_scope, scope_errors)
}

fn analyze_declaration(declaration: Ast::VariableDeclarator, kind: &Ast::VariableDeclarationKind, scope: &Scope) -> Result<Variable, Error> {
    let variable_name = match declaration.id {
        Ast::Pattern::Identifier { name, .. } => name,
        Ast::Pattern::ObjectPattern { ref properties, .. } => {
            return analyze_object_destructure(properties, &declaration, kind);
        },
        Ast::Pattern::ArrayPattern { ref elements, .. } => {
            return analyze_array_destructure(elements, &declaration, kind);
        },
        Ast::Pattern::AssignmentPattern { left: ref pattern, argument: ref default } => {
            return analyze_assignment_pattern(pattern, default, &declaration, kind);
        },
        Ast::Pattern::RestElement { argument, .. } => {
            let pattern = *argument;

            match pattern {
                Ast::Pattern::Identifier { name, .. } => name,
                _ => unreachable!("a rest element can only contain a Pattern::Identifier!")
            }
        }
    };

    let mut variable_type = match declaration.init {
        Ast::VariableDeclaratorInit::Expression(value) => determine_expression_type(&value, scope)?,
        Ast::VariableDeclaratorInit::Null => Type::Undefined,
    };

    variable_type.assign_name(variable_name.clone());

    let variable = Variable::new(variable_name, variable_type);

    Ok(variable)
}

fn analyze_object_destructure(_properties: &[Ast::ObjectPatternProperty], _declaration: &Ast::VariableDeclarator, _kind: &Ast::VariableDeclarationKind) -> Result<Variable, Error> {
    panic!("Object Destructuring is not implemented!");
}

fn analyze_array_destructure(_elements: &[Ast::Pattern], _declaration: &Ast::VariableDeclarator, _kind: &Ast::VariableDeclarationKind) -> Result<Variable, Error> {
    panic!("Array Destructuring is not implemented!");
}

fn analyze_assignment_pattern(_pattern: &Ast::Pattern, _default: &Ast::Expression, _declaration: &Ast::VariableDeclarator, _kind: &Ast::VariableDeclarationKind) -> Result<Variable, Error> {
    panic!("Assignment Patterns are not implemented!");
}

fn type_from_properties(properties: &[Ast::ObjectExpressionProperty], scope: &Scope) -> Result<Type, Error> {
    let properties: Result<HashMap<String, Type>, Error> = properties.iter().map(|property| {
        let transformed = match property {
            Ast::ObjectExpressionProperty::Property(property) => {
                (expression_to_string(&property.key), determine_expression_type(&property.value, &scope)?)
            },

            Ast::ObjectExpressionProperty::SpreadElement(_) => panic!("Property spread for Object literals is not implement!"),
        };

        Ok(transformed)
    }).collect();

    let new_type = Type::Object(Box::new(ObjectType::new(None, properties?, Some((&*OBJECT_PROTOTYPE).clone()))));

    Ok(new_type)
}

fn determine_expression_type(expression: &Ast::Expression, scope: &Scope) -> Result<Type, Error> {
    let var_type: Type = match expression {
        Ast::Expression::Literal { value: literal, .. } => literal.into(),
        Ast::Expression::Identifier { name, .. } => {
            scope.locate(&name)?.current_type().clone()
        },
        Ast::Expression::ThisExpression { .. } => {
            Type::Undefined
        },
        Ast::Expression::ArrayExpression { elements, .. } => {
            let mixed: Result<Vec<Type>, Error> = elements.iter().map(|element| {
                match element {
                    Ast::ExpressionListItem::Null => Ok(Type::Undefined),
                    Ast::ExpressionListItem::Expression(expression) => determine_expression_type(&expression, &scope),
                    Ast::ExpressionListItem::Spread(element) => {
                        match element {
                            Ast::SpreadElement::SpreadElement { argument, .. } => determine_expression_type(&argument, scope)
                        }
                    }
                }
            }).collect();

            Type::Composed { outer: (*ARRAY_PROTOTYPE).clone(), inner: Box::new(Type::Mixed(mixed?))}
        },

        Ast::Expression::ObjectExpression { properties, .. } => {
            type_from_properties(properties, &scope)?
        },

        Ast::Expression::FunctionExpression { .. } => {
            panic!("function expressions are not implemented!");
        },

        Ast::Expression::UnaryExpression { argument, .. } => determine_expression_type(&argument, &scope)?,
        Ast::Expression::UpdateExpression { argument, .. } => determine_expression_type(&argument, &scope)?,
        Ast::Expression::BinaryExpression { .. } => Type::Boolean,
        Ast::Expression::AssignmentExpression { right, .. } => determine_expression_type(&right, &scope)?,
        Ast::Expression::LogicalExpression { .. } => Type::Boolean,

        Ast::Expression::MemberExpression { object, property, .. } => {
            match &**object {
                Ast::SuperExpression::Expression(expression) => {
                    let object = determine_expression_type(&expression, &scope)?;
                    let properties: &HashMap<String, Type> = object.properties();
                    let mut member_type = None;

                    for (name, type_) in properties {
                        if name != &expression_to_string(&**property) {
                            continue;
                        }

                        member_type = Some(type_.clone());
                    }

                    if let Some(type_) = member_type {
                        type_
                    } else {
                        Err(AccessError::UndefinedProperty { property: expression_to_string(&**property), object: object.to_string() })?
                    }
                },

                Ast::SuperExpression::Super { .. } => panic!("super expressions are not implemented!"),
            }
        },

        Ast::Expression::ConditionalExpression { alternate, consequent, .. } => {
            let left_type = determine_expression_type(&**alternate, &scope)?;
            let right_type = determine_expression_type(&**consequent, &scope)?;

            if left_type == right_type {
                left_type
            } else {
                Type::Mixed(vec!(left_type, right_type))
            }
        },

        //this is not right because we actually need the return type not the function type
        Ast::Expression::CallExpression { callee, .. } => {
            match &**callee {
                Ast::SuperExpression::Expression(expression) => determine_expression_type(&expression, scope)?,
                Ast::SuperExpression::Super(_) => panic!("super expressions are not implemented!"),
            }
        },

        Ast::Expression::NewExpression { callee, .. } => determine_expression_type(&callee, &scope)?,
        Ast::Expression::ArrowFunctionExpression { .. } => {
            Type::Function(Box::new(FunctionType::new(vec!())))
        },

        Ast::Expression::SequenceExpression { expressions: list } => {
            match list.last() {
                Some(expression) => determine_expression_type(&expression, &scope)?,
                None => unreachable!("it's not possible to have an empty expression list!"),
            }
        },

        Ast::Expression::Yield { argument, .. } => {
            match argument {
                Some(box_) => determine_expression_type(&**box_, &scope)?,
                None => Type::Undefined,
            }
        },

        Ast::Expression::TemplateLiteral { .. } => Type::String,

        //here we also need the return type instead of the function type
        Ast::Expression::TaggedTemplateExpression { tag, .. } => determine_expression_type(tag, &scope)?,
        Ast::Expression::AwaitExpression { argument, .. } => determine_expression_type(argument, &scope)?.unwrap(),
        Ast::Expression::MetaProperty => Type::Undefined,
        Ast::Expression::JSXElement { .. } => Type::Undefined,
        Ast::Expression::JsxFragment { .. } => Type::Undefined
    };

    Ok(var_type)
}

fn literal_to_string(literal: &Ast::Literal) -> String {
    match literal {
        Ast::Literal::StringLiteral(literal) => literal.0.to_string(),
        Ast::Literal::NullLiteral(_) => "null".to_string(),
        Ast::Literal::NumericLiteral(literal) => literal.0.to_string(),
        Ast::Literal::BooleanLiteral(literal) => literal.0.to_string(),
        Ast::Literal::RegExpLiteral(literal) => "/".to_string() + &literal.pattern + "/" + &literal.flags,
    }
}

fn expression_to_string(expression: &Ast::Expression) -> String {
    match expression {
        Ast::Expression::Literal { value, .. } => literal_to_string(value),
        Ast::Expression::Identifier { name, .. } => name.clone(),
        _ => "NotRepresentable".to_string(),
    }
}
