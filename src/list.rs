use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct List<T>
    where T: Copy + Clone + Debug
{
    data: Vec<T>,
    elements: i32,
    capacity: i32,
    vacant: Vec<i32>,
}

impl<T> List<T>
    where
        T: Copy + Debug + Default,
{
    pub fn new() -> Self {
        Self::with_capacity(128)
    }

    pub fn with_capacity(capacity: i32) -> Self {
        let mut data = Vec::new();
        data.resize(capacity as usize, T::default());
        Self {
            data,
            capacity,
            elements: 0,
            vacant: Vec::new(),
        }
    }

    pub fn size(&self) -> i32 {
        self.elements
    }

    pub fn get(&self, index: i32) -> &T {
        debug_assert!(index < self.elements);
        &self.data[index as usize]
    }

    pub fn get_mut(&mut self, index: i32) -> &mut T {
        debug_assert!(index < self.elements);
        &mut self.data[index as usize]
    }

    pub fn set(&mut self, index: i32, element: T) {
        debug_assert!(index < self.elements);
        self.data[index as usize] = element;
    }

    pub fn clear(&mut self) {
        self.elements = 0;
        self.vacant.clear();
    }

    pub fn push(&mut self, element: T) -> i32 {
        let new_pos = self.elements + 1;
        if new_pos > self.capacity {
            let new_cap = new_pos * 2;
            self.data.resize(new_cap as usize, T::default());
            self.capacity = new_cap
        }
        let index = self.elements;
        self.elements += 1;
        self.data[index as usize] = element;
        index
    }

    pub fn pop(&mut self) -> T {
        debug_assert!(self.elements > 0);
        self.elements -= 1;
        self.data[self.elements as usize]
    }

    pub fn insert(&mut self, element: T) -> i32 {
        if !self.vacant.is_empty() {
            let index = self.vacant.pop().unwrap();
            self.data[index as usize] = element;
            return index;
        }
        return self.push(element);
    }

    pub fn erase(&mut self, index: i32) {
        self.vacant.push(index);
    }
}