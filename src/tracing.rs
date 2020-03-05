use ratel::{ ast as Ast };
use dynamic_typing::{ TracedTypeChange, Location, TracedTypeMuation, Scope, SafeBorrow, Type, TracedChange };
use failure::*;
use expressions::{ determine_expression_type, expression_to_string };
use error::{ TypeError };

pub fn tracing_pass<'a>(ast: Ast::StatementList, mut scope: Scope<'a>) -> (Scope<'a>, Vec<Error>) {
    let mut error_collection = vec!();

    for statement in ast {
        if let Ast::Statement::Expression(expression) = statement.item {
            trace_expression(expression, &mut scope, &mut error_collection);
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

            if let Ast::Expression::Member(member_expression) = receiver.item {
                let object_type = match determine_expression_type(&member_expression.object.item, scope) {
                    Ok(object_type) => object_type,
                    Err(error) => { error_collection.push(error); return; },
                };

                let property = member_expression.property.item.to_string();

                match object_type {
                    Type::Object(object_data) => {
                        let has_property = object_data.borrow_safe(|object_data| object_data.properties.contains_key(&property));

                        let change = if has_property {
                            TracedTypeMuation::Update(property)
                        } else {
                            TracedTypeMuation::Add(property)
                        };

                        object_data.borrow_mut_safe(|object_data| {
                            object_data.change(change, assigned_type, Location::from(*expression).collapse(true));
                        });
                    },

                    Type::Function(object_data) => {
                        let has_property = object_data.borrow_safe(|object_data| object_data.properties.contains_key(&property));

                        let change = if has_property {
                            TracedTypeMuation::Update(property)
                        } else {
                            TracedTypeMuation::Add(property)
                        };

                        object_data.borrow_mut_safe(|object_data| {
                            object_data.change(change, assigned_type, Location::from(*expression).collapse(true));
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
                }
            }
        },

        _ => (),
    }
}
