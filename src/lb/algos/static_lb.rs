use crate::lb::LoadBalancer;

pub struct StaticLB {
    index: u32,
}

impl StaticLB {
    pub fn new() -> Self {
        StaticLB {
            index: 0,
        }
    }
    pub fn update(&mut self, index: u32) {
        self.index = index;
    }
}

impl LoadBalancer for StaticLB {
    fn get_server(&self) -> Option<u32> {
        return Some(self.index);
    }
}