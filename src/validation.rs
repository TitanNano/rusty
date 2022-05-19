use ast_nodes::{ExpressionNode, ExpressionNodeStruct, Node};
use context::Context;
use dynamic_typing::{new_mutex_ref, CustomType, MutexRef, SafeBorrow, Scope, Scoped, Type};
use error::ValidationError;
use expression_meta_data::{MetaCarry, MetaHashMap};
use expressions::{determine_expression_type, expression_to_string};
use meta_data_collection::collect_meta_data;
use ratel::ast as Ast;
use std::clone::Clone;
use std::sync::Arc;
use traveler::{travel_ast, travel_ast_statement, AstEvent};

pub fn validation_pass<'ast>(ast: Ast::StatementList<'ast>, context: &mut Context<'ast>) {
    let event_record = travel_ast(ast);

    validate_events(event_record, context);
}

pub fn validate_events<'ast>(
    event_record: Vec<AstEvent<'ast, ExpressionNodeStruct<'ast>>>,
    context: &mut Context<'ast>,
) {
    for data in event_record.into_iter() {
        let meta_data = collect_meta_data(&data, context);

        match data {
            AstEvent::Identifier { identifier, .. } => {
                let variable = context.scope.locate(identifier);

                let error = match variable {
                    Ok(_) => None,
                    Err(error) => {
                        let error = ValidationError::from(error);

                        meta_data.borrow_mut_safe(|data| data.set_error(error.into()))
                    }
                };

                if let Some(error) = error {
                    context.errors.insert(error);
                }
            }

            AstEvent::Assignment { node: _node, left, right } => {
                let receiver_meta_data = context.node_meta_data(&left);
                let value_meta_data = context.node_meta_data(&right);

                let their_type = value_meta_data.borrow_safe(|data| data.expression_type());
                let own_type = receiver_meta_data.borrow_safe(|data| data.expression_type());

                meta_data.borrow_mut_safe(|data| {
                    data.adopt_errors(&value_meta_data);
                    data.adopt_errors(&receiver_meta_data);
                });

                if own_type == their_type || own_type == Type::Null {
                    continue;
                }

                let validation_error = ValidationError::AssignTypeMissmatch {
                    target: left.source(),
                    own_type: own_type.to_string(),
                    their_type: their_type.to_string(),
                    location: left.location().clone(),
                };

                let validation_error =
                    meta_data.borrow_mut_safe(|data| data.set_error(validation_error.into()));

                if let Some(validation_error) = validation_error {
                    context.errors.insert(validation_error);
                }
            }

            AstEvent::PropertyAccess {
                object, property, ..
            } => {
                let object_type = determine_expression_type(&object.expression, &context.scope)
                    .expect("variable has to exist at this location!");
                let property_result =
                    object_type.query_property(&property.source(), property.location());

                if let Some(_) = property_result {
                    continue;
                }

                let node_location = property.location().clone();

                let validation_error = ValidationError::UnknownProperty {
                    object: expression_to_string(&object.expression),
                    property: property.source(),
                    location: node_location,
                };

                let validation_error =
                    meta_data.borrow_mut_safe(|data| data.set_error(validation_error.into()));

                if let Some(validation_error) = validation_error {
                    context.errors.insert(validation_error);
                }
            }

            AstEvent::Addition { left, right, .. } => {
                let left_meta_data = context.node_meta_data(&left);
                let right_meta_data = context.node_meta_data(&right);

                let (left_type, right_type) = (
                    left_meta_data.borrow_safe(|data| data.expression_type()),
                    right_meta_data.borrow_safe(|data| data.expression_type()),
                );

                if left_type == Type::String || right_type == Type::String {
                    continue;
                }

                if let Type::Composed { outer, .. } = left_type {
                    let is_array = outer.borrow_safe(|object| object.is_array());

                    if is_array {
                        continue;
                    }
                }

                meta_data.borrow_mut_safe(|data| {
                    data.adopt_errors(&left_meta_data);
                    data.adopt_errors(&right_meta_data);
                });
            }

            AstEvent::Equality { node, left, right } => {
                let left_meta_data = context.node_meta_data(&left);
                let right_meta_data = context.node_meta_data(&right);

                let (left_type, right_type) = (
                    left_meta_data.borrow_safe(|data| data.expression_type()),
                    right_meta_data.borrow_safe(|data| data.expression_type()),
                );

                if left_type != right_type {
                    let validation_error = ValidationError::CompareTypeMissmatch {
                        left_type: left_type.to_string(),
                        right_type: right_type.to_string(),
                        location: node.location().to_owned(),
                    };

                    let validation_error =
                        meta_data.borrow_mut_safe(|data| data.set_error(validation_error.into()));

                    if let Some(validation_error) = validation_error {
                        context.errors.insert(validation_error);
                    }
                }

                meta_data.borrow_mut_safe(|data| {
                    data.adopt_errors(&left_meta_data);
                    data.adopt_errors(&right_meta_data);
                });
            }

            AstEvent::Conditional { .. } => {}
            AstEvent::AlternateBody { .. } => {}
            AstEvent::AfterIf { .. } => {}
            AstEvent::ConsequentBody { test, statement } => {
                let mut meta_data = MetaCarry::new();
                let mut block_scope = Scope::from(context.scope.clone());
                let mut error = None;

                let test_meta = context.node_meta_data(&test);

                context.clear_error(&test_meta);

                test_meta.borrow_safe(|data| {
                    if let Some(comparison_meta) = data.comparison() {
                        let (left_expression, right_expression) = comparison_meta.members();
                        let left_meta = context.node_meta_data(left_expression);
                        let right_meta = context.node_meta_data(right_expression);

                        if left_meta.borrow_safe(|data| data.variable().is_none())
                            && right_meta.borrow_safe(|data| data.variable().is_none())
                        {
                            let local_error = ValidationError::NonsensicalComparison {
                                expression: test.source(),
                                location: test.location().clone(),
                            };

                            error = meta_data.set_error(Arc::from(local_error));
                        }
                    }
                });

                if let Some(x_error) = error {
                    context.errors.insert(x_error);
                }

                block_scope.set_name("IfConsequentBlockScope".to_string());

                let local_event_record = travel_ast_statement(statement);

                validate_events(local_event_record, context)
            }

            AstEvent::Literal { node, literal } => {
                let mut meta_data = MetaCarry::new();
                let literal_type = Type::from(&literal);

                meta_data.set_expression_type(literal_type);
                context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            AstEvent::DynamicPropertyAccess { node, property, .. } => {
                let property_type = context
                    .node_meta_data(&property)
                    .borrow_safe(|data| data.expression_type());
                let mut meta_data = MetaCarry::new();

                if property_type != Type::String {
                    let error = ValidationError::InvalidType {
                        expression: property.source(),
                        current_type: property_type.to_string(),
                        expected_type: Type::String.to_string(),
                        location: node.location().to_owned(),
                    };

                    let error = meta_data.set_error(Arc::from(error));

                    if let Some(error) = error {
                        context.errors.insert(error);
                    }
                }

                meta_data.set_expression_type(Type::Undefined);
                context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            AstEvent::Array { node, .. } => {
                let mut meta_data = MetaCarry::new();
                let expression_type = determine_expression_type(node.expression(), &context.scope)
                    .expect("this is definetely an array!");

                meta_data.set_expression_type(expression_type);
                context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            AstEvent::This { .. } => {}
            AstEvent::Template { node, .. } => {
                let mut meta_data = MetaCarry::new();

                meta_data.set_expression_type(Type::String);
                context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            AstEvent::Sequence { node, sequence } => {
                let mut meta_data = MetaCarry::new();
                let last_item = sequence.into_iter().last().expect(
                    "sequences should have at least two items, or it wouldn't be a sequence",
                );
                let expression_type = determine_expression_type(&last_item, &context.scope)
                    .expect("there has to be a type!");

                meta_data.set_expression_type(expression_type);
                context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            AstEvent::FunctionCall {
                node,
                function,
                arguments,
            } => {
                let mut meta_data = MetaCarry::new();
                let function_definition = context
                    .scope
                    .locate(&expression_to_string(&function.expression))
                    .expect("function should exist");
                let arguments = map_argument_types(&arguments, &context.data_map);

                let return_type = function_definition.borrow_safe(|definition| {
                    let def_type = definition.current_type();

                    match def_type {
                        Type::Function(data) => {
                            data.borrow_safe(|data| data.return_type(&arguments))
                        }
                        _ => panic!("only functions can be called!"),
                    }
                });

                meta_data.set_expression_type(return_type);
                context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            AstEvent::Spread { node, argument } => {
                let argument_meta_data = context.node_meta_data(&argument);

                context.set_node_meta_data(&node, argument_meta_data);
            }

            AstEvent::PreOrPostFix { node, operand, .. } => {
                let meta_data = context.node_meta_data(&operand);

                context.set_node_meta_data(&node, meta_data);
            }

            AstEvent::Object { node, .. } => {
                let mut meta_data = MetaCarry::new();
                let expression_type = determine_expression_type(node.expression(), &context.scope)
                    .expect("it should be possible to determine an object type");

                meta_data.set_expression_type(expression_type);
                context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }
            AstEvent::Function { .. } => {}
            AstEvent::Class { .. } => {}
        }
    }
}

pub fn map_argument_types<'ast>(
    args: &Vec<ExpressionNodeStruct<'ast>>,
    data_map: &MutexRef<MetaHashMap<'ast>>,
) -> Vec<Type> {
    args.into_iter()
        .map(|expression_node| {
            let meta = data_map.borrow_safe(|map| map.get(&expression_node).unwrap().clone());

            meta.borrow_safe(|data| data.expression_type())
        })
        .collect()
}
