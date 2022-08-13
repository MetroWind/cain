use std::collections::HashMap;

use serde::ser;
use serde::ser::{SerializeStruct, SerializeSeq};
use serde::{Serialize, Deserialize};

use crate::error;
use crate::error::Error;
use crate::data_types::Category;

pub trait TreeData
{
    fn id(&self) -> i64;
}

struct TreeNode<T: TreeData + Serialize>
{
    data: T,
    parent_id: Option<i64>,
    children: Vec<i64>,
}

impl<T: TreeData + Serialize> TreeNode<T>
{
    fn new(d: T, p: Option<i64>) -> Self
    {
        Self { data: d, parent_id: p, children: Vec::new() }
    }

    fn id(&self) -> i64 { self.data.id() }

    fn data(&self) -> &T { &self.data }
}

impl TreeData for Category
{
    fn id(&self) -> i64 { self.id }
}

#[derive(Serialize)]
#[serde(untagged)]
enum SerializationValue<'a, T: Serialize>
{
    Data(&'a T),
    Children(Vec<HashMap<&'a str, SerializationValue<'a, T>>>),
}

struct NodeChildIterator<'a, T: TreeData + Serialize>
{
    tree: &'a Tree<T>,
    children: &'a [i64],
    current_idx: usize,
}

impl<'a, T: TreeData + Serialize> Iterator for NodeChildIterator<'a, T>
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item>
    {
        if self.current_idx >= self.children.len()
        {
            None
        }
        else
        {
            self.current_idx += 1;
            self.tree.findByID(self.children[self.current_idx - 1])
        }
    }
}

pub struct Tree<T: TreeData + Serialize>
{
    nodes: Vec<TreeNode<T>>,
    id_to_index: HashMap<i64, usize>,
}

impl<T: TreeData + Serialize> Tree<T>
{
    pub fn new(root: T) -> Self
    {
        let mut m: HashMap<i64, usize> = HashMap::new();
        m.insert(root.id(), 0 as usize);
        Self {
            nodes: vec![TreeNode::new(root, None)],
            id_to_index: m,
        }
    }

    pub fn addNode(&mut self, d: T, p: i64) -> Result<(), Error>
    {
        let i: usize = self.id_to_index.get(&p)
            .ok_or_else(|| rterr!("Parent not found"))?.clone();
        let id = d.id();
        self.nodes[i].children.push(id);
        self.nodes.push(TreeNode::new(d, Some(p)));
        self.id_to_index.insert(id, self.nodes.len() - 1);
        Ok(())
    }

    // pub fn removeNode(&mut self, id: i64) -> Result<(), Error>
    // {
    //     let i: usize = self.id_to_index.get(&id)
    //         .ok_or_else(|| rterr!("Parent not found"))?.clone();
    //     self.id_to_index.remove(&id);
    //     if i < self.nodes.len() - 1
    //     {
    //         self.nodes.swap_remove(i);
    //         self.id_to_index.insert(self.nodes[i].id(), i);
    //     }
    //     Ok(())
    // }

    fn nodeByID(&self, id: i64) -> Option<&TreeNode<T>>
    {
        let index: usize = if let Some(i) = self.id_to_index.get(&id)
        {
            i.clone()
        }
        else
        {
            return None;
        };
        self.nodes.get(index)
    }

    fn nodeByIDMut(&mut self, id: i64) -> Option<&mut TreeNode<T>>
    {
        let index: usize = if let Some(i) = self.id_to_index.get(&id)
        {
            i.clone()
        }
        else
        {
            return None;
        };
        self.nodes.get_mut(index)
    }

    pub fn findByID(&self, id: i64) -> Option<&T>
    {
        self.nodeByID(id).map(|node| &node.data)
    }

    pub fn findByIDMut(&mut self, id: i64) -> Option<&mut T>
    {
        self.nodeByIDMut(id).map(|node| &mut node.data)
    }

    pub fn modifyNode(&mut self, mut d: T) -> Result<(), Error>
    {
        if let Some(mut data) = self.findByIDMut(d.id())
        {
            data = &mut d;
            Ok(())
        }
        else
        {
            Err(rterr!("Modifing unexisting node: {}", d.id()))
        }
    }

    pub fn root(&self) -> &T { self.nodes[0].data() }

    pub fn children(&self, id: i64) -> Result<NodeChildIterator<T>, Error>
    {
        let node = self.nodeByID(id).ok_or_else(
            || rterr!("Node with ID {} not found", id))?;
        Ok(NodeChildIterator {
            tree: self,
            children: &node.children,
            current_idx: 0,
        })
    }

    fn serializeNode(&self, id: i64) ->
        Result<HashMap<&str, SerializationValue<T>>, Error>
    {
        let node: &TreeNode<T> = self.nodeByID(id).ok_or_else(
            || rterr!("ID not found: {}", id))?;
        let mut result = HashMap::new();
        result.insert("data", SerializationValue::<T>::Data(&node.data));
        let children: Result<Vec<_>, Error> =
            node.children.iter().map(|child_id| {
                self.serializeNode(child_id.clone())
            }).collect();

        result.insert("children", SerializationValue::<T>::Children(
            children?));
        Ok(result)
    }
}

impl<T: TreeData + Serialize> Serialize for Tree<T>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: ser::Serializer,
    {
        serializer.collect_map(self.serializeNode(self.root().id()).map_err(
            |e| ser::Error::custom(format!("{}", e)))?)
    }
}

// ========== Tests =================================================>

#[cfg(test)]
mod tests
{
    use super::*;
    use std::error::Error as StdError;
    use serde_json::json;

    #[derive(Clone, Debug, Serialize, PartialEq)]
    struct TestTreeData
    {
        id: i64,
    }

    impl TestTreeData
    {
        pub fn new(id: i64) -> Self
        {
            Self { id: id }
        }
    }

    impl TreeData for TestTreeData
    {
        fn id(&self) -> i64 { self.id }
    }

    #[test]
    fn createTree() -> Result<(), Error>
    {
        let tree = Tree::new(TestTreeData::new(1));
        assert_eq!(tree.root().id(), 1);
        Ok(())
    }

    #[test]
    fn addNode() -> Result<(), Error>
    {
        let mut tree = Tree::new(TestTreeData::new(1));
        tree.addNode(TestTreeData::new(2), 1)?;
        let d = tree.findByID(2).ok_or_else(|| rterr!("Shit happened"))?;
        assert_eq!(d.id(), 2);
        Ok(())
    }

    #[test]
    fn serialization() -> Result<(), serde_json::Error>
    {
        let tree = Tree::new(TestTreeData::new(1));
        assert_eq!(serde_json::to_value(&tree)?,
                   json!({
                       "data": { "id": 1 },
                       "children": []
                   }));
        Ok(())
    }

    #[test]
    fn serializationRecursive() -> Result<(), Box<dyn StdError>>
    {
        let mut tree = Tree::new(TestTreeData::new(1));
        tree.addNode(TestTreeData::new(2), 1)?;
        assert_eq!(serde_json::to_value(&tree)?,
                   json!({
                       "data": { "id": 1 },
                       "children": [{
                           "data": { "id": 2 },
                           "children": []
                       }]
                   }));
        Ok(())
    }

    #[test]
    fn iterator() -> Result<(), Box<dyn StdError>>
    {
        let mut tree = Tree::new(TestTreeData::new(1));
        tree.addNode(TestTreeData::new(2), 1)?;
        tree.addNode(TestTreeData::new(3), 1)?;
        let result: Vec<&TestTreeData> = tree.children(1)?.collect();
        assert_eq!(result.len(), 2);
        assert_eq!(*result[0], TestTreeData::new(2));
        assert_eq!(*result[1], TestTreeData::new(3));
        Ok(())
    }
}
