use std::{borrow::Borrow, cell::RefCell, collections::HashMap, ops::BitAnd, rc::Rc};

use map_macro::hash_map;
use vtree::Component;

mod microdom {
    use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

    use downcast::{downcast, Any};
    use dyn_clone::{clone_trait_object, DynClone};

    #[derive(Debug)]
    pub enum ElementType {
        Div(HashMap<String, String>),
        Span(HashMap<String, String>),
        Button(u64),
    }

    impl ElementType {}

    impl ToString for ElementType {
        fn to_string(&self) -> String {
            match self {
                ElementType::Div(attributes) => string_tag_inner("div", attributes),
                ElementType::Span(attributes) => string_tag_inner("span", attributes),
                ElementType::Button(message) => format!("button onclick=\"{:?}\"", message),
            }
        }
    }

    fn string_tag_inner(tag: &str, attributes_map: &HashMap<String, String>) -> String {
        let attributes_string = attributes_map
            .iter()
            .map(|(name, value)| format!("{}=\"{}\"", name, value))
            .collect::<String>();

        format!("{}", tag)
    }

    #[derive(Debug)]
    pub enum NodeType<T> {
        Element {
            element_type: T,
            children: Vec<Rc<RefCell<Node>>>,
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
                        .map(|child: &Rc<RefCell<Node>>| { child.as_ref().borrow().to_string() })
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

    impl ToString for Node {
        fn to_string(&self) -> String {
            self.element_type.to_string()
        }
    }
}

mod vtree {
    // Node(Component) -> LiteralNode(Node) -> Output

    use std::any::Any;
    use std::fmt::Debug;

    pub trait StaticComponent: 'static + Debug {
        type Props: Debug + Clone + 'static;
        type LiteralNode;
        fn new() -> Self;
        fn render(
            &self,
            props: &Self::Props,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<Self::LiteralNode>;
    }

    pub trait Component: 'static + Debug {
        type Props: Debug + Clone + 'static;
        type Message;
        type LiteralNode;
        fn new() -> Self;
        fn render(
            &self,
            props: &Self::Props,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<Self::LiteralNode>;
        fn on_message(&mut self, message: &Self::Message) {}
        fn on_message_any(&mut self, message: Box<dyn Any>) {
            if let Some(message) = message.downcast_ref::<Self::Message>() {
                self.on_message(message);
            }
        }
    }

    impl<T> Component for T
    where
        T: StaticComponent,
    {
        type Props = T::Props;
        type Message = ();
        type LiteralNode = T::LiteralNode;

        fn new() -> Self {
            T::new()
        }

        fn render(
            &self,
            props: &Self::Props,
            children: &Vec<Option<Node<Self::LiteralNode>>>,
        ) -> Node<Self::LiteralNode> {
            T::render(&self, props, children)
        }
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
            log::trace!("expecting: {}", std::any::type_name::<T::Props>());
            log::trace!("got: {}", std::any::type_name_of_val(&*props));
            let new_props = props.downcast_ref::<T::Props>().unwrap();
            (self as &T).render(new_props, children)
        }
    }

    pub trait ComponentFactory: Debug {
        type LiteralNode;
        type Props;
        fn factory() -> Box<dyn AnyComponent<LiteralNode = Self::LiteralNode>>;
        fn clone_props(from: &Box<dyn Any>) -> Box<dyn Any>;
    }

    impl<T> ComponentFactory for T
    where
        T: Component,
        T::Props: 'static + Clone,
    {
        type LiteralNode = T::LiteralNode;
        type Props = T::Props;

        fn factory() -> Box<dyn AnyComponent<LiteralNode = Self::LiteralNode>> {
            Box::new(T::new())
        }

        fn clone_props(from: &Box<dyn Any>) -> Box<dyn Any> {
            Box::new(from.downcast_ref::<T::Props>().unwrap().clone())
        }
    }

    #[derive(Debug)]
    pub enum NodeType<T> {
        Component {
            component: fn() -> Box<dyn AnyComponent<LiteralNode = T>>,
            clone_props: fn(&Box<dyn Any>) -> Box<dyn Any>,
            props: Box<dyn Any>,
        },
        Raw(T),
    }

    impl<T> Clone for NodeType<T>
    where
        T: Clone,
    {
        fn clone(&self) -> Self {
            match self {
                NodeType::Component {
                    component,
                    clone_props,
                    props,
                } => NodeType::Component {
                    component: *component,
                    clone_props: *clone_props,
                    props: clone_props(props),
                },
                NodeType::Raw(literal) => NodeType::Raw(literal.clone()),
            }
        }
    }

    #[derive(Clone, Debug)]
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
                    clone_props: T::clone_props,
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

    /*pub trait Renderer<L> {
        fn new(initial: Node<L>) -> Self;
        fn mount(&mut self);
    }*/
}

mod vtree_microdom {
    use core::panic;
    use std::borrow::BorrowMut;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::hash::Hash;
    use std::rc::Rc;
    use std::task::Context;
    use std::vec;

    use downcast::{downcast, Any};
    use dyn_clone::clone_trait_object;
    use dyn_clone::DynClone;

    use crate::microdom;
    use crate::vtree;
    use crate::vtree::AnyComponent;

    pub trait Message: Debug + DynClone + Any {}
    clone_trait_object!(Message);
    downcast!(dyn Message);

    impl<T> Message for T where T: 'static + Debug + Clone {}

    #[derive(Clone, Debug)]
    pub enum LiteralNode {
        Div(HashMap<String, String>),
        Span(HashMap<String, String>),
        Button(Box<dyn Message>),
        Text(String),
    }

    #[derive(Debug)]
    struct InternalComponentNode<L> {
        component: Box<dyn AnyComponent<LiteralNode = L>>,
        vtree_node: vtree::Node<L>,
        rendered: Option<Rc<InternalNode<L>>>,
    }

    impl InternalComponentNode<LiteralNode> {
        fn new(vtree_node: vtree::Node<LiteralNode>) -> Self {
            if let vtree::NodeType::Component {
                component,
                clone_props: _,
                props: _,
            } = &vtree_node.node_type
            {
                Self {
                    component: component(),
                    vtree_node,
                    rendered: None,
                }
            } else {
                panic!("InternalComponentNode must be created with NodeType::Component");
            }
        }
        fn mount(&mut self, context: &mut RenderingContext) -> Rc<RefCell<microdom::Node>> {
            let rendered = if let vtree::NodeType::Component {
                component,
                props,
                clone_props: _,
            } = &self.vtree_node.node_type
            {
                self.component.render(props, &self.vtree_node.children)
            } else {
                panic!("InternalComponentNode must be created with NodeType::Component");
            };

            self.rendered = Some(Rc::new(instantiate_internal_component(rendered)));

            Rc::get_mut(&mut self.rendered.as_mut().unwrap())
                .unwrap()
                .mount(context)
        }
    }

    #[derive(Debug)]
    struct InternalLiteralNode {
        vtree_node: vtree::Node<LiteralNode>,
        bind_to: Option<Rc<RefCell<microdom::Node>>>,
        children: Vec<Option<Rc<InternalNode<LiteralNode>>>>, // children: InternalNode<L>
    }

    impl InternalLiteralNode {
        fn new(vtree_node: vtree::Node<LiteralNode>) -> Self {
            if let vtree::NodeType::Raw(_) = &vtree_node.node_type {
                Self {
                    vtree_node,
                    bind_to: None,
                    children: vec![],
                }
            } else {
                panic!("InternalLiteralNode must be created with NodeType::Raw");
            }
        }

        fn mount(&mut self, context: &mut RenderingContext) -> Rc<RefCell<microdom::Node>> {
            if let vtree::NodeType::Raw(literal_node) = &self.vtree_node.node_type {
                let children: Vec<Rc<RefCell<microdom::Node>>> = self
                    .vtree_node
                    .children
                    .iter_mut()
                    .filter_map(|child| {
                        if let Some(child) = child {
                            Some(instantiate_internal_component(child.clone()).mount(context))
                        } else {
                            None
                        }
                    })
                    .collect();

                self.bind_to = Some(Rc::new(RefCell::new(microdom::Node {
                    element_type: match literal_node {
                        LiteralNode::Div(attributes) => microdom::NodeType::Element {
                            element_type: microdom::ElementType::Div(attributes.clone()),
                            children: children,
                        },
                        LiteralNode::Span(attributes) => microdom::NodeType::Element {
                            element_type: microdom::ElementType::Span(attributes.clone()),
                            children: children,
                        },
                        LiteralNode::Button(message) => microdom::NodeType::Element {
                            element_type: microdom::ElementType::Button(
                                context.new_event_handler_id(),
                            ),
                            children: children,
                        },
                        LiteralNode::Text(text) => microdom::NodeType::Text(text.clone()),
                    },
                })));

                self.bind_to.as_ref().unwrap().clone()
            } else {
                panic!("InternalLiteralNode must be created with NodeType::Raw");
            }
        }
    }

    #[derive(Debug)]
    enum InternalNode<L> {
        Component(InternalComponentNode<L>),
        Literal(InternalLiteralNode),
    }

    impl InternalNode<LiteralNode> {
        fn mount(&mut self, context: &mut RenderingContext) -> Rc<RefCell<microdom::Node>> {
            log::trace!("mounting {:?}", self);
            match self {
                InternalNode::Component(internal_component_node) => {
                    internal_component_node.mount(context)
                }
                InternalNode::Literal(internal_literal_node) => {
                    internal_literal_node.mount(context)
                }
            }
        }
    }

    impl ToString for InternalNode<LiteralNode> {
        fn to_string(&self) -> String {
            todo!();
            /*(match self {
                InternalNode::Component(c) => {
                    c.rendered.as_ref().map_or("None".to_string(), |c| {
                        format!("<Component{component}>{children}</Component{component}>", component = c.to_string(), children = c.ch)
                    })
                }
                InternalNode::Literal(_) => {

                },
            }*/
        }
    }

    fn instantiate_internal_component(
        vtree_node: vtree::Node<LiteralNode>,
    ) -> InternalNode<LiteralNode> {
        match &vtree_node.node_type {
            vtree::NodeType::Component {
                component: _,
                props: _,
                clone_props: _,
            } => InternalNode::Component(InternalComponentNode::new(vtree_node)),
            vtree::NodeType::Raw(_literal) => {
                InternalNode::Literal(InternalLiteralNode::new(vtree_node))
            }
        }
    }

    #[derive(Debug)]
    pub struct RenderingContext {
        found_event_handlers: HashSet<EventHandlerMapping>,
    }

    impl RenderingContext {
        pub fn new() -> Self {
            Self {
                found_event_handlers: HashSet::new(),
            }
        }
        pub fn new_event_handler_id(&mut self) -> u64 {
            // TODO: Counter
            let id = rand::random();
            self.found_event_handlers.insert(EventHandlerMapping {
                id,
                message: Box::new(()),
            });

            id
        }
    }

    #[derive(Debug)]
    struct EventHandlerMapping {
        pub id: u64,
        pub message: Box<dyn Message>,
    }

    impl Eq for EventHandlerMapping {}

    impl PartialEq for EventHandlerMapping {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    impl Hash for EventHandlerMapping {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.id.hash(state);
        }
    }

    #[derive(Debug)]
    pub struct MicrodomRenderer {
        root: InternalNode<LiteralNode>,
        target: Rc<RefCell<microdom::Node>>,
        event_handlers: Rc<RefCell<HashMap<u64, EventHandlerMapping>>>,
    }

    impl MicrodomRenderer {
        pub fn new(initial: vtree::Node<LiteralNode>, target: Rc<RefCell<microdom::Node>>) -> Self {
            Self {
                root: instantiate_internal_component(initial),
                target,
                event_handlers: Rc::new(RefCell::new(HashMap::new())),
            }
        }

        pub fn mount(&mut self) {
            let mut context = RenderingContext::new();

            let result = self.root.mount(&mut context);

            if let microdom::NodeType::Element {
                element_type,
                children,
            } = &mut self.target.as_ref().borrow_mut().element_type
            {
                children.push(result);
            }
        }

        pub fn on_message(&mut self, message_id: u64) {
            let event_handlers = self.event_handlers.borrow();
            let Some(message_mapping) = event_handlers.get(&message_id) else {
                return;
            };

            let message = message_mapping.message.clone();
        }
    }
}

#[derive(Debug)]
struct TestComponent;

impl vtree::StaticComponent for TestComponent {
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
struct Counter {
    count: u32,
}

impl vtree::Component for Counter {
    type Props = ();
    type Message = ();
    type LiteralNode = vtree_microdom::LiteralNode;

    fn new() -> Self {
        Counter { count: 0 }
    }

    fn render(
        &self,
        props: &Self::Props,
        children: &Vec<Option<vtree::Node<Self::LiteralNode>>>,
    ) -> vtree::Node<Self::LiteralNode> {
        vtree::Node::new_literal(
            vtree_microdom::LiteralNode::Div(hash_map! {
                "class".to_string() => "counter".to_string()
            }),
            vec![
                Some(vtree::Node::new_literal(
                    vtree_microdom::LiteralNode::Text(format!("Count: {}", self.count)),
                    vec![],
                )),
                Some(vtree::Node::new_literal(
                    vtree_microdom::LiteralNode::Button(Box::new(())),
                    vec![],
                )),
            ],
        )
    }

    fn on_message(&mut self, message: &Self::Message) {
        self.count += 1;
    }
}

#[derive(Debug, Clone)]
struct BiTreeProps {
    depth: u32,
}

#[derive(Debug)]
struct BiTree;

impl vtree::StaticComponent for BiTree {
    type Props = BiTreeProps;
    type LiteralNode = vtree_microdom::LiteralNode;

    fn new() -> Self {
        BiTree
    }

    fn render(
        &self,
        props: &Self::Props,
        children: &Vec<Option<vtree::Node<Self::LiteralNode>>>,
    ) -> vtree::Node<Self::LiteralNode> {
        if props.depth == 0 {
            return vtree::Node::new_literal(
                vtree_microdom::LiteralNode::Div(hash_map! {}),
                vec![],
            );
        }

        vtree::Node::new_literal(
            vtree_microdom::LiteralNode::Div(hash_map! {}),
            vec![
                Some(vtree::Node::new::<BiTree>(
                    BiTreeProps {
                        depth: props.depth - 1,
                    },
                    vec![],
                )),
                Some(vtree::Node::new::<BiTree>(
                    BiTreeProps {
                        depth: props.depth - 1,
                    },
                    vec![],
                )),
            ],
        )
    }
}

#[derive(Debug)]
struct App {
    date: std::time::Instant,
    seconds: u64,
}

impl vtree::Component for App {
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
        vtree::Node::new_literal(
            vtree_microdom::LiteralNode::Div(HashMap::new()),
            vec![
                Some(vtree::Node::new::<Counter>((), vec![])),
                Some(vtree::Node::new::<BiTree>(BiTreeProps { depth: 6 }, vec![])),
            ],
        )
    }

    fn on_message(&mut self, message: &Self::Message) {
        self.seconds = self.date.elapsed().as_secs();
    }
}

fn main() {
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let time = std::time::Instant::now();

    let app = App::new();

    let container = Rc::new(RefCell::new(microdom::Node {
        element_type: microdom::NodeType::Element {
            element_type: microdom::ElementType::Div(hash_map! {
                "id".to_string() => "app".to_string()
            }),
            children: vec![],
        },
    }));

    let a = app.render(&(), &vec![]);
    let mut renderer = vtree_microdom::MicrodomRenderer::new(a, container.clone());

    renderer.mount();

    println!("mount: {:}ms", time.elapsed().as_micros() as f64 / 1000.0);

    println!("{}", container.as_ref().borrow().element_type.to_string());

    let mut rl = rustyline::DefaultEditor::new().unwrap();

    loop {
        let line = rl.readline(">> ");

        match line {
            Ok(line) => {
                let tokens = line.split_whitespace().collect::<Vec<&str>>();

                if tokens.len() != 2 {
                    println!("Invalid command syntax: <command> <args_without_white_space>");
                    continue;
                }

                let command = tokens[0];
                let arg = tokens[1];

                match command {
                    "message" => {
                        let message = arg.parse::<u64>();

                        if let Ok(message) = message {
                            renderer.on_message(message);
                            println!("message: {:?}", message);
                        } else {
                            println!("Invalid message: {:?}", arg);
                        }
                    }
                    _ => {
                        println!("Unknown command: {:?}", command);
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
    }
}
