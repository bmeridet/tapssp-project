
use std::fmt::Display;

pub enum ObjectType {
    LoxString,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoxString {
    pub value: String,
    pub hash: usize,
}

impl Display for LoxString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl LoxString {
    pub fn new(value: &str) -> Self {
        let hash = LoxString::hash(&value);
        LoxString { 
            value: value.to_string(), 
            hash,
        }
    }

    pub fn from_string(s: &str) -> Self {
        LoxString::new(s)
    }

    fn hash(s: &str) -> usize {
        let mut hash = 2166136261usize;
        for c in s.chars() {
            hash ^= c as usize;
            hash = hash.wrapping_mul(16777619);
        }
        hash
    }
}