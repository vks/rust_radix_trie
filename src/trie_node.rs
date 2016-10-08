use {TrieNode, KeyValue, NibbleVec, BRANCH_FACTOR};
use keys::*;

macro_rules! no_children {
    () => ([
        None, None, None, None,
        None, None, None, None,
        None, None, None, None,
        None, None, None, None
    ])
}

impl<K, V> TrieNode<K, V> where K: TrieKey {
    /// Create a value-less, child-less TrieNode.
    pub fn new() -> TrieNode<K, V> {
        TrieNode {
            key: NibbleVec::new(),
            key_value: None,
            children: no_children![],
            child_count: 0,
        }
    }

    /// Create a TrieNode with no children.
    pub fn with_key_value(key_fragments: NibbleVec, key: K, value: V) -> TrieNode<K, V> {
        TrieNode {
            key: key_fragments,
            key_value: Some(Box::new(KeyValue { key: key, value: value })),
            children: no_children![],
            child_count: 0,
        }
    }

    /// Get the key stored at this node, if any.
    pub fn key(&self) -> Option<&K> {
        self.key_value.as_ref().map(|kv| &kv.key)
    }

    /// Get the value stored at this node, if any.
    pub fn value(&self) -> Option<&V> {
        self.key_value.as_ref().map(|kv| &kv.value)
    }

    /// Get a mutable reference to the value stored at this node, if any.
    pub fn value_mut(&mut self) -> Option<&mut V> {
        self.key_value.as_mut().map(|kv| &mut kv.value)
    }

    /// Get the value whilst checking a key match.
    pub fn value_checked(&self, key: &K) -> Option<&V> {
        self.key_value.as_ref().map(|kv| {
            check_keys(&kv.key, key);
            &kv.value
        })
    }

    /// Get a mutable value whilst checking a key match.
    pub fn value_checked_mut(&mut self, key: &K) -> Option<&mut V> {
        self.key_value.as_mut().map(|kv| {
            check_keys(&kv.key, key);
            &mut kv.value
        })
    }

    /// Add a child at the given index, given that none exists there already.
    pub fn add_child(&mut self, idx: usize, node: Box<TrieNode<K, V>>) {
        debug_assert!(self.children[idx].is_none());
        self.child_count += 1;
        self.children[idx] = Some(node);
    }

    /// Remove a child at the given index, if it exists.
    pub fn take_child(&mut self, idx: usize) -> Option<Box<TrieNode<K, V>>> {
        self.children[idx].take().map(|node| {
            self.child_count -= 1;
            node
        })
    }

    /// Helper function for removing the single child of a node.
    pub fn take_only_child(&mut self) -> Box<TrieNode<K, V>> {
        debug_assert!(self.child_count == 1);
        for i in 0 .. BRANCH_FACTOR {
            if let Some(child) = self.take_child(i) {
                return child;
            }
        }
        unreachable!("node with child_count 1 has no actual children");
    }

    /// Set the key and value of a node, given that it currently lacks one.
    pub fn add_key_value(&mut self, key: K, value: V) {
        debug_assert!(self.key_value.is_none());
        self.key_value = Some(Box::new(KeyValue { key: key, value: value }));
    }

    /// Move the value out of a node, whilst checking that its key is as expected.
    /// Can panic (see check_keys).
    pub fn take_value(&mut self, key: &K) -> Option<V> {
        self.key_value.take().map(|kv| {
            check_keys(&kv.key, key);
            kv.value
        })
    }

    /// Replace a value, returning the previous value if there was one.
    pub fn replace_value(&mut self, key: K, value: V) -> Option<V> {
        let previous = self.take_value(&key);
        self.add_key_value(key, value);
        previous
    }

    /// Get a reference to this node if it has a value.
    pub fn as_value_node(&self) -> Option<&TrieNode<K, V>> {
        self.key_value.as_ref().map(|_| self)
    }

    /// Split a node at a given index in its key, transforming it into a prefix node of its
    /// previous self.
    pub fn split(&mut self, idx: usize) {
        // Extract all the parts of the suffix node, starting with the key.
        let key = self.key.split(idx);

        // Key-value.
        let key_value = self.key_value.take();

        // Children.
        let mut children = no_children![];

        for (i, child) in self.children.iter_mut().enumerate() {
            if child.is_some() {
                children[i] = child.take();
            }
        }

        // Child count.
        let child_count = self.child_count;
        self.child_count = 1;

        // Insert the collected items below what is now an empty prefix node.
        let bucket = key.get(0) as usize;
        self.children[bucket] = Some(Box::new(
            TrieNode {
                key: key,
                key_value: key_value,
                children: children,
                child_count: child_count,
            }
        ));
    }

    /// Check the integrity of a trie subtree (quite costly).
    /// Return true and the size of the subtree if all checks are successful,
    /// or false and a junk value if any test fails.
    pub fn check_integrity_recursive(&self, prefix: &NibbleVec) -> (bool, usize) {
        let mut sub_tree_size = 0;
        let is_root = prefix.len() == 0;

        // Check that no value-less, non-root nodes have only 1 child.
        if !is_root && self.child_count == 1 && self.key_value.is_none() {
            println!("Value-less node with a single child.");
            return (false, sub_tree_size);
        }

        // Check that all non-root key vector's have length > 1.
        if !is_root && self.key.len() == 0 {
            println!("Key length is 0 at non-root node.");
            return (false, sub_tree_size);
        }

        // Check that the child count matches the actual number of children.
        let child_count = self.children.iter().fold(0, |acc, e| acc + (e.is_some() as usize));

        if child_count != self.child_count {
            println!("Child count error, recorded: {}, actual: {}", self.child_count, child_count);
            return (false, sub_tree_size);
        }

        // Compute the key fragments for this node, according to the trie.
        let trie_key = prefix.clone().join(&self.key);

        // Account for this node in the size check, and check its key.
        match self.key_value {
            Some(ref kv) => {
                sub_tree_size += 1;

                let actual_key = kv.key.encode();

                if trie_key != actual_key {
                    return (false, sub_tree_size);
                }
            }
            None => ()
        }

        // Recursively check children.
        for i in 0 .. BRANCH_FACTOR {
            if let Some(ref child) = self.children[i] {
                match child.check_integrity_recursive(&trie_key) {
                    (false, _) => return (false, sub_tree_size),
                    (true, child_size) => sub_tree_size += child_size
                }
            }
        }

        (true, sub_tree_size)
    }
}