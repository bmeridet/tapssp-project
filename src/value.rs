use std::{fmt, ops::Deref};
use std::rc::Rc;
use std::fmt::Display;
use crate::objects::{LoxString};

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(LoxString),
    Nil,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{:?}", n),
            Value::Bool(b) => write!(f, "{:?}", b),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Nil => write!(f, "nil"),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Nil => write!(f, "nil"),
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            _ => false
        }
    }

    pub fn is_falsey(&self) -> bool {
        !self.is_truthy()
    }

    pub fn as_number(&self) -> Option<f64> {
        if let Value::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }
}