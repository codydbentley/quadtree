mod quadtree;
mod list;

pub trait QuadtreeVisitor {
    fn entity(&mut self, entity_id: i32, x: i32, y: i32, width: i32, height: i32);
    fn leaf(&mut self, depth: u8, x: i32, y: i32, width: i32, height: i32);
    fn branch(&mut self, depth: u8, x: i32, y: i32, width: i32, height: i32);
}

pub use quadtree::*;
pub use list::*;



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        todo!()
    }
}
