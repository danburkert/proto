use std::collections::HashMap;

use petgraph::algo::has_path_connecting;
use petgraph::graph::NodeIndex;
use petgraph::Graph;

use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto,
};

use crate::{fully_qualified_name::FullyQualifiedName, path::PathMap};

/// `MessageGraph` builds a graph of messages whose edges correspond to nesting.
/// The goal is to recognize when message types are recursively nested, so
/// that fields can be boxed when necessary.
pub struct MessageGraph {
    index: HashMap<String, NodeIndex>,
    graph: Graph<String, ()>,
    messages: HashMap<String, DescriptorProto>,
    boxed: PathMap<()>,
}

impl MessageGraph {
    pub(crate) fn new<'a>(
        files: impl Iterator<Item = &'a FileDescriptorProto>,
        boxed: PathMap<()>,
    ) -> MessageGraph {
        let mut msg_graph = MessageGraph {
            index: HashMap::new(),
            graph: Graph::new(),
            messages: HashMap::new(),
            boxed,
        };

        for file in files {
            let package = format!(
                "{}{}",
                if file.package.is_some() { "." } else { "" },
                file.package.as_ref().map(String::as_str).unwrap_or("")
            );
            for msg in &file.message_type {
                msg_graph.add_message(&package, msg);
            }
        }

        msg_graph
    }

    fn get_or_insert_index(&mut self, msg_name: String) -> NodeIndex {
        let MessageGraph {
            ref mut index,
            ref mut graph,
            ..
        } = *self;
        assert_eq!(b'.', msg_name.as_bytes()[0]);
        *index
            .entry(msg_name.clone())
            .or_insert_with(|| graph.add_node(msg_name))
    }

    /// Adds message to graph IFF it contains a non-repeated field containing another message.
    /// The purpose of the message graph is detecting recursively nested messages and co-recursively nested messages.
    /// Because prost does not box message fields, recursively nested messages would not compile in Rust.
    /// To allow recursive messages, the message graph is used to detect recursion and automatically box the recursive field.
    /// Since repeated messages are already put in a Vec, boxing them isn’t necessary even if the reference is recursive.
    fn add_message(&mut self, package: &str, msg: &DescriptorProto) {
        let msg_name = format!("{}.{}", package, msg.name.as_ref().unwrap());
        let msg_index = self.get_or_insert_index(msg_name.clone());

        for field in &msg.field {
            if field.r#type() == Type::Message && field.label() != Label::Repeated {
                let field_index = self.get_or_insert_index(field.type_name.clone().unwrap());
                self.graph.add_edge(msg_index, field_index, ());
            }
        }
        self.messages.insert(msg_name.clone(), msg.clone());

        for msg in &msg.nested_type {
            self.add_message(&msg_name, msg);
        }
    }

    /// Try get a message descriptor from current message graph
    pub fn get_message(&self, message: &str) -> Option<&DescriptorProto> {
        self.messages.get(message)
    }

    /// Returns true if message type `inner` is nested in message type `outer`.
    pub fn is_nested(&self, outer: &str, inner: &str) -> bool {
        let outer = match self.index.get(outer) {
            Some(outer) => *outer,
            None => return false,
        };
        let inner = match self.index.get(inner) {
            Some(inner) => *inner,
            None => return false,
        };

        has_path_connecting(&self.graph, outer, inner, None)
    }

    /// Returns `true` if this message can automatically derive Copy trait.
    pub fn can_message_derive_copy(&self, fq_message_name: &FullyQualifiedName) -> bool {
        self.get_message(fq_message_name.as_ref())
            .unwrap()
            .field
            .iter()
            .all(|field| self.can_field_derive_copy(fq_message_name, field))
    }

    /// Returns `true` if the type of this field allows deriving the Copy trait.
    pub fn can_field_derive_copy(
        &self,
        fq_message_name: &FullyQualifiedName,
        field: &FieldDescriptorProto,
    ) -> bool {
        // repeated field cannot derive Copy
        if field.label() == Label::Repeated {
            false
        } else if field.r#type() == Type::Message {
            // nested and boxed messages cannot derive Copy
            if self.is_nested(field.type_name(), fq_message_name.as_ref())
                || self
                    .boxed
                    .get_first_field(fq_message_name, field.name())
                    .is_some()
            {
                false
            } else {
                self.can_message_derive_copy(&FullyQualifiedName::from_type_name(field.type_name()))
            }
        } else {
            matches!(
                field.r#type(),
                Type::Float
                    | Type::Double
                    | Type::Int32
                    | Type::Int64
                    | Type::Uint32
                    | Type::Uint64
                    | Type::Sint32
                    | Type::Sint64
                    | Type::Fixed32
                    | Type::Fixed64
                    | Type::Sfixed32
                    | Type::Sfixed64
                    | Type::Bool
                    | Type::Enum
            )
        }
    }
}
