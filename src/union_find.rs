/// Fork of https://raw.githubusercontent.com/tov/disjoint-sets-rs/master/src/array.rs
/// License: https://github.com/tov/disjoint-sets-rs/blob/master/LICENSE-MIT

#[derive(Debug, Clone)]
pub struct UnionFind {
    parents: Vec<usize>,
    ranks: Vec<u8>,
}

impl UnionFind {
    pub fn new(size: usize) -> Self {
        UnionFind {
            parents: (0..size).collect(),
            ranks: vec![0; size],
        }
    }

    pub fn parent(&self) -> &Vec<usize> {
        &self.parents
    }

    pub fn reset(&mut self) {
        for i in 0..self.parents.len() {
            self.parents[i] = i;
            self.ranks[i] = 0;
        }
    }

    /// Joins the sets of the two given elements. Returns whether anything changed.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        if a == b {
            return false;
        }

        let a = self.find(a);
        let b = self.find(b);

        if a == b {
            return false;
        }

        if self.ranks[a] > self.ranks[b] {
            self.parents[b] = a;
        } else if self.ranks[b] > self.ranks[a] {
            self.parents[a] = b;
        } else {
            self.parents[a] = b;
            self.ranks[b] += 1;
        }

        true
    }

    /// Finds the representative of the given element’s set.
    /// https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Finding_set_representatives
    pub fn find(&mut self, mut element: usize) -> usize {
        let mut parent = self.parents[element];
        while element != parent {
            let grandparent = self.parents[parent];
            self.parents[element] = grandparent;
            element = parent;
            parent = grandparent;
        }
        element
    }

    /// The parent of each element is its root
    pub fn force(&mut self) {
        for element in 0..self.parents.len() {
            self.parents[element] = self.find(element);
        }
    }

    pub fn into_roots(mut self) -> Vec<usize> {
        self.force();
        self.parents
    }
}
