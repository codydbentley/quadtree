use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct List<T>
where
    T: Copy + Clone + Debug,
{
    data: Vec<T>,
    cursor: usize,
    capacity: usize,
    vacant: Vec<usize>,
}

impl<T> List<T>
where
    T: Copy + Debug + Default,
{
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::new();
        data.resize(capacity, T::default());
        Self {
            data,
            capacity,
            cursor: 0,
            vacant: Vec::new(),
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn get(&self, index: usize) -> &T {
        debug_assert!(index < self.cursor);
        &self.data[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut T {
        debug_assert!(index < self.cursor);
        &mut self.data[index]
    }

    pub fn set(&mut self, index: usize, element: T) {
        debug_assert!(index < self.cursor);
        self.data[index] = element;
    }

    pub fn clear(&mut self) {
        self.cursor = 0;
        self.vacant.clear();
    }

    pub fn push(&mut self, element: T) -> usize {
        let new_pos = self.cursor + 1;
        if new_pos > self.capacity {
            let new_cap = self.cursor * 2;
            self.data.resize(new_cap, T::default());
            self.capacity = new_cap
        }
        let index = self.cursor;
        self.cursor += 1;
        self.data[index] = element;
        index
    }

    pub fn pop(&mut self) -> T {
        debug_assert!(self.cursor > 0);
        self.cursor -= 1;
        self.data[self.cursor]
    }

    pub fn insert(&mut self, element: T) -> usize {
        match self.vacant.pop() {
            Some(vacant) => {
                self.data[vacant] = element;
                vacant
            }
            None => self.push(element),
        }
    }

    pub fn erase(&mut self, index: usize) {
        self.vacant.push(index);
    }
}

impl<T> Default for List<T>
where
    T: Copy + Debug + Default,
{
    fn default() -> Self {
        List::new(128)
    }
}

pub trait FreeVec {
    fn something();
}

impl<T> FreeVec for Vec<T> {
    fn something() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor() {
        let mut list = List::<u8>::default();
        assert_eq!(list.cursor(), 0);

        list.push(1);
        assert_eq!(list.cursor(), 1);

        list.insert(2);
        assert_eq!(list.cursor(), 2);

        list.pop();
        assert_eq!(list.cursor(), 1);

        list.push(3);
        assert_eq!(list.cursor(), 2);

        // This will create a vacancy, not move the cursor
        list.erase(0);
        assert_eq!(list.cursor(), 2);

        // This will fill the vacant slot, no cursor movement
        list.insert(4);
        assert_eq!(list.cursor(), 2);

        list.clear();
        assert_eq!(list.cursor(), 0);
    }

    #[test]
    fn capacity() {
        let mut list = List::<u8>::new(2);
        assert_eq!(list.capacity, 2);

        list.push(1);
        list.push(2);
        assert_eq!(list.capacity, 2);

        list.erase(0);
        assert_eq!(list.capacity, 2);

        list.insert(3);
        assert_eq!(list.capacity, 2);

        list.insert(4);
        assert_eq!(list.capacity, 4);

        list.insert(5);
        assert_eq!(list.capacity, 4);

        list.pop();
        list.pop();
        list.pop();
        list.pop();
        assert_eq!(list.cursor, 0);
        assert_eq!(list.capacity, 4);
    }

    #[test]
    fn vacant() {
        let mut list = List::<u8>::default();
        assert!(list.vacant.is_empty());

        for i in 1..=100 {
            list.push(i);
        }

        for i in 2..=9 {
            let x = i * 10;
            list.erase(x);
            let y = list.insert(i as u8);
            assert_eq!(x, y);
        }
    }
}
