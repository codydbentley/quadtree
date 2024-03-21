use crate::list::List;
use crate::QuadtreeVisitor;

#[derive(Copy, Clone, Debug)]
struct EntityNode {
    next: i32,
    entity: i32,
}

impl Default for EntityNode {
    fn default() -> Self {
        Self {
            next: -1,
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
    first_child: i32,
    num_children: i32,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            first_child: -1,
            num_children: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct NodeData {
    idx: i32,
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

pub struct Quadtree {
    root: NodeData,
    max_entities: i32,
    max_depth: u8,
    entity_nodes: List<EntityNode>,
    entities: List<Entity>,
    nodes: List<Node>,
}

impl Quadtree {
    const FC_EMPTY_LEAF: i32 = -1;
    const NUM_CHILD_BRANCH_FLAG: i32 = -1;

    pub fn new(x: i32, y: i32, width: i32, height: i32, max_entities_per_region: i32) -> Self {
        let mut nodes = List::new();
        let root_idx = nodes.insert(Node::default());
        Self {
            root: NodeData {
                idx: root_idx,
                depth: 0,
                x,
                y,
                hx: width / 2,
                hy: height / 2,
            },
            max_entities: max_entities_per_region,
            max_depth: Self::calc_max_depth(width, height),
            nodes,
            entity_nodes: List::new(),
            entities: List::new(),
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

    pub fn insert(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> i32{
        let new_entity_idx = self.entities.insert(Entity {
            left: x1 as i32,
            top: y1 as i32,
            right: x2 as i32,
            bottom: y2 as i32,
        });
        self.node_insert(self.root, new_entity_idx);
        new_entity_idx
    }

    pub fn remove(&mut self, entity_idx: i32) {
        // Find the leaves.
        let entity = self.entities.get(entity_idx);
        let leaves = self.find_leaves(self.root, entity.left, entity.top, entity.right, entity.bottom);

        // For each leaf node, remove the element node.
        for i in 0..leaves.size() {
            let nd_data_idx = leaves.get(i).idx;

            // Walk the list until we find the element node.
            let mut node_idx = self.nodes.get(nd_data_idx).first_child;
            let mut prev_index = Self::FC_EMPTY_LEAF;
            while node_idx != Self::FC_EMPTY_LEAF && self.entity_nodes.get(node_idx).entity != entity_idx {
                prev_index = node_idx;
                node_idx = self.entity_nodes.get(node_idx).next;
            }

            if node_idx != Self::FC_EMPTY_LEAF {
                // Remove the element node.
                let next_index = self.entity_nodes.get(node_idx).next;
                if prev_index == Self::FC_EMPTY_LEAF {
                    self.nodes.get_mut(nd_data_idx).first_child = next_index;
                } else {
                    self.entity_nodes.get_mut(prev_index).next = next_index;
                }
                self.entity_nodes.erase(node_idx);

                // Decrement the leaf element count.
                self.nodes.get_mut(nd_data_idx).first_child -= 1;
            }
        }

        // Remove the element.
        self.entities.erase(entity_idx);
    }

    pub fn cleanup(&mut self) {
        let mut to_process = List::<i32>::new();

        // Only process the root if it's not a leaf.
        if self.nodes.get(self.root.idx).num_children == Self::NUM_CHILD_BRANCH_FLAG {
            // Push the root index to the stack.
            to_process.push(self.root.idx);
        }

        while to_process.size() > 0 {
            // Pop a node from the stack.
            let node_idx = to_process.pop();
            let node = *self.nodes.get(node_idx);
            let mut num_empty_leaves = 0;

            // Loop through the children.
            for i in 0..4 {
                let child_idx = node.first_child + i;
                // Increment empty leaf count if the child is an empty
                // leaf. Otherwise, if the child is a branch, add it to
                // the stack to be processed in the next iteration.
                let child_node = *self.nodes.get(child_idx);
                if child_node.num_children == 0 {
                    num_empty_leaves += 1;
                } else if child_node.num_children == Self::NUM_CHILD_BRANCH_FLAG {
                    // Push the child index to the stack.
                    to_process.push(child_idx);
                }
            }

            // If all the children were empty leaves, remove them and
            // make this node the new empty leaf.
            if num_empty_leaves == 4 {
                // Remove all 4 children in reverse order so that they
                // can be reclaimed on subsequent insertions in proper
                // order.
                self.nodes.erase(node.first_child + 3);
                self.nodes.erase(node.first_child + 2);
                self.nodes.erase(node.first_child + 1);
                self.nodes.erase(node.first_child + 0);

                // Make this node the new empty leaf.
                self.nodes.get_mut(node_idx).first_child = Self::FC_EMPTY_LEAF;
                self.nodes.get_mut(node_idx).num_children = 0;
            }
        }
    }

    pub fn query(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> Vec<i32> {
        self.query_omit(x1, y1, x2, y2, -1)
    }

    pub fn query_omit(&self, x1: f32, y1: f32, x2: f32, y2: f32, omit_entity_id: i32) -> Vec<i32> {
        let mut out = Vec::<i32>::new();

        // Find the leaves that intersect the specified query rectangle.
        let q_left = x1 as i32;
        let q_top = y1 as i32;
        let q_right = x2 as i32;
        let q_bottom = y2 as i32;
        let leaves = self.find_leaves(self.root, q_left, q_top, q_right, q_bottom);

        let mut seen = Vec::<bool>::new();
        seen.resize(self.entities.size() as usize, false);

        // For each leaf node, look for elements that intersect.
        for i in 0..leaves.size() {
            let nd_data_idx = leaves.get(i).idx;

            // Walk the list and add elements that intersect.
            let mut next_enode_idx = self.nodes.get(nd_data_idx).first_child;
            while next_enode_idx != Self::FC_EMPTY_LEAF {
                let entity_node = self.entity_nodes.get(next_enode_idx);
                let entity = self.entities.get(entity_node.entity);
                if !seen[entity_node.entity as usize]
                    && entity_node.entity != omit_entity_id
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
                    seen[entity_node.entity as usize] = true;
                }
                next_enode_idx = entity_node.next;
            }
        }
        out
    }

    pub fn traverse<V>(&self, visitor: &mut V)
        where
            V: QuadtreeVisitor,
    {
        let mut to_process = List::<NodeData>::new();
        to_process.push(self.root);

        while to_process.size() > 0 {
            let nd_data = to_process.pop();

            if self.nodes.get(nd_data.idx).num_children == Self::NUM_CHILD_BRANCH_FLAG {
                // Push the children of the branch to the stack.
                let fc = self.nodes.get(nd_data.idx).first_child;
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
                    nd_data.x,
                    nd_data.y,
                    nd_data.hx << 1,
                    nd_data.hy << 1,
                );
            } else {
                visitor.leaf(
                    nd_data.depth,
                    nd_data.x,
                    nd_data.y,
                    nd_data.hx << 1,
                    nd_data.hy << 1,
                );
                let mut node_idx = self.nodes.get(nd_data.idx).first_child;
                while node_idx != Self::FC_EMPTY_LEAF {
                    let entity_node= self.entity_nodes.get(node_idx);
                    let entity = self.entities.get(entity_node.entity);
                    let w = entity.right - entity.left;
                    let h = entity.bottom - entity.top;
                    let x = entity.left + (w>>1);
                    let y = entity.top + (h>>1);
                    visitor.entity(entity_node.entity, x, y, w, h);
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
        let mut leaves = List::<NodeData>::new();
        let mut to_process = List::<NodeData>::new();
        to_process.push(start_node);

        while to_process.size() > 0 {
            let nd_data = to_process.pop();
            if self.nodes.get(nd_data.idx).num_children != Self::NUM_CHILD_BRANCH_FLAG {
                leaves.push(nd_data);
            } else {
                let fc = self.nodes.get(nd_data.idx).first_child;
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

    fn node_insert(&mut self, start_node: NodeData, entity_idx: i32) {
        let entity = self.entities.get(entity_idx);
        let leaves = self.find_leaves(
            start_node,
            entity.left,
            entity.top,
            entity.right,
            entity.bottom,
        );

        for i in 0..leaves.size() {
            let nd_data = leaves.get(i);
            self.leaf_insert(*nd_data, entity_idx);
        }
    }

    fn leaf_insert(&mut self, node_data: NodeData, entity_idx: i32) {
        let first_child = self.nodes.get(node_data.idx).first_child;
        let e_node = self.entity_nodes.push(EntityNode {
            entity: entity_idx,
            next: first_child,
        });
        self.nodes.get_mut(node_data.idx).first_child = e_node;

        // If the leaf is full, split it.
        if self.nodes.get(node_data.idx).num_children == self.max_entities && node_data.depth < self.max_depth {
            // Transfer elements from the leaf node to a list of elements.
            let mut entities = List::<i32>::new();
            while self.nodes.get(node_data.idx).first_child != Self::FC_EMPTY_LEAF {
                let index = self.nodes.get(node_data.idx).first_child;
                let e_node = *self.entity_nodes.get(index);

                // Pop off the element node from the leaf and remove it from the qt.
                self.nodes.get_mut(node_data.idx).first_child = e_node.next;
                self.entity_nodes.erase(index);

                // Insert element to the list.
                entities.push(e_node.entity);
            }

            // Initialize 4 child nodes.
            let fc = self.nodes.insert(Node::default());
            self.nodes.insert(Node::default());
            self.nodes.insert(Node::default());
            self.nodes.insert(Node::default());

            self.nodes.get_mut(node_data.idx).first_child = fc;
            self.nodes.get_mut(node_data.idx).num_children = Self::NUM_CHILD_BRANCH_FLAG;

            // Transfer the elements in the former leaf node to its new children.
            for i in 0..entities.size() {
                self.node_insert(node_data, *entities.get(i));
            }
        } else {
            // Increment the leaf element count.
            self.nodes.get_mut(node_data.idx).num_children += 1;
        }
    }
}