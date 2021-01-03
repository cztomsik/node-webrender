// observable model
// x holds the data/truth (tree of nodes)
// x allows changes
// x notifies listener

use std::collections::HashMap;
use crate::util::{IdTree};

pub type NodeId = u32;

#[derive(Debug)]
pub enum DocumentEvent {
    ParentChanged(NodeId),
    NodeDestroyed(NodeId),

    TextNodeCreated(NodeId),
    TextChanged(NodeId),

    ElementCreated(NodeId),
    AttributesChanged(NodeId),
    NodeInserted(NodeId, NodeId, usize),
    NodeRemoved(NodeId, NodeId),
}

pub struct Document {
    tree: IdTree<NodeData>,
    root: NodeId,

    listener: Box<dyn Fn(DocumentEvent)>
}

// private shorthand
type Event = DocumentEvent;

impl Document {
    pub fn new(listener: impl Fn(DocumentEvent) + 'static) -> Self {
        let listener = Box::new(listener);
        let mut tree = IdTree::new();

        let root = tree.create_node(NodeData::Element(ElementData {
            local_name: ":root".to_owned(),
            attributes: HashMap::new(),
         }));

        listener(Event::ElementCreated(root));

        Self { tree, root, listener }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    // shared for all node types

    pub fn is_element(&self, node: NodeId) -> bool {
        matches!(self.tree.data(node), NodeData::Element(_))
    }

    pub fn is_text(&self, node: NodeId) -> bool {
        matches!(self.tree.data(node), NodeData::Text(_))
    }

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.tree.parent(node)
    }

    pub fn children(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.tree.children(node)
    }

    pub fn insert_child(&mut self, parent: NodeId, child: NodeId, index: usize) {
        self.tree.insert_child(parent, child, index);

        self.emit(Event::NodeInserted(parent, child, index));
        self.emit(Event::ParentChanged(child));
    }

    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) {
        self.tree.remove_child(parent, child);

        self.emit(Event::NodeRemoved(parent, child));
        self.emit(Event::ParentChanged(child));
    }

    pub fn free_node(&mut self, node: NodeId) {
        self.tree.free_node(node);

        self.emit(Event::NodeDestroyed(node));
    }

    // text node

    pub fn create_text_node(&mut self, text: &str) -> NodeId {
        let id = self.tree.create_node(NodeData::Text(text.to_owned()));

        self.emit(Event::TextNodeCreated(id));

        id
    }

    pub fn text(&self, text_node: NodeId) -> &str {
        self.tree.data(text_node).text()
    }

    pub fn set_text(&mut self, text_node: NodeId, text: &str) {
        *self.tree.data_mut(text_node) = NodeData::Text(text.to_owned());

        self.emit(Event::TextChanged(text_node));
    }

    // element

    pub fn create_element(&mut self, local_name: &str) -> NodeId {
        let id = self.tree.create_node(NodeData::Element(ElementData {
            local_name: local_name.to_owned(),
            attributes: HashMap::new(),
        }));

        self.emit(Event::ElementCreated(id));

        id
    }

    pub fn local_name(&self, element: NodeId) -> &str {
        &self.tree.data(element).el().local_name
    }

    pub fn attribute(&self, element: NodeId, att_name: &str) -> Option<&str> {
        self.tree.data(element).el().attributes.get(att_name).map(String::as_ref)
    }

    pub fn set_attribute(&mut self, element: NodeId, att_name: &str, value: &str) {
        self.tree.data_mut(element).el_mut().attributes.insert(att_name.to_owned(), value.to_owned());

        self.emit(Event::AttributesChanged(element));
    }

    pub fn remove_attribute(&mut self, element: NodeId, att_name: &str) {
        self.tree.data_mut(element).el_mut().attributes.remove(att_name);

        self.emit(Event::AttributesChanged(element));
    }

    // helpers

    fn emit(&self, event: Event) {
        (self.listener)(event);
    }
}


// private from here

enum NodeData {
    Element(ElementData),
    Text(String),
}

struct ElementData {
    local_name: String,
    attributes: HashMap<String, String>,
}

// TODO: macro?
impl NodeData {
    fn el(&self) -> &ElementData {
        if let NodeData::Element(data) = &self {
            data
        } else {
            panic!("not an element")
        }
    }

    fn el_mut(&mut self) -> &mut ElementData {
        if let NodeData::Element(data) = self {
            data
        } else {
            panic!("not an element")
        }
    }

    fn text(&self) -> &str {
        if let NodeData::Text(data) = &self {
            data
        } else {
            panic!("not a text node")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut d = Document::new(|_| {});

        let div = d.create_element("div");
        let hello = d.create_text_node("hello");

        d.insert_child(d.root(), div, 0);
        d.insert_child(div, hello, 0);
    }
}
