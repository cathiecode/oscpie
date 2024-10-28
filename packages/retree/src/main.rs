use std::rc::Rc;

use map_macro::hash_map;
use vtree::{MutableComponent, Renderer};

mod microdom {
    use std::{borrow::Borrow, collections::HashMap, rc::Rc};

    #[derive(Debug)]
    pub enum ElementType {
        Div(HashMap<String, String>),
        Span(HashMap<String, String>),
    }

    impl ElementType {}

    impl ToString for ElementType {
        fn to_string(&self) -> String {
            match self {
                ElementType::Div(attributes) => string_tag_inner("div", attributes),
                ElementType::Span(attributes) => string_tag_inner("span", attributes),
            }
        }
    }

    fn string_tag_inner(tag: &str, attributes_map: &HashMap<String, String>) -> String {
        let attributes_string = attributes_map
            .iter()
            .map(|(name, value)| format!("{}=\"{}\"", name, value))
            .collect::<String>();

        format!("{} {}", tag, attributes_string)
    }

    #[derive(Debug)]
    pub enum NodeType<T> {
        Element {
            element_type: T,
            children: Vec<Rc<Option<Node>>>,
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
                        .flat_map(|child: &Rc<Option<Node>>| -> &Option<Node> { child.borrow() })
                        .map(|element: &Node| element.to_string())
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
    use std::rc::Rc;

    pub trait Component: 'static + Debug {
        type Props;
        type LiteralNode;
        fn new() -> Self;
        fn render(
            &self,
            props: &Self::Props,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<Self::LiteralNode>;
    }

    pub trait MutableComponent: 'static + Debug {
        type Props;
        type Message;
        type LiteralNode;
        fn new() -> Self;
        fn render(
            &self,
            props: &Self::Props,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<Self::LiteralNode>;
        fn on_message(&mut self, message: Self::Message);
    }

    // Propsの型を外で気にしたくないので隠す
    pub trait AnyComponent: Debug {
        type LiteralNode;
        fn render(
            &self,
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
            props: &Box<dyn Any>,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<T::LiteralNode> {
            (self as &T).render(props.downcast_ref::<T::Props>().unwrap(), children)
        }
    }

    pub trait ComponentFactory: Debug {
        type LiteralNode;
        fn factory() -> Box<dyn AnyComponent<LiteralNode = Self::LiteralNode>>;
    }

    impl<T> ComponentFactory for T
    where
        T: Component,
        T::Props: 'static,
    {
        type LiteralNode = T::LiteralNode;
        fn factory() -> Box<dyn AnyComponent<LiteralNode = Self::LiteralNode>> {
            Box::new(T::new())
        }
    }

    #[derive(Debug)]
    pub enum NodeType<T> {
        Component {
            component: fn() -> Box<dyn AnyComponent<LiteralNode = T>>,
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
                    component: T::factory,
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
        fn new(initial: Rc<Node<L>>) -> Self;
        fn mount(&mut self);
    }
}

mod vtree_microdom {
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::microdom;
    use crate::vtree;
    use crate::vtree::AnyComponent;

    #[derive(Clone, Debug)]
    pub enum LiteralNode {
        Div(HashMap<String, String>),
        Span(HashMap<String, String>),
        Text(String),
    }

    #[derive(Debug)]
    struct InternalComponentNode<L> {
        component: Box<dyn AnyComponent<LiteralNode = L>>,
        vtree_node: Rc<vtree::Node<L>>,
        rendered: Option<Rc<InternalNode<L>>>,
    }

    impl InternalComponentNode<LiteralNode> {
        fn new(vtree_node: Rc<vtree::Node<LiteralNode>>) -> Self {
            if let vtree::NodeType::Component { component, props: _ } = &vtree_node.node_type {
                Self {
                    component: component(),
                    vtree_node,
                    rendered: None,
                }
            } else {
                panic!("InternalComponentNode must be created with NodeType::Component");
            }
        }
        fn mount(&mut self) -> Rc<InternalNode<LiteralNode>> {
            let rendered = if let vtree::NodeType::Component { component: _, props } = &self.vtree_node.node_type {
                self.component.render(props, &self.vtree_node.children)
            } else {
                panic!("InternalComponentNode must be created with NodeType::Component");
            };

            self.rendered = Some(Rc::new(instantiate_internal_component(Rc::new(rendered))));

            self.rendered.clone().unwrap()
        }
    }

    #[derive(Debug)]
    struct InternalLiteralNode {
        component: LiteralNode,
        bind_to: Rc<microdom::Node>,
        children: Vec<Option<Rc<InternalNode<LiteralNode>>>>, // children: InternalNode<L>
    }

    impl InternalLiteralNode {
        fn new(vtree_node: Rc<vtree::Node<LiteralNode>>) -> Self {
            
            if let vtree::NodeType::Raw(literal) = &vtree_node.node_type {
                Self {
                    component: literal.clone(),
                    bind_to: Rc::new(microdom::Node {
                        element_type: match literal {
                            LiteralNode::Div(attributes) => microdom::NodeType::Element {
                                element_type: microdom::ElementType::Div(attributes.clone()),
                                children: vec![],
                            },
                            LiteralNode::Span(attributes) => microdom::NodeType::Element {
                                element_type: microdom::ElementType::Span(attributes.clone()),
                                children: vec![],
                            },
                            LiteralNode::Text(text) => microdom::NodeType::Text(text.clone()),
                        },
                    }),
                    children: vec![],
                }
            } else {
                panic!("InternalLiteralNode must be created with NodeType::Raw");
            }
        }

        fn mount(&mut self) -> Rc<InternalNode<LiteralNode>> {
            let children: Vec<Option<Rc<InternalNode<LiteralNode>>>> = self
                .children
                .iter_mut()
                .map(|child| {
                    if let Some(child) = child {
                        Some(Rc::get_mut(child).unwrap().mount())
                    } else {
                        None
                    }
                })
                .collect();

            Rc::new(InternalNode::Literal(InternalLiteralNode {
                component: self.component.clone(),
                bind_to: self.bind_to.clone(),
                children,
            }))
        }
    }

    #[derive(Debug)]
    enum InternalNode<L> {
        Component(InternalComponentNode<L>),
        Literal(InternalLiteralNode),
    }

    impl InternalNode<LiteralNode> {
        fn mount(&mut self) -> Rc<InternalNode<LiteralNode>> {
            match self {
                InternalNode::Component(internal_component_node) => internal_component_node.mount(),
                InternalNode::Literal(internal_literal_node) => internal_literal_node.mount(),
            }
        }
    }

    fn instantiate_internal_component(vtree_node: Rc<vtree::Node<LiteralNode>>) -> InternalNode<LiteralNode> {
        match &vtree_node.node_type {
            vtree::NodeType::Component { component, props } => {
                InternalNode::Component(InternalComponentNode::new(vtree_node))
            }
            vtree::NodeType::Raw(literal) => InternalNode::Literal(InternalLiteralNode::new(vtree_node)),
        }
    }

    #[derive(Debug)]
    pub struct MicrodomRenderer {
        root: InternalNode<LiteralNode>,
    }

    impl MicrodomRenderer {
        pub fn render(
            &self
        ) {
            println!("{:?}", &self.root);
        }
    }

    impl vtree::Renderer<LiteralNode> for MicrodomRenderer {
        fn new(initial: Rc<vtree::Node<LiteralNode>>) -> Self {
            Self {
                root: instantiate_internal_component(initial),
            }
        }

        fn mount(&mut self) {
            self.root.mount();
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
        props: &Self::Props,
        children: &Vec<Option<vtree::Node<Self::LiteralNode>>>,
    ) -> vtree::Node<Self::LiteralNode> {
        vtree::Node::new_literal(
            vtree_microdom::LiteralNode::Div(hash_map! {
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
struct App {
    date: std::time::Instant,
    seconds: u64,
}

impl vtree::MutableComponent for App {
    type Props = ();
    type Message = ();
    type LiteralNode = vtree_microdom::LiteralNode;

    fn new() -> Self {
        Self {
            date: std::time::Instant::now(),
            seconds: 0,
        }
    }

    fn render(
        &self,
        props: &Self::Props,
        children: &Vec<Option<vtree::Node<Self::LiteralNode>>>,
    ) -> vtree::Node<Self::LiteralNode> {
        vtree::Node::new::<TestComponent>("Hello, world! This is my first application! Please make sure that this text is rendered in moderate time.".to_string(), vec![])
    }

    fn on_message(&mut self, message: Self::Message) {
        self.seconds = self.date.elapsed().as_secs();
    }
}

fn main() {
    let time = std::time::Instant::now();

    let app = App::new();

    let a = app.render(&(), &vec![]);
    let mut renderer = vtree_microdom::MicrodomRenderer::new(Rc::new(a));

    renderer.render();

    println!("render 1: {:}ms", time.elapsed().as_micros() as f64 / 1000.0);

    renderer.mount();

    println!("mount: {:}ms", time.elapsed().as_micros() as f64 / 1000.0);

    renderer.render();

    println!("render 2: {:}ms", time.elapsed().as_micros() as f64 / 1000.0);
}
