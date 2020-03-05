use ratel::{ ast as Ast };
use ast_nodes::{ ExpressionNode, ExpressionNodeStruct, NewExpressionNodeFromAst, StringNodeStruct };
use dynamic_typing::{ Location };

#[derive(Clone, Debug)]
pub enum AstEvent<'ast, En: ExpressionNode<'ast>> {
    Assignment {
        node: En,
        left: En,
        right: En,
    },

    Addition {
        node: En,
        left: En,
        right: En,
    },

    Equality {
        node: En,
        left: En,
        right: En,
    },

    Conditional {
        node: En,
        test: En,
        consequent: En,
        alternate: En,
    },

    PropertyAccess {
        node: En,
        object: En,
        property: StringNodeStruct,
    },

    DynamicPropertyAccess {
        node: En,
        object: En,
        property: En,
    },

    ConsequentBody {
        test: En,
        statement: Ast::Statement<'ast>,
    },

    AlternateBody {
        expression: En,
    },

    AfterIf {
        expression: En,
    },

    Identifier {
        node: En,
        identifier: Ast::Identifier<'ast>

    },

    Literal {
        node: En,
        literal: Ast::Literal<'ast>,
    },

    Array {
        node: En,
        expression: Ast::expression::ArrayExpression<'ast>,
    },

    Sequence {
        node: En,
        sequence: Ast::ExpressionList<'ast>,
    },

    This {
        node: En,
        this: Ast::expression::ThisExpression
    },

    PreOrPostFix {
        node: En,
        operand: En,
        operator: Ast::OperatorKind
    },

    Template {
        node: En,
    },

    FunctionCall {
        node: En,
        function: En,
        arguments: Vec<En>,
    },

    Spread {
        node: En,
        argument: En,
    },

    Object {
        node: En,
        expression: Ast::expression::ObjectExpression<'ast>,
    },

    Function {
        node: En,
        params: toolshed::list::List<'ast, ratel::ast::Node<'ast, ratel::ast::Pattern<'ast>>>,
        body: AstFunctionBody<'ast>,
    },

    Class {
        node: En,
        class_expression: Ast::Class<'ast, Ast::OptionalName<'ast>>,
    }
}

#[derive(Clone, Debug)]
pub enum AstFunctionBody<'ast> {
    StatementBlock(Ast::Block<'ast, Ast::Statement<'ast>>),
    SingleExpression(Ast::Expression<'ast>)
}

impl<'ast> From<Ast::expression::ArrowBody<'ast>> for AstFunctionBody<'ast> {
    fn from(value: Ast::expression::ArrowBody) -> AstFunctionBody {
        match value {
            Ast::expression::ArrowBody::Block(block) => AstFunctionBody::StatementBlock(**block),
            Ast::expression::ArrowBody::Expression(expression) => AstFunctionBody::SingleExpression(**expression),
        }
    }
}

pub trait PiggybackCapable {
    fn new() -> Self;
}

type EventRecord<'ast> = Vec<AstEvent<'ast, ExpressionNodeStruct<'ast>>>;

pub fn travel_ast<'ast>(ast: Ast::StatementList<'ast>) -> Vec<AstEvent<'ast, ExpressionNodeStruct<'ast>>> {
    let mut event_record = vec!();

    for statement in ast {
        let local_event_record = travel_ast_statement(statement.item);

        event_record.extend(local_event_record);
    };

    event_record
}

pub fn travel_ast_statement<'ast>(statement: Ast::Statement<'ast>) -> Vec<AstEvent<'ast, ExpressionNodeStruct<'ast>>> {
    let mut event_record = vec!();

    match statement {
        Ast::Statement::Expression(expression) => {
            let (_, local_event_record) = travel_expression(expression);

            event_record.extend(local_event_record);
        },

        Ast::Statement::If(if_statement) => {
            let (test, local_event_record) = travel_expression(if_statement.test);

            event_record.extend(local_event_record);
            event_record.push(AstEvent::ConsequentBody { test: test.clone(), statement: **if_statement.consequent });

            if if_statement.alternate.is_some() {
                event_record.push(AstEvent::AlternateBody { expression: test.clone() });
            }

            event_record.push(AstEvent::AfterIf { expression: test.clone() });
        }

        _ => {},
    };

    event_record
}

pub fn travel_expression<'ast>(expression: Ast::ExpressionNode<'ast>) -> (ExpressionNodeStruct<'ast>, Vec<AstEvent<'ast, ExpressionNodeStruct<'ast>>>) {
    let mut event_record: EventRecord = vec!();

    let expression = match expression.item {
        Ast::Expression::Binary(binary_expression) => {
            let node = ExpressionNodeStruct::from(expression);
            let operator = binary_expression.operator;
            let (left, left_event_record) = travel_expression(binary_expression.left);
            let (right, right_event_record) = travel_expression(binary_expression.right);

            event_record.extend(left_event_record);
            event_record.extend(right_event_record);

            match operator {
                Ast::OperatorKind::Assign => {
                    event_record.push(AstEvent::Assignment { node: node.clone(), left, right });
                },

                Ast::OperatorKind::Addition => {
                    event_record.push(AstEvent::Addition { node: node.clone(), left, right });
                },

                Ast::OperatorKind::StrictEquality => {
                    event_record.push(AstEvent::Equality { node: node.clone(), left, right });
                }

                Ast::OperatorKind::StrictInequality => {
                    event_record.push(AstEvent::Equality { node: node.clone(), left, right });
                }

                _ => (),
            };

            node
        },

        Ast::Expression::Conditional(conditional_expression) => {
            let node = ExpressionNodeStruct::from(expression);
            let (test, test_event_record) = travel_expression(conditional_expression.test);
            let (consequent, cons_event_record) = travel_expression(conditional_expression.consequent);
            let (alternate, alt_event_record) = travel_expression(conditional_expression.consequent);

            event_record.extend(test_event_record);
            event_record.extend(cons_event_record);
            event_record.extend(alt_event_record);

            event_record.push(AstEvent::Conditional { node: node.clone(), test, consequent, alternate });

            node
        },

        Ast::Expression::Member(member_expression) => {
            let node = ExpressionNodeStruct::from(expression);
            let (object, new_event_record) = travel_expression(member_expression.object);
            let property = StringNodeStruct::from(member_expression.property);

            event_record.extend(new_event_record);

            event_record.push(AstEvent::PropertyAccess { node: node.clone(), object, property });

            node
        }

        Ast::Expression::Identifier(identifier_expression) => {
            let node = ExpressionNodeStruct::from(expression);

            event_record.push(AstEvent::Identifier { node: node.clone(), identifier: identifier_expression });

            node
        },

        Ast::Expression::Void => {
            ExpressionNodeStruct::from(expression)
        },

        Ast::Expression::Literal(literal_expression) => {
            let node = ExpressionNodeStruct::from(expression);

            event_record.push(AstEvent::Literal { node: node.clone(), literal: literal_expression });

            node
        },

        Ast::Expression::Array(array_expression) => {
            let node = ExpressionNodeStruct::from(expression);

            event_record.push(AstEvent::Array { node: node.clone(), expression: array_expression });

            node
        },

        Ast::Expression::Sequence(sequence_expression) => {
            let node = ExpressionNodeStruct::from(expression);

            event_record.push(AstEvent::Sequence { node: node.clone(), sequence: sequence_expression.body });

            node
        },

        Ast::Expression::This(this_expression) => {
            let node = ExpressionNodeStruct::from(expression);

            event_record.push(AstEvent::This { node: node.clone(), this: this_expression });

            node
        },

        Ast::Expression::MetaProperty(meta_property) => {
            panic!("got meta property access {}.{}", **meta_property.meta, **meta_property.property);
        },

        Ast::Expression::ComputedMember(computed_member) => {
            let node = ExpressionNodeStruct::from(expression);
            let (object, object_event_record) = travel_expression(computed_member.object);
            let (property, property_event_record) = travel_expression(computed_member.property);

            event_record.extend(object_event_record);
            event_record.extend(property_event_record);

            event_record.push(AstEvent::DynamicPropertyAccess { node: node.clone(), object, property });

            node
        },

        Ast::Expression::Call(function_call) => {
            let node = ExpressionNodeStruct::from(expression);
            let (function, local_event_record) = travel_expression(function_call.callee);
            let mut arguments = vec!();

            event_record.extend(local_event_record);

            for argument in function_call.arguments {
                let (expression, local_event_record) = travel_expression(*argument);

                event_record.extend(local_event_record);

                arguments.push(expression);
            }

            event_record.push(AstEvent::FunctionCall { node: node.clone(), function, arguments });

            node
        },

        Ast::Expression::Prefix(prefix) => {
            let node = ExpressionNodeStruct::from(expression);
            let (operand, local_event_record) = travel_expression(prefix.operand);
            let operator = prefix.operator;

            event_record.extend(local_event_record);

            event_record.push(AstEvent::PreOrPostFix { node: node.clone(), operand, operator });

            node
        },

        Ast::Expression::Postfix(postfix) => {
            let node = ExpressionNodeStruct::from(expression);
            let (operand, local_event_record) = travel_expression(postfix.operand);
            let operator = postfix.operator;

            event_record.extend(local_event_record);

            event_record.push(AstEvent::PreOrPostFix { node: node.clone(), operand, operator });

            node
        }

        Ast::Expression::Template(_template) => {
            let node = ExpressionNodeStruct::from(expression);

            // we are currently ignoring all expressions inside the template literal
            event_record.push(AstEvent::Template { node: node.clone() });

            node
        },

        Ast::Expression::TaggedTemplate(template) => {
            let (_function, local_event_record) = travel_expression(template.tag);
            let template_expression = Ast::Expression::Template(**template.quasi);
            let loc = Location::from(*template.quasi);
            let node = ExpressionNodeStruct::new(template_expression, loc);

            event_record.extend(local_event_record);

        //    let arguments = vec!(travel_expression(Ast::ExpressionNode::new(loc_ref), arena, callback));

        //    event_record.push(AstEvent::FunctionCall { node, function });

            node
        },

        Ast::Expression::Spread(spread_expression) => {
            let node = ExpressionNodeStruct::from(expression);
            let (argument, local_event_record) = travel_expression(spread_expression.argument);

            event_record.extend(local_event_record);
            event_record.push(AstEvent::Spread { node: node.clone(), argument });

            node
        },

        Ast::Expression::Object(object_expression) => {
            let node = ExpressionNodeStruct::from(expression);

            event_record.push(AstEvent::Object { node: node.clone(), expression: object_expression });

            node
        },

        Ast::Expression::Arrow(arrow_function) => {
            let node = ExpressionNodeStruct::from(expression);
            let Ast::expression::ArrowExpression { params, body, .. } = arrow_function;
            let function_body = AstFunctionBody::from(body);

            event_record.push(AstEvent::Function { node: node.clone(), params, body: function_body });

            node
        },

        Ast::Expression::Function(function) => {
            let node = ExpressionNodeStruct::from(expression);
            let Ast::Function { params, body, .. } = function;
            let statement_list = AstFunctionBody::StatementBlock(**body);


            event_record.push(AstEvent::Function { node: node.clone(), params, body: statement_list });

            node
        },

        Ast::Expression::Class(class_expression) => {
            let node = ExpressionNodeStruct::from(expression);

            event_record.push(AstEvent::Class { class_expression, node: node.clone() });

            node
        }

    };

    (expression, event_record)
}
