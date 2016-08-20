use sum_tree::SumTree;

pub trait Dictionary {
    fn new(n_symbols: u32) -> Self;
    fn total_frequency(&self) -> u32;
    fn symbol_frequency(&self, sym: u32) -> u32;
    fn symbol_lookup(&self, val: u32) -> u32;
    fn frequency_up_to_symbol(&self, sym: u32) -> u32;
    fn increment(&mut self, sym: u32);
}

impl Dictionary for SumTree<u32> {
    fn new(n_symbols: u32) -> SumTree<u32> {
        let mut tree = SumTree::new(n_symbols as usize);
        for i in 0..n_symbols {
            tree.increment(i as u32, 1);
        }
        tree
    }
    fn total_frequency(&self) -> u32 {
        self.get_total() as u32
    }
    fn symbol_frequency(&self, sym: u32) -> u32 {
        self.get(sym as usize) as u32
    }
    fn symbol_lookup(&self, v: u32) -> u32 {
        self.get_index(v) as u32
    }
    fn frequency_up_to_symbol(&self, sym: u32) -> u32 {
        self.get_before(sym as usize) as u32
    }
    fn increment(&mut self, sym: u32) {

        let amount = 1;
        const SYMBOL_MAX_FREQ: u32 = 1 << 16;

        if self.symbol_frequency(sym) < SYMBOL_MAX_FREQ {
            SumTree::increment(self, sym, amount);
        }
    }
}
