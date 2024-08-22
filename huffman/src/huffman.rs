use std::{cmp::Reverse, collections::BinaryHeap};

use crate::HashMap;

#[derive(Clone)]
enum HuffmanNode {
    Leaf {
        element: char,
        weight: u32,
    },
    Internal {
        weight: u32,
        left: Box<HuffmanNode>,
        right: Box<HuffmanNode>,
    },
}

impl HuffmanNode {
    fn new_leaf(element: char, weight: u32) -> HuffmanNode {
        HuffmanNode::Leaf { element, weight }
    }

    fn new_internal(left: HuffmanNode, right: HuffmanNode, weight: u32) -> HuffmanNode {
        HuffmanNode::Internal {
            weight,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn weight(&self) -> u32 {
        match self {
            Self::Leaf { weight, .. } => *weight,
            Self::Internal { weight, .. } => *weight,
        }
    }

    #[allow(dead_code)]
    fn left(&self) -> Option<HuffmanNode> {
        match self {
            Self::Leaf { .. } => None,
            Self::Internal { left, .. } => Some(*left.clone()),
        }
    }

    #[allow(dead_code)]
    fn right(&self) -> Option<HuffmanNode> {
        match self {
            Self::Leaf { .. } => None,
            Self::Internal { right, .. } => Some(*right.clone()),
        }
    }

    #[allow(dead_code)]
    fn elem(&self) -> Option<char> {
        match self {
            Self::Leaf { element, .. } => Some(element.clone()),
            Self::Internal { .. } => None,
        }
    }
}

pub struct HuffmanTree {
    root: HuffmanNode,
}

impl PartialEq for HuffmanTree {
    fn eq(&self, other: &Self) -> bool {
        self.root.weight() == other.root.weight()
    }
}

impl PartialOrd for HuffmanTree {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.root.weight().cmp(&other.root.weight()))
    }
}

impl Eq for HuffmanTree {}

impl Ord for HuffmanTree {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.root.weight().cmp(&other.root.weight())
    }
}

impl HuffmanTree {
    pub fn build_huffman(freq: HashMap<char, u32>) -> Option<HuffmanTree> {
        if freq.len() < 2 {
            return None;
        }

        let mut heap = BinaryHeap::new();

        for (elem, weight) in freq {
            heap.push(Reverse(HuffmanTree {
                root: HuffmanNode::new_leaf(elem, weight),
            }));
        }

        while heap.len() > 1 {
            let Reverse(left_tree) = heap.pop().unwrap();
            let Reverse(right_tree) = heap.pop().unwrap();

            let new_weight = right_tree.root.weight() + left_tree.root.weight();

            let new_tree = HuffmanTree {
                root: HuffmanNode::new_internal(left_tree.root, right_tree.root, new_weight),
            };

            heap.push(Reverse(new_tree));
        }

        Some(heap.pop().unwrap().0)
    }

    pub fn generate_prefix_codes(tree: HuffmanTree) -> HashMap<char, String> {
        let mut codes = HashMap::new();
        HuffmanTree::recursive_generate_codes(&tree.root, &mut String::new(), &mut codes);

        codes
    }

    fn recursive_generate_codes(
        node: &HuffmanNode,
        prefix: &mut String,
        code_table: &mut HashMap<char, String>,
    ) {
        match node {
            HuffmanNode::Leaf { element, .. } => {
                code_table.insert(*element, prefix.clone());
            }
            HuffmanNode::Internal { left, right, .. } => {
                prefix.push('0');
                HuffmanTree::recursive_generate_codes(left, prefix, code_table);
                prefix.pop();

                prefix.push('1');
                HuffmanTree::recursive_generate_codes(right, prefix, code_table);
                prefix.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HuffmanTree;
    use std::collections::HashMap;

    #[test]
    fn test_one_node() {
        let mut hashmap = HashMap::new();
        hashmap.insert('A', 70);

        let tree = HuffmanTree::build_huffman(hashmap);

        assert!(tree.is_none());
    }

    #[test]
    fn test_two_nodes() {
        let mut hashmap = HashMap::new();
        hashmap.insert('A', 70);
        hashmap.insert('B', 89);

        let tree = HuffmanTree::build_huffman(hashmap).unwrap();

        assert_eq!(tree.root.weight(), 159);
        assert_eq!(tree.root.left().unwrap().weight(), 70);
        assert_eq!(tree.root.right().unwrap().weight(), 89);
    }

    #[test]
    fn test_multi_nodes() {
        let mut hashmap = HashMap::new();
        hashmap.insert('C', 32);
        hashmap.insert('D', 42);
        hashmap.insert('E', 120);
        hashmap.insert('K', 7);
        hashmap.insert('L', 42);
        hashmap.insert('M', 24);
        hashmap.insert('U', 37);
        hashmap.insert('Z', 2);

        let tree = HuffmanTree::build_huffman(hashmap).unwrap();

        assert_eq!(tree.root.weight(), 306);
        assert_eq!(tree.root.left().unwrap().weight(), 120);
        assert_eq!(tree.root.right().unwrap().weight(), 186);
        assert_eq!(tree.root.left().unwrap().elem().unwrap(), 'E');
        assert!(tree.root.right().unwrap().elem().is_none());
    }
}
