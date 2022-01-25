use core::mem::size_of;

#[derive(Clone, Copy, Debug, Default)]
pub struct IdSet(u64);

impl IdSet {
    pub fn enable(&mut self, index: u8) {
        self.set(index, true);
    }

    pub fn disable(&mut self, index: u8) {
        self.set(index, false);
    }

    pub fn set(&mut self, index: u8, enabled: bool) {
        debug_assert!((index as usize) < size_of::<Self>() * 8);
        let flag = 1 << index;
        if enabled {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    pub fn get(&self, index: u8) -> bool {
        debug_assert!((index as usize) < size_of::<Self>() * 8);
        let flag = 1 << index;
        self.0 & flag != 0
    }
}

impl core::iter::FromIterator<u8> for IdSet {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut set = IdSet::default();
        for index in iter {
            set.enable(index);
        }
        set
    }
}
