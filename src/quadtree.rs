use crate::list::List;

pub trait Visitor {
    fn entity(&mut self, entity_id: usize, idx: usize, next_entity: Option<usize>, x: i32, y: i32, width: i32, height: i32);
    fn leaf(&mut self, depth: u8, idx: usize, num_children: Option<usize>, first_entity: Option<usize>, x: i32, y: i32, width: i32, height: i32);
    fn branch(&mut self, depth: u8, idx: usize, first_leaf: usize, x: i32, y: i32, width: i32, height: i32);
}

#[derive(Copy, Clone, Debug)]
struct EntityNode {
    next: Option<usize>,
    entity: usize,
}

impl Default for EntityNode {
    fn default() -> Self {
        Self {
            next: None,
            entity: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Entity {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Node {
    first_child: Option<usize>,
    num_children: Option<usize>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            first_child: None,
            num_children: Some(0),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct NodeData {
    idx: usize,
    depth: u8,
    x: i32,
    y: i32,
    hx: i32,
    hy: i32,
}

impl Default for NodeData {
    fn default() -> Self {
        Self {
            idx: 0,
            depth: 0,
            x: 0,
            y: 0,
            hx: 0,
            hy: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Quadtree {
    root: NodeData,
    max_entities: u16,
    max_depth: u8,
    entity_nodes: List<EntityNode>,
    entities: List<Entity>,
    nodes: List<Node>,
}

impl Quadtree {
    pub fn new(x: f32, y: f32, width: f32, height: f32, max_entities_per_region: u16) -> Self {
        let mut nodes = List::default();
        let root_idx = nodes.insert(Node::default());
        let width = width as i32;
        let height = height as i32;
        Self {
            root: NodeData {
                idx: root_idx,
                depth: 0,
                x: x as i32,
                y: y as i32,
                hx: width / 2,
                hy: height / 2,
            },
            max_entities: max_entities_per_region,
            max_depth: Self::calc_max_depth(width, height),
            nodes,
            entity_nodes: List::default(),
            entities: List::default(),
        }
    }

    fn calc_max_depth(w: i32, h: i32) -> u8 {
        let mut depth: u8 = 0;
        let mut size = match w <= h {
            true => w,
            false => h,
        };
        while size > 2 {
            size = size / 2;
            depth += 1;
        }
        depth
    }

    pub fn insert(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> usize {
        let new_entity_idx = self.entities.insert(Entity {
            left: x1 as i32,
            top: y1 as i32,
            right: x2 as i32,
            bottom: y2 as i32,
        });
        self.node_insert(self.root, new_entity_idx);
        new_entity_idx
    }

    pub fn remove(&mut self, entity_idx: usize) {
        // Find the leaves.
        let entity = self.entities.get(entity_idx);
        let leaves = self.find_leaves(self.root, entity.left, entity.top, entity.right, entity.bottom);

        // For each leaf node, remove the element node.
        for i in 0..leaves.cursor() {
            let nd_data_idx = leaves.get(i).idx;

            // Walk the list until we find the element node.
            let mut node_idx = self.nodes.get(nd_data_idx).first_child;
            let mut prev_index = None;
            while node_idx.is_some() && self.entity_nodes.get(node_idx.unwrap()).entity != entity_idx {
                prev_index = node_idx;
                node_idx = self.entity_nodes.get(node_idx.unwrap()).next;
            }

            if node_idx.is_some() {
                // Remove the element node.
                let next_index = self.entity_nodes.get(node_idx.unwrap()).next;
                if prev_index.is_none() {
                    self.nodes.get_mut(nd_data_idx).first_child = next_index;
                } else {
                    self.entity_nodes.get_mut(prev_index.unwrap()).next = next_index;
                }
                self.entity_nodes.erase(node_idx.unwrap());

                // Decrement the leaf element count.
                let num_children = self.nodes.get(nd_data_idx).num_children.unwrap();
                if num_children == 0 {
                    self.nodes.get_mut(nd_data_idx).num_children = None;
                } else {
                    self.nodes.get_mut(nd_data_idx).num_children = Some(num_children-1);
                }
            }
        }

        // Remove the element.
        self.entities.erase(entity_idx);
    }

    pub fn cleanup(&mut self) {
        let mut to_process = List::<usize>::default();

        // Only process the root if it's not a leaf.
        if self.nodes.get(self.root.idx).num_children.is_none() {
            // Push the root index to the stack.
            to_process.push(self.root.idx);
        }

        while to_process.cursor() > 0 {
            // Pop a node from the stack.
            let node_idx = to_process.pop();
            let node = self.nodes.get(node_idx);
            let mut num_empty_leaves = 0;

            // Loop through the children.
            for i in 0..4 {
                let child_idx = node.first_child.unwrap() + i;
                // Increment empty leaf count if the child is an empty
                // leaf. Otherwise, if the child is a branch, add it to
                // the stack to be processed in the next iteration.
                let child_node = self.nodes.get(child_idx);
                if child_node.num_children.is_none() {
                    // Push the child index to the stack.
                    to_process.push(child_idx);
                } else if child_node.num_children.unwrap() == 0 {
                    num_empty_leaves += 1;
                }
            }

            // If all the children were empty leaves, remove them and
            // make this node the new empty leaf.
            if num_empty_leaves == 4 {
                // Remove all 4 children in reverse order so that they
                // can be reclaimed on subsequent insertions in proper
                // order.
                let fc = node.first_child.unwrap();
                self.nodes.erase(fc + 3);
                self.nodes.erase(fc + 2);
                self.nodes.erase(fc + 1);
                self.nodes.erase(fc + 0);

                // Make this node the new empty leaf.
                self.nodes.get_mut(node_idx).first_child = None;
                self.nodes.get_mut(node_idx).num_children = Some(0);
            }
        }
    }

    pub fn query(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> Vec<usize> {
        self.query_omit(x1, y1, x2, y2, None)
    }

    pub fn query_omit(&self, x1: f32, y1: f32, x2: f32, y2: f32, omit_entity_id: Option<usize>) -> Vec<usize> {
        let mut out = Vec::<usize>::new();

        // Find the leaves that intersect the specified query rectangle.
        let q_left = x1 as i32;
        let q_top = y1 as i32;
        let q_right = x2 as i32;
        let q_bottom = y2 as i32;
        let leaves = self.find_leaves(self.root, q_left, q_top, q_right, q_bottom);

        let mut seen = Vec::<bool>::new();
        seen.resize(self.entities.cursor(), false);

        // For each leaf node, look for elements that intersect.
        for i in 0..leaves.cursor() {
            let nd_data_idx = leaves.get(i).idx;

            // Walk the list and add elements that intersect.
            let mut next_enode_idx = self.nodes.get(nd_data_idx).first_child;
            while next_enode_idx.is_some() {
                let entity_node = self.entity_nodes.get(next_enode_idx.unwrap());
                let entity = self.entities.get(entity_node.entity);
                if !seen[entity_node.entity]
                    && !(omit_entity_id.is_some() && entity_node.entity == omit_entity_id.unwrap())
                    && Self::intersect(
                    q_left,
                    q_top,
                    q_right,
                    q_bottom,
                    entity.left,
                    entity.top,
                    entity.right,
                    entity.bottom,
                )
                {
                    out.push(entity_node.entity);
                    seen[entity_node.entity] = true;
                }
                next_enode_idx = entity_node.next;
            }
        }
        out
    }

    pub fn traverse(&self, visitor: &mut impl Visitor) {
        let mut to_process = List::<NodeData>::default();
        to_process.push(self.root);

        while to_process.cursor() > 0 {
            let nd_data = to_process.pop();

            let node = self.nodes.get(nd_data.idx);
            if node.num_children.is_none() {
                // Push the children of the branch to the stack.
                let fc = self.nodes.get(nd_data.idx).first_child.unwrap();
                let qx = nd_data.hx >> 1;
                let qy = nd_data.hy >> 1;
                let l = nd_data.x - qx;
                let t = nd_data.y - qy;
                let r = nd_data.x + qx;
                let b = nd_data.y + qy;
                to_process.push(NodeData{idx:fc + 0, depth:nd_data.depth + 1, x:l, y:t, hx:qx, hy:qy});
                to_process.push(NodeData{idx:fc + 1, depth:nd_data.depth + 1, x:r, y:t, hx:qx, hy:qy});
                to_process.push(NodeData{idx:fc + 2, depth:nd_data.depth + 1, x:l, y:b, hx:qx, hy:qy});
                to_process.push(NodeData{idx:fc + 3, depth:nd_data.depth + 1, x:r, y:b, hx:qx, hy:qy});
                visitor.branch(
                    nd_data.depth,
                    nd_data.idx,
                    fc,
                    nd_data.x,
                    nd_data.y,
                    nd_data.hx << 1,
                    nd_data.hy << 1,
                );
            } else {
                visitor.leaf(
                    nd_data.depth,
                    nd_data.idx,
                    node.num_children,
                    node.first_child,
                    nd_data.x,
                    nd_data.y,
                    nd_data.hx << 1,
                    nd_data.hy << 1,
                );
                let mut node_idx = self.nodes.get(nd_data.idx).first_child;
                while node_idx != None {
                    let entity_node= self.entity_nodes.get(node_idx.unwrap());
                    let entity = self.entities.get(entity_node.entity);
                    let w = entity.right - entity.left;
                    let h = entity.bottom - entity.top;
                    let x = entity.left + (w>>1);
                    let y = entity.top + (h>>1);
                    visitor.entity(entity_node.entity, node_idx.unwrap(), entity_node.next, x, y, w, h);
                    node_idx = entity_node.next
                }
            }
        }
    }

    fn intersect(l1: i32, t1: i32, r1: i32, b1: i32, l2: i32, t2: i32, r2: i32, b2: i32) -> bool {
        l2 <= r1 && r2 >= l1 && t2 <= b1 && b2 >= t1
    }

    fn find_leaves(
        &self,
        start_node: NodeData,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> List<NodeData> {
        let mut leaves = List::<NodeData>::default();
        let mut to_process = List::<NodeData>::default();
        to_process.push(start_node);

        while to_process.cursor() > 0 {
            let nd_data = to_process.pop();
            if self.nodes.get(nd_data.idx).num_children.is_some() {
                leaves.push(nd_data);
            } else {
                let fc = self.nodes.get(nd_data.idx).first_child.unwrap();
                let qx = nd_data.hx >> 1;
                let qy = nd_data.hy >> 1;
                let l = nd_data.x - qx;
                let t = nd_data.y - qy;
                let r = nd_data.x + qx;
                let b = nd_data.y + qy;

                if top <= nd_data.y {
                    if left <= nd_data.x {
                        to_process.push(NodeData {
                            idx: fc + 0,
                            depth: nd_data.depth + 1,
                            x: l,
                            y: t,
                            hx: qx,
                            hy: qy,
                        });
                    }
                    if right > nd_data.x {
                        to_process.push(NodeData {
                            idx: fc + 1,
                            depth: nd_data.depth + 1,
                            x: r,
                            y: t,
                            hx: qx,
                            hy: qy,
                        });
                    }
                }
                if bottom > nd_data.y {
                    if left <= nd_data.x {
                        to_process.push(NodeData {
                            idx: fc + 2,
                            depth: nd_data.depth + 1,
                            x: l,
                            y: b,
                            hx: qx,
                            hy: qy,
                        });
                    }
                    if right > nd_data.x {
                        to_process.push(NodeData {
                            idx: fc + 3,
                            depth: nd_data.depth + 1,
                            x: r,
                            y: b,
                            hx: qx,
                            hy: qy,
                        });
                    }
                }
            }
        }
        return leaves;
    }

    fn node_insert(&mut self, start_node: NodeData, entity_idx: usize) {
        let entity = self.entities.get(entity_idx);
        let leaves = self.find_leaves(
            start_node,
            entity.left,
            entity.top,
            entity.right,
            entity.bottom,
        );

        for i in 0..leaves.cursor() {
            let nd_data = leaves.get(i);
            self.leaf_insert(*nd_data, entity_idx);
        }
    }

    fn leaf_insert(&mut self, node_data: NodeData, entity_idx: usize) {
        let first_child = self.nodes.get(node_data.idx).first_child;
        let e_node = self.entity_nodes.push(EntityNode {
            entity: entity_idx,
            next: first_child,
        });
        self.nodes.get_mut(node_data.idx).first_child = Some(e_node);

        // If the leaf is full, split it.
        if self.nodes.get(node_data.idx).num_children.unwrap() == (self.max_entities as usize) && node_data.depth < self.max_depth {
            // Transfer elements from the leaf node to a list of elements.
            let mut entities = List::<usize>::default();
            while self.nodes.get(node_data.idx).first_child.is_some() {
                let index = self.nodes.get(node_data.idx).first_child;
                let e_node = *self.entity_nodes.get(index.unwrap());

                // Pop off the element node from the leaf and remove it from the qt.
                self.nodes.get_mut(node_data.idx).first_child = e_node.next;
                self.entity_nodes.erase(index.unwrap());

                // Insert element to the list.
                entities.push(e_node.entity);
            }

            // Initialize 4 child nodes.
            let fc = self.nodes.insert(Node::default());
            self.nodes.insert(Node::default());
            self.nodes.insert(Node::default());
            self.nodes.insert(Node::default());

            self.nodes.get_mut(node_data.idx).first_child = Some(fc);
            self.nodes.get_mut(node_data.idx).num_children = None;

            // Transfer the elements in the former leaf node to its new children.
            for i in 0..entities.cursor() {
                self.node_insert(node_data, *entities.get(i));
            }
        } else {
            // Increment the leaf element count.
            let num_children = self.nodes.get_mut(node_data.idx).num_children.unwrap();
            self.nodes.get_mut(node_data.idx).num_children = Some(num_children+1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestVisitor {
        entities: Vec<usize>,
        leaves: Vec<u8>,
        branches: Vec<u8>,
    }

    impl TestVisitor {
        fn new() -> Self {
            Self {
                entities: Vec::new(),
                leaves: Vec::new(),
                branches: Vec::new()
            }
        }

        fn reset(&mut self) {
            self.entities.clear();
            self.leaves.clear();
            self.branches.clear();
            println!("\n/////// RESET ///////\n")
        }

        fn assert_counts(&self, entities: usize, leaves: usize, branches: usize) {
            assert_eq!(self.entities.len(), entities);
            assert_eq!(self.leaves.len(), leaves);
            assert_eq!(self.branches.len(), branches);
        }
    }

    impl Visitor for TestVisitor {
        fn entity(&mut self, entity_id: usize, idx: usize, next_entity: Option<usize>, _x: i32, _y: i32, _width: i32, _height: i32) {
            println!("----[EN: {entity_id} idx:{idx}->{next_entity:?}]");
            self.entities.push(entity_id);
        }

        fn leaf(&mut self, depth: u8, idx: usize, num_children: Option<usize>, first_entity: Option<usize>, x: i32, y: i32, w: i32, h: i32) {
            println!("--[LF: {idx}, children: {num_children:?}, first_entity: {first_entity:?}, d:{depth}, x:{x}, y:{y}, w:{w}, h:{h}]");
            self.leaves.push(depth);
        }

        fn branch(&mut self, depth: u8, idx: usize, first_leaf: usize, x: i32, y: i32, w: i32, h: i32) {
            println!("[BR: {idx},  d:{depth}, first_leaf:{first_leaf}, x:{x}, y:{y}, w:{w}, h:{h}]");
            self.branches.push(depth);
        }
    }

    #[test]
    fn calc_max_depth() {
        // Test expected boundaries for depths
        for x in 1..=30u8 {
            // The minimum size (height or width) of a region is 2 units.
            // Because each subdivision is a divide by two,
            // depth boundaries naturally occur at powers of 2.
            // However, since it is all integer division,
            // the imprecision shifts the boundaries by +(power_of_2/2)
            // Example:
            // 32 = 2^5
            // With imprecision, we have to add 50%:
            // 32 + (32/2) = 48
            // This means 47 is the upper bound of depth 4
            // and 48 is the lower bound of depth 5.
            let prev_x = x-1;
            let power: i32 = 1 << x;
            let next_lower = power + (power>>1);
            let prev_upper = next_lower - 1;
            assert_eq!(Quadtree::calc_max_depth(prev_upper, prev_upper), prev_x);
            assert_eq!(Quadtree::calc_max_depth(next_lower, next_lower), x);
        }
    }

    #[test]
    fn new() {
        let qt = Quadtree::new(10.0, 15.0, 100.0, 100.0, 8);
        assert_eq!(qt.max_depth, 6);
        assert_eq!(qt.max_entities, 8);
        assert_eq!(qt.root.depth, 0);
        assert_eq!(qt.root.idx, 0);
        assert_eq!(qt.root.x, 10);
        assert_eq!(qt.root.y, 15);
        assert_eq!(qt.root.hx, 50);
        assert_eq!(qt.root.hy, 50);
        assert_eq!(qt.entities.cursor(), 0);
        assert_eq!(qt.entity_nodes.cursor(), 0);
        assert_eq!(qt.nodes.cursor(), 1);
    }

    #[test]
    fn insert_and_traverse() {
        let mut qt = Quadtree::new(0.0, 0.0, 100.0, 100.0, 4);
        let entity = qt.insert(-30.0, -30.0, 70.0, 70.0);
        assert_eq!(entity, 0);

        /***
        |-----------------------|
        |                       |
        |                       |
        |                       |
        |           x           |
        |                       |
        |                       |
        |                       |
        |-----------------------|
        Because the first entity is so large and covers most of the root quad,
        we will see it occupy leaves alongside the other smaller entities that
        get inserted. This will explain why subdivided leaves have 2 entities
        per leaf, denoted by the `| x x |`
         */
        let mut tv = TestVisitor::new();
        qt.traverse(&mut tv);
        tv.assert_counts(1, 1, 0);
        tv.reset();

        // Fill each leaf, which should trigger subdivides
        // NW
        /***
        |-----------------------|
        | x x | x x |           |
        |-----|-----|     x     |
        | x x | x x |           |
        |-----------|-----------|
        |           |           |
        |     x     |     x     |
        |           |           |
        |-----------------------|
         */
        qt.insert(-40.0, 30.0, -30.0, 40.0);
        qt.insert(-40.0, 10.0, -30.0, 20.0);
        qt.insert(-20.0, 30.0, -10.0, 40.0);
        qt.insert(-20.0, 10.0, -10.0, 20.0);
        qt.traverse(&mut tv);
        tv.assert_counts(11, 7, 2);
        tv.reset();

        // NE
        /***
        |-----------------------|
        | x x | x x | x x | x x |
        |-----|-----|-----|-----|
        | x x | x x | x x | x x |
        |-----------|-----------|
        |           |           |
        |     x     |     x     |
        |           |           |
        |-----------------------|
         */
        qt.insert(30.0, 30.0, 40.0, 40.0);
        qt.insert(30.0, 10.0, 40.0, 20.0);
        qt.insert(10.0, 30.0, 20.0, 40.0);
        qt.insert(10.0, 10.0, 20.0, 20.0);
        qt.traverse(&mut tv);
        tv.assert_counts(18, 10, 3);
        tv.reset();

        // SW
        /***
        |-----------------------|
        | x x | x x | x x | x x |
        |-----|-----|-----|-----|
        | x x | x x | x x | x x |
        |-----|-----|-----------|
        | x x | x x |           |
        |-----|-----|     x     |
        | x x | x x |           |
        |-----------------------|
         */
        qt.insert(-40.0, -40.0, -30.0, -30.0);
        qt.insert(-40.0, -20.0, -30.0, -10.0);
        qt.insert(-20.0, -40.0, -10.0, -30.0);
        qt.insert(-20.0, -20.0, -10.0, -10.0);
        qt.traverse(&mut tv);
        tv.assert_counts(25, 13, 4);
        tv.reset();

        // SE
        /***
        |-----------------------|
        | x x | x x | x x | x x |
        |-----|-----|-----|-----|
        | x x | x x | x x | x x |
        |-----|-----|-----|-----|
        | x x | x x | x x | x x |
        |-----|-----|-----|-----|
        | x x | x x | x x | x x |
        |-----------------------|
         */
        qt.insert(30.0, -40.0, 40.0, -30.0);
        qt.insert(30.0, -20.0, 40.0, -10.0);
        qt.insert(10.0, -40.0, 20.0, -30.0);
        qt.insert(10.0, -20.0, 20.0, -10.0);
        qt.traverse(&mut tv);
        tv.assert_counts(32, 16, 5);
        tv.reset();
    }

    #[test]
    fn query_and_omit() {
        let mut qt = Quadtree::new(0.0, 0.0, 100.0, 100.0, 4);

        // Cover almost entire quadtree root
        let entity = qt.insert(-30.0, -30.0, 70.0, 70.0);
        let q = qt.query(-10.0, -10.0, 10.0, 10.0);
        assert!(q.contains(&0));

        // NW
        qt.insert(-40.0, 30.0, -30.0, 40.0);
        qt.insert(-40.0, 10.0, -30.0, 20.0);
        qt.insert(-20.0, 30.0, -10.0, 40.0);
        qt.insert(-20.0, 10.0, -10.0, 20.0);
        let q = qt.query(-50.0, 0.0, 0.0, 50.0);
        let q_omit = qt.query_omit(-50.0, 0.0, 0.0, 50.0, Some(entity));
        assert_eq!(q.len(), 5);
        assert!(q.contains(&0));
        assert!(q.contains(&1));
        assert!(q.contains(&2));
        assert!(q.contains(&3));
        assert!(q.contains(&4));
        assert!(!q_omit.contains(&0) && q_omit.len() == 4);

        // NE
        qt.insert(30.0, 30.0, 40.0, 40.0);
        qt.insert(30.0, 10.0, 40.0, 20.0);
        qt.insert(10.0, 30.0, 20.0, 40.0);
        qt.insert(10.0, 10.0, 20.0, 20.0);
        let q = qt.query(0.0, 0.0, 50.0, 50.0);
        let q_omit = qt.query_omit(0.0, 0.0, 50.0, 50.0, Some(entity));
        assert_eq!(q.len(), 5);
        assert!(q.contains(&0));
        assert!(q.contains(&5));
        assert!(q.contains(&6));
        assert!(q.contains(&7));
        assert!(q.contains(&8));
        assert!(!q_omit.contains(&0) && q_omit.len() == 4);

        // SW
        qt.insert(-40.0, -40.0, -30.0, -30.0);
        qt.insert(-40.0, -20.0, -30.0, -10.0);
        qt.insert(-20.0, -40.0, -10.0, -30.0);
        qt.insert(-20.0, -20.0, -10.0, -10.0);
        let q = qt.query(-50.0, -50.0, 0.0, 0.0);
        let q_omit = qt.query_omit(-50.0, -50.0, 0.0, 0.0, Some(entity));
        assert_eq!(q.len(), 5);
        assert!(q.contains(&0));
        assert!(q.contains(&9));
        assert!(q.contains(&10));
        assert!(q.contains(&11));
        assert!(q.contains(&12));
        assert!(!q_omit.contains(&0) && q_omit.len() == 4);

        // SE
        qt.insert(30.0, -40.0, 40.0, -30.0);
        qt.insert(30.0, -20.0, 40.0, -10.0);
        qt.insert(10.0, -40.0, 20.0, -30.0);
        qt.insert(10.0, -20.0, 20.0, -10.0);
        let q = qt.query(0.0, -50.0, 50.0, 0.0);
        let q_omit = qt.query_omit(0.0, -50.0, 50.0, 0.0, Some(entity));
        assert_eq!(q.len(), 5);
        assert!(q.contains(&0));
        assert!(q.contains(&13));
        assert!(q.contains(&14));
        assert!(q.contains(&15));
        assert!(q.contains(&16));
        assert!(!q_omit.contains(&0) && q_omit.len() == 4);

        // Center
        let q = qt.query(-10.0, -10.0, 10.0, 10.0);
        let q_omit = qt.query_omit(-10.0, -10.0, 10.0, 10.0, Some(entity));
        assert_eq!(q.len(), 5);
        assert!(q.contains(&0));
        assert!(q.contains(&4));
        assert!(q.contains(&8));
        assert!(q.contains(&12));
        assert!(q.contains(&16));
        assert!(!q_omit.contains(&0) && q_omit.len() == 4);
    }

    #[test]
    fn remove_and_cleanup() {
        let mut qt = Quadtree::new(0.0, 0.0, 100.0, 100.0, 4);

        // Populate the quadtree
        // Large centered entity
        qt.insert(-30.0, -30.0, 70.0, 70.0); // 0

        // NW
        qt.insert(-40.0, 30.0, -30.0, 40.0); // 1
        qt.insert(-40.0, 10.0, -30.0, 20.0); // 2
        qt.insert(-20.0, 30.0, -10.0, 40.0); // 3
        qt.insert(-20.0, 10.0, -10.0, 20.0); // 4

        // NE
        qt.insert(30.0, 30.0, 40.0, 40.0); // 5
        qt.insert(30.0, 10.0, 40.0, 20.0); // 6
        qt.insert(10.0, 30.0, 20.0, 40.0); // 7
        qt.insert(10.0, 10.0, 20.0, 20.0); // 8

        // SW
        qt.insert(-40.0, -40.0, -30.0, -30.0); // 9
        qt.insert(-40.0, -20.0, -30.0, -10.0); // 10
        qt.insert(-20.0, -40.0, -10.0, -30.0); // 11
        qt.insert(-20.0, -20.0, -10.0, -10.0); // 12

        // SE
        qt.insert(30.0, -40.0, 40.0, -30.0); // 13
        qt.insert(30.0, -20.0, 40.0, -10.0); // 14
        qt.insert(10.0, -40.0, 20.0, -30.0); // 15
        qt.insert(10.0, -20.0, 20.0, -10.0); // 16

        let mut tv = TestVisitor::new();
        qt.traverse(&mut tv);
        tv.assert_counts(32, 16, 5);
        tv.reset();

        qt.remove(16);
        qt.remove(15);
        qt.remove(14);
        qt.remove(13);
        qt.traverse(&mut tv);
        tv.assert_counts(28, 16, 5);
        tv.reset();

        qt.remove(12);
        qt.remove(11);
        qt.remove(10);
        qt.remove(9);
        qt.traverse(&mut tv);
        tv.assert_counts(24, 16, 5);
        tv.reset();

        qt.remove(8);
        qt.remove(7);
        qt.remove(6);
        qt.remove(5);
        qt.traverse(&mut tv);
        tv.assert_counts(20, 16, 5);
        tv.reset();

        qt.remove(4);
        qt.remove(3);
        qt.remove(2);
        qt.remove(1);
        qt.traverse(&mut tv);
        tv.assert_counts(16, 16, 5);
        tv.reset();

        qt.remove(0);
        qt.traverse(&mut tv);
        tv.assert_counts(0, 16, 5);
        tv.reset();

        qt.cleanup();
        qt.traverse(&mut tv);
        tv.assert_counts(0, 4, 1);
        tv.reset();

        qt.cleanup();
        qt.traverse(&mut tv);
        tv.assert_counts(0, 1, 0);
        tv.reset();
    }
}