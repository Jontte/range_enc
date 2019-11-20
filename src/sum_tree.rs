use num;
use num::NumCast;
use std::cmp;

fn upper_power_of_two(mut v: u32) -> u32 {
    v -= 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v += 1;
    v
}

pub struct SumTree<T> {
    tree: Vec<T>,
    start_index: usize,
}

impl<T> SumTree<T>
where
    T: num::Num + Copy + num::traits::NumCast + cmp::Ord,
{
    pub fn new(buckets: usize) -> SumTree<T> {
        let start = (upper_power_of_two(buckets as u32) - 1) as usize;
        SumTree {
            tree: vec![NumCast::from(0).unwrap(); (2 * start + 1) as usize],
            start_index: start,
        }
    }

    pub fn increment(&mut self, bucket: u32, amount: T) {
        let mut index = bucket as usize + self.start_index;

        while index != 0 {
            let k = self.tree[index];
            self.tree[index] = k + amount;
            index = (index - 1) >> 1;
        }
        let k = self.tree[0];
        self.tree[0] = k + amount;
    }

    pub fn get(&self, bucket: usize) -> T {
        // assert(index+start_index < m_tree.size());
        self.tree[bucket + self.start_index]
    }

    pub fn get_total(&self) -> T {
        self.tree[0]
    }

    pub fn get_before(&self, bucket: usize) -> T {
        let mut ret: T = NumCast::from(0).unwrap();
        let mut index = bucket + self.start_index;
        while index != 0 {
            if (index & 1) == 0 {
                ret = ret + self.tree[index - 1];
            }
            index = (index - 1) >> 1;
        }
        ret
    }

    pub fn get_index(&self, value: T) -> usize {
        let mut index: usize = 0;
        let mut value = value;

        loop {
            let left = 2 * index + 1;
            let right = 2 * index + 2;

            if left >= self.tree.len() {
                break;
            }

            if value < self.tree[left] {
                index = left;
            } else {
                value = value - self.tree[left];
                index = right;
            }
        }
        index - self.start_index
    }
}

#[test]
fn test_tree() {
    let mut tree = SumTree::<u32>::new(8);
    tree.increment(0, 10);
    tree.increment(1, 10);
    tree.increment(2, 10);
    tree.increment(7, 1);

    assert!(tree.get_before(0) == 0);
    assert!(tree.get_before(0) + tree.get(0) == 10);
    assert!(tree.get_before(1) == 10);
    assert!(tree.get_before(2) == 20);
    assert!(tree.get_before(3) == 30);
    assert!(tree.get_before(7) == 30);

    assert!(tree.get_index(0) == 0);
    assert!(tree.get_index(1) == 0);
    assert!(tree.get_index(9) == 0);
    assert!(tree.get_index(10) == 1);
    assert!(tree.get_index(29) == 2);
    assert!(tree.get_index(30) == 7);
}
