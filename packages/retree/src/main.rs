use vtree::{Component, Renderer};
use map_macro::hash_map;

mod microdom {
    use std::collections::HashMap;

    #[derive(Debug)]
    pub enum ElementType {
        Div(HashMap<String, String>),
        Span(HashMap<String, String>),
    }

    impl ElementType {

    }

    impl ToString for ElementType {
        fn to_string(&self) -> String {
            match self {
                ElementType::Div(attributes) => string_tag_inner("div", attributes),
                ElementType::Span(attributes) => string_tag_inner("span", attributes),
            }
        }
    }

    fn string_tag_inner(tag: &str, attributes_map: &HashMap<String, String>) -> String{
        let attributes_string = attributes_map.iter().map(|(name, value)| {
            format!("{}=\"{}\"", name, value)
        }).collect::<String>();

        format!("{} {}", tag, attributes_string)
    }

    #[derive(Debug)]
    pub enum NodeType<T> {
        Element {
            element_type: T,
            children: Vec<Option<Node>>,
        },
        Text(String),
    }

    impl<T> ToString for NodeType<T>
    where
        T: ToString,
    {
        fn to_string(&self) -> String {
            match self {
                NodeType::Element {
                    element_type,
                    children,
                } => format!(
                    "<{tag}>{children}</{tag}>",
                    tag = element_type.to_string(),
                    children = children
                        .iter()
                        .flat_map(|child| child)
                        .map(|element| element.to_string())
                        .collect::<String>()
                ),
                NodeType::Text(text) => text.clone(),
            }
        }
    }

    #[derive(Debug)]
    pub struct Node {
        pub element_type: NodeType<ElementType>,
    }

    impl ToString for &Node {
        fn to_string(&self) -> String {
            self.element_type.to_string()
        }
    }
}

mod vtree {
    // Node(Component) -> LiteralNode(Node) -> Output

    use std::any::Any;
    use std::fmt::Debug;

    pub struct Context;

    pub trait Component: 'static + Debug {
        type Props;
        type LiteralNode;
        fn new() -> Self;
        fn render(
            &self,
            ctx: &mut Context,
            props: &Self::Props,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<Self::LiteralNode>;
    }

    // Propsの型を外で気にしたくないので隠す
    pub trait AnyComponent: Debug {
        type LiteralNode;
        fn render(
            &self,
            ctx: &mut Context,
            props: &Box<dyn Any>,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<Self::LiteralNode>;
    }

    impl<T> AnyComponent for T
    where
        T: Component,
        T::Props: 'static,
    {
        type LiteralNode = T::LiteralNode;
        fn render(
            &self,
            ctx: &mut Context,
            props: &Box<dyn Any>,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<T::LiteralNode> {
            (self as &T).render(ctx, props.downcast_ref::<T::Props>().unwrap(), children)
        }
    }

    #[derive(Debug)]
    pub enum NodeType<T> {
        Component {
            component: Box<dyn AnyComponent<LiteralNode = T>>,
            props: Box<dyn Any>,
        },
        Raw(T),
    }

    #[derive(Debug)]
    pub struct Node<T> {
        pub node_type: NodeType<T>,
        // NOTE: 取り扱いはpropsと同じ
        // FIXME: childrenを取りたくない、あるいはchildrenの型を限定したい場合はどうする?
        pub children: Vec<Option<Node<T>>>,
    }

    impl<V> Node<V> {
        pub fn new<T>(props: T::Props, children: Vec<Option<Node<V>>>) -> Node<V>
        where
            T: Component<LiteralNode = V>,
        {
            Node {
                node_type: NodeType::Component {
                    component: Box::new(T::new()),
                    props: Box::new(props),
                },
                children,
            }
        }

        pub fn new_literal(literal: V, children: Vec<Option<Node<V>>>) -> Node<V> {
            Node {
                node_type: NodeType::Raw(literal),
                children,
            }
        }
    }

    pub trait Renderer<L> {
        fn new(initial: &Node<L>) -> Self;
        fn diff(prev: &Node<L>, next: &Node<L>);
    }
}

mod vtree_microdom {
    use std::collections::HashMap;

    use crate::microdom;
    use crate::vtree;

    #[derive(Debug)]
    pub enum LiteralNode {
        Div(HashMap<String, String>),
        Span(HashMap<String, String>),
        Text(String),
    }

    pub struct MicrodomRenderer;

    impl MicrodomRenderer {
        pub fn render_oneshot(ctx: &mut vtree::Context, node: &vtree::Node<LiteralNode>) -> Option<microdom::Node> {
                match &node.node_type {
                vtree::NodeType::Component { component, props } => {
                    let node = component.render(ctx, props, &node.children);
                    Self::render_oneshot(ctx, &node)
                }
                vtree::NodeType::Raw(literal) => {
                    let children: Vec<Option<microdom::Node>> = node
                        .children
                        .iter()
                        .filter_map(|child| child.as_ref())
                        .map(|child| Self::render_oneshot(ctx, child))
                        .collect();
                    match literal {
                        LiteralNode::Div(attributes) => Some(microdom::Node {
                            element_type: microdom::NodeType::Element {
                                element_type: microdom::ElementType::Div(attributes.clone()),
                                children: children,
                            },
                        }),
                        LiteralNode::Span(attributes) => Some(microdom::Node {
                            element_type: microdom::NodeType::Element {
                                element_type: microdom::ElementType::Span(attributes.clone()),
                                children,
                            },
                        }),
                        LiteralNode::Text(text) => Some(microdom::Node {
                            element_type: microdom::NodeType::Text(text.clone()),
                        }),
                    }
                }
            }
        }    
    }

    impl vtree::Renderer<LiteralNode> for MicrodomRenderer {
        fn new(initial: &vtree::Node<LiteralNode>) -> Self {
            let mut ctx = vtree::Context {};
            println!("{}", (&Self::render_oneshot(&mut ctx, initial).unwrap()).to_string());

            Self {

            }
        }
    
        fn diff(prev: &vtree::Node<LiteralNode>, next: &vtree::Node<LiteralNode>) {
            todo!()
        }
    }
}

#[derive(Debug)]
struct TestComponent;

impl vtree::Component for TestComponent {
    type Props = String;
    type LiteralNode = vtree_microdom::LiteralNode;

    fn new() -> Self {
        TestComponent
    }

    fn render(
        &self,
        ctx: &mut vtree::Context,
        props: &Self::Props,
        children: &Vec<Option<vtree::Node<Self::LiteralNode>>>,
    ) -> vtree::Node<Self::LiteralNode> {
        vtree::Node::new_literal(
            vtree_microdom::LiteralNode::Div(hash_map!{
                "class".to_string() => "no-block".to_string()
            }),
            vec![
                Some(vtree::Node::new_literal(
                    vtree_microdom::LiteralNode::Text(props[0..1].to_string()),
                    vec![],
                )),
                if props.len() > 1 {
                    Some(vtree::Node::new::<TestComponent>(
                        props[1..].to_string(),
                        vec![],
                    ))
                } else {
                    None
                },
            ],
        )
    }
}

#[derive(Debug)]
struct App;

impl vtree::Component for App {
    type Props = ();
    type LiteralNode = vtree_microdom::LiteralNode;

    fn new() -> Self {
        todo!()
    }

    fn render(
        &self,
        ctx: &mut vtree::Context,
        props: &Self::Props,
        children: &Vec<Option<vtree::Node<Self::LiteralNode>>>,
    ) -> vtree::Node<Self::LiteralNode> {
        vtree::Node::new::<TestComponent>("Hello, world! This is my first application! Please make sure that this text is rendered in moderate time.".to_string(), vec![])
    }
}

fn main() {
    let time = std::time::Instant::now();

    let app = App;
    let mut ctx = vtree::Context {};

    let a = app.render(&mut ctx, &(), &vec![]);
    vtree_microdom::MicrodomRenderer::new(&a);

    println!("build: {:}ms", time.elapsed().as_micros() as f64 / 1000.0);

    //println!("real vtree: {}", ((&b.unwrap()).to_string()));

    println!("string: {:}ms", time.elapsed().as_micros() as f64 / 1000.0);
}
