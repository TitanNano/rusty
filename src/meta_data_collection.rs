use ast_nodes::{ExpressionNode, ExpressionNodeStruct, Node};
use context::Context;
use dynamic_typing::{
    new_mutex_ref, BindableScope, CustomType, FunctionType, Location, MutexRef, ObjectType,
    SafeBorrow, Scope, Scoped, TracedChange, TracedTypeChange, TracedTypeMuation, Type,
};
use error::ValidationError;
use expression_meta_data::{ComparisonMeta, ComparisonType, MetaCarry};
use expressions::{determine_expression_type, expression_to_string};
use ratel::ast as Ast;
use std::clone::Clone;
use std::sync::Arc;
use traitcast::cast_ref;
use traveler::AstEvent;
use validation::map_argument_types;

pub fn collect_meta_data<'ast>(
    event: &AstEvent<'ast, ExpressionNodeStruct<'ast>>,
    context: &mut Context<'ast>,
) -> MutexRef<MetaCarry<'ast>> {
    match event {
        AstEvent::Identifier { node, identifier } => {
            let variable = context.scope.locate(identifier);
            let mut meta_data = MetaCarry::new();

            let node_variable = match variable {
                Ok(variable) => Some(variable.clone()),
                Err(_) => None,
            };

            meta_data.set_variable(node_variable);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::Assignment { node, left, right } => {
            let mut meta_data = MetaCarry::new();
            let receiver_meta_data = context.node_meta_data(&left);
            let value_meta_data = context.node_meta_data(&right);

            let their_type = value_meta_data.borrow_safe(|data| data.expression_type());
            let _own_type = receiver_meta_data.borrow_safe(|data| data.expression_type());

            meta_data.set_expression_type(their_type.clone());

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::PropertyAccess {
            object,
            property,
            node,
        } => {
            let mut meta_data = MetaCarry::new();
            let object_type = determine_expression_type(&object.expression, &context.scope)
                .expect("variable has to exist at this location!");
            let property_result =
                object_type.query_property(&property.source(), property.location());

            if let Some(property_type) = property_result {
                meta_data.set_expression_type(property_type);

                return context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            meta_data.set_expression_type(Type::Undefined);
            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::Addition { node, left, right } => {
            let mut meta_data = MetaCarry::new();
            let left_meta_data = context.node_meta_data(&left);
            let right_meta_data = context.node_meta_data(&right);

            let (left_type, right_type) = {
                let left_type = left_meta_data.borrow_safe(|data| data.expression_type());
                let right_type = right_meta_data.borrow_safe(|data| data.expression_type());

                (left_type, right_type)
            };

            if left_type == Type::String || right_type == Type::String {
                meta_data.set_expression_type(Type::String);

                return context.set_node_meta_data(&node, new_mutex_ref(meta_data));
            }

            if let Type::Composed { outer, .. } = left_type {
                let is_array = outer.borrow_safe(|object| object.is_array());

                if is_array {
                    meta_data.set_expression_type(Type::String);

                    return context.set_node_meta_data(&node, new_mutex_ref(meta_data));
                }
            }

            meta_data.set_expression_type(Type::Number);
            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::Equality { node, left, right } => {
            let mut meta_data = MetaCarry::new();

            meta_data.set_comparison(ComparisonMeta {
                kind: ComparisonType::Equality,
                members: (left.clone(), right.clone()),
            });

            meta_data.set_expression_type(Type::Boolean);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::Conditional {
            node,
            consequent,
            alternate,
            ..
        } => {
            let mut meta_data = MetaCarry::new();
            let consequent_type = context
                .node_meta_data(&consequent)
                .borrow_safe(|data| data.expression_type());
            let alternate_type = context
                .node_meta_data(&alternate)
                .borrow_safe(|data| data.expression_type());

            let cond_type = if alternate_type != consequent_type {
                Type::Mixed(vec![alternate_type, consequent_type])
            } else {
                alternate_type
            };

            meta_data.set_expression_type(cond_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::AlternateBody { expression } => context.node_meta_data(&expression),
        AstEvent::AfterIf { expression } => context.node_meta_data(&expression),

        AstEvent::ConsequentBody {
            test,
            statement: _statement,
        } => {
            let block_scope = Scope::from(context.scope.clone());
            let block_scope_ref = new_mutex_ref(block_scope);

            let test_meta = context.node_meta_data(&test);

            if let Some(comparison_meta) = &test_meta.borrow_safe(|data| data.comparison()) {
                let (left_expression, right_expression) = comparison_meta.members();

                let left_meta = context.node_meta_data(left_expression);
                let right_meta = context.node_meta_data(right_expression);

                let left_type = left_meta.borrow_safe(|data| data.expression_type());
                let right_type = right_meta.borrow_safe(|data| data.expression_type());

                let inner_errors = {
                    let inner_context = context.derive(&block_scope_ref);

                    track_type_assignment(&left_meta, &right_type, test.location(), &inner_context);
                    track_type_assignment(&right_meta, &left_type, test.location(), &inner_context);

                    inner_context.errors
                };

                context.errors.extend(inner_errors);
            }

            context.node_meta_data(&test)
        }

        AstEvent::Literal { node, literal } => {
            let mut meta_data = MetaCarry::new();
            let literal_type = Type::from(literal);

            meta_data.set_expression_type(literal_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
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

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::Array { node, .. } => {
            let mut meta_data = MetaCarry::new();
            let expression_type = determine_expression_type(node.expression(), &context.scope)
                .expect("this is definetely an array!");

            meta_data.set_expression_type(expression_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::This { node, this } => {
            let mut meta_data = MetaCarry::new();
            let expression_type =
                determine_expression_type(&Ast::Expression::This(this.clone()), &context.scope)
                    .expect("this musst have a type!");

            meta_data.set_expression_type(expression_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }
        AstEvent::Template { node, .. } => {
            let mut meta_data = MetaCarry::new();

            meta_data.set_expression_type(Type::String);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::Sequence { node, sequence } => {
            let mut meta_data = MetaCarry::new();
            let last_item = sequence
                .into_iter()
                .last()
                .expect("sequences should have at least two items, or it wouldn't be a sequence");
            let expression_type = determine_expression_type(&last_item, &context.scope)
                .expect("there has to be a type!");

            meta_data.set_expression_type(expression_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
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
            let arguments = map_argument_types(arguments, &context.data_map);

            let return_type = function_definition.borrow_safe(|definition| {
                let def_type = definition.current_type();

                match def_type {
                    Type::Function(data) => data.borrow_safe(|data| data.return_type(&arguments)),
                    _ => panic!("only functions can be called!"),
                }
            });

            meta_data.set_expression_type(return_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }

        AstEvent::Spread { node, argument } => {
            let argument_meta_data = context.node_meta_data(&argument);

            context.set_node_meta_data(&node, argument_meta_data)
        }

        AstEvent::PreOrPostFix { node, operand, .. } => {
            let meta_data = context.node_meta_data(&operand);

            context.set_node_meta_data(&node, meta_data)
        }

        AstEvent::Object { node, .. } => {
            let mut meta_data = MetaCarry::new();
            let expression_type = determine_expression_type(node.expression(), &context.scope)
                .expect("it should be possible to determine an object type");

            meta_data.set_expression_type(expression_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }
        AstEvent::Function {
            node,
            params: _params,
            body: _body,
        } => {
            let mut meta_data = MetaCarry::new();
            let fun_type = determine_expression_type(node.expression(), &context.scope)
                .expect("expect function type");

            meta_data.set_expression_type(fun_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }
        AstEvent::Class {
            node,
            class_expression: _class_expression,
        } => {
            let mut meta_data = MetaCarry::new();
            let class_type = determine_expression_type(node.expression(), &context.scope)
                .expect("class type is expected");

            meta_data.set_expression_type(class_type);

            context.set_node_meta_data(&node, new_mutex_ref(meta_data))
        }
    }
}

fn track_type_assignment<'ast>(
    node_meta: &MutexRef<MetaCarry<'ast>>,
    target_type: &Type,
    location: &Location,
    context: &Context<'ast>,
) {
    node_meta.borrow_safe(|data| {
        if let Some(prop_access) = &data.property_access() {
            let object_meta = context.node_meta_data(prop_access.object());
            let object_type = object_meta.borrow_safe(|data| data.expression_type());
            let property_name = prop_access.property().source();

            match object_type {
                Type::Object(inner_type) => {
                    let custom_type =
                        cast_ref::<MutexRef<ObjectType>, MutexRef<dyn CustomType>>(&inner_type)
                            .expect("we should be able to cast from ObjectType to CustomType");

                    let custom_type = context
                        .scope
                        .borrow_mut_safe(|scope| scope.bind(custom_type));

                    custom_type.borrow_mut_safe(|object_type| {
                        let unboxed_object_type = &mut **object_type;

                        update_object_property_type(
                            unboxed_object_type,
                            &property_name,
                            target_type,
                            location,
                        );
                    })
                }

                Type::Function(inner_type) => {
                    let custom_type =
                        cast_ref::<MutexRef<FunctionType>, MutexRef<dyn CustomType>>(&inner_type)
                            .expect("we should be able to cast from ObjectType to CustomType");

                    let custom_type = context
                        .scope
                        .borrow_mut_safe(|scope| scope.bind(custom_type));

                    custom_type.borrow_mut_safe(|object_type| {
                        let unboxed_object_type = &mut **object_type;

                        update_object_property_type(
                            unboxed_object_type,
                            &property_name,
                            target_type,
                            location,
                        );
                    })
                }

                _ => unreachable!("it has to be a object variant"),
            };
        }
    });

    node_meta.borrow_safe(|data| {
        data.variable()
            .as_ref()
            .unwrap()
            .borrow_mut_safe(|variable| {
                variable.change(TracedTypeChange, target_type.clone(), location.clone())
            });
    });
}

pub fn update_object_property_type<T: TracedChange<TracedTypeMuation, Type, Location> + ?Sized>(
    object: &mut T,
    property: &str,
    new_type: &Type,
    location: &Location,
) {
    object.change(
        TracedTypeMuation::Update(property.to_owned()),
        new_type.clone(),
        location.clone(),
    )
}
