use std::alloc::{alloc, dealloc, Layout};
use crate::value::Value;
use crate::objects::LoxString;
use std::ptr::{null_mut, read, write};

pub struct Entry {
    key: Option<LoxString>,
    value: Value,
}

#[derive(Debug)]
pub struct Table {
    count: usize,
    capacity: usize,
    entries: *mut Entry,
}

impl Table {
    const MAX_LOAD: f32 = 0.75;

    pub fn new() -> Self {
        Table {
            count: 0,
            capacity: 0,
            entries: null_mut(),
        }
    }

    pub fn get(&self, key: &LoxString) -> Option<Value> {
        if self.count == 0 {
            return None;
        }
            
        unsafe {
            let entry = Self::find_entry(self.entries, key, self.capacity);
            if (*entry).key.is_none() {
                None
            } else {
                Some((*entry).value.clone())
            }
        }
    }

    pub fn set(&mut self, key: LoxString, value: Value) -> bool {
        unsafe {
            if self.count + 1 > (self.capacity as f32 * Self::MAX_LOAD) as usize {
                let new_capacity = if self.capacity == 0 { 8 } else { self.capacity * 2 };
                self.adjust_capacity(new_capacity);
            }

            let entry = Self::find_entry(self.entries, &key, self.capacity);
            let is_new = (*entry).key.is_none();

            if is_new && (*entry).value == Value::Nil{
                self.count += 1;
            }

            (*entry).key = Some(key);
            (*entry).value = value;

            is_new
        }
    }

    pub fn delete(&mut self, key: &LoxString) -> bool {
        if self.count == 0 {
            return false;
        }

        unsafe {
            let entry = Self::find_entry(self.entries, key, self.capacity);
            if (*entry).key.is_none() {
                return false;
            }

            (*entry).key = None;
            (*entry).value = Value::Bool(true);
            true
        }
    }

    pub fn add_table(&mut self, table: &Table) {
        unsafe {
            for i in 0..table.capacity {
                let entry = table.entries.add(i);

                if let Some(ref k) = (*entry).key {
                    self.set(k.clone(), (*entry).value.clone());
                }
            }
        }
    }

    pub fn find_string(&self, s: &str, hash: usize) -> Option<&LoxString> {
        if self.count == 0 {
            return None;
        }

        unsafe {
            let mut index = hash & (self.capacity - 1);

            loop {
                let entry = self.entries.add(index);

                match (*entry).key {
                    Some(ref k) => {
                        if *s == k.value {
                            return Some(k);
                        }
                    },
                    None => {
                        if let Value::Nil = (*entry).value {
                            return None;
                        }
                    }
                }

                index = (index + 1) & (self.capacity - 1);
            }
        }
    }

    unsafe fn find_entry(entries: *mut Entry, key: &LoxString, capacity: usize) -> *mut Entry {
        debug_assert!(capacity.is_power_of_two() && capacity > 0);

        let mut index = key.hash & (capacity - 1);

        loop {
            let entry = entries.add(index);

            match (*entry).key {
                Some(ref k) => {
                    if *k == *key {
                        return entry;
                    }
                },
                None => {
                    match (*entry).value {
                        Value::Nil | Value::Bool(true) => return entry,
                        _ => continue
                    }
                }
            }

            index = (index + 1) & (capacity - 1);
        }
    }

    unsafe fn adjust_capacity(&mut self, new_capacity: usize) {
        let entries = alloc(Layout::array::<Entry>(new_capacity).unwrap()) as *mut Entry;

        for i in 0..new_capacity {
            let entry = entries.add(i);
            write(entry, Entry { key: None, value: Value::Nil });
        }

        self.count = 0;
        for i in 0..self.capacity {
            let entry = self.entries.add(i);

            match (*entry).key {
                Some(ref k) => {
                    let dest = Self::find_entry(entries, k, new_capacity);
                    (*dest).key = (*entry).key.take();
                    (*dest).value = read(&(*entry).value);

                    self.count += 1;
                },
                None => continue
            }
        }

        if self.capacity > 0 {
            dealloc (
                self.entries.cast(),
                Layout::array::<Entry>(self.capacity).unwrap()
            );
        }

        self.entries = entries;
        self.capacity = new_capacity;
    }

    pub fn iter(&self) -> IterTable {
        IterTable {
            current: self.entries,
            end: unsafe { self.entries.add(self.capacity) }
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        unsafe {
            if !self.entries.is_null() {  
                dealloc(
                    self.entries.cast(),
                    Layout::array::<Entry>(self.capacity).unwrap()
                );
            }
        }
    }
}

pub struct IterTable {
    current: *mut Entry,
    end: *const Entry,
}

impl Iterator for IterTable {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        while !std::ptr::eq(self.current, self.end) {
            unsafe {
                let entry = self.current;
                self.current = self.current.add(1);
                if (*entry).key.is_none() {
                    continue;
                }
                return Some(read(entry));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_n(table: &mut Table, n: usize) {
        for i in 0..n {
            table.set(LoxString::new(&format!("a{}", i)), Value::Bool(true));
        }
    }

    #[test]
    fn test_new() {
        let table = Table::new();
        assert_eq!(table.count, 0);
        assert_eq!(table.capacity, 0);
        assert!(table.entries.is_null());
    }

    #[test]
    fn test_set_once() {
        let mut table = Table::new();
        table.set(LoxString::new("a"), Value::Bool(true));

        assert_eq!(table.count, 1);
        assert_eq!(table.capacity, 8);
        assert!(!table.entries.is_null());
    }

    #[test]
    fn test_set_twice() {
        let mut table = Table::new();
        table.set(LoxString::new("a"), Value::Bool(true));
        
        assert_eq!(table.get(&LoxString::new("a")), Some(Value::Bool(true)));

        table.set(LoxString::new("a"), Value::Number(1.0));
        assert_eq!(table.get(&LoxString::new("a")), Some(Value::Number(1.0)));
    }

    #[test]
    fn test_get() {
        let mut table = Table::new();
        table.set(LoxString::new("a"), Value::Bool(true));
        table.set(LoxString::new("b"), Value::Number(23.0));

        assert_eq!(table.get(&LoxString::new("a")), Some(Value::Bool(true)));
        assert_eq!(table.get(&LoxString::new("b")), Some(Value::Number(23.0)));
        assert_eq!(table.get(&LoxString::new("c")), None);
    }

    #[test]
    fn test_grow() {
        let mut table = Table::new();
        load_n(&mut table, 9);
        assert_eq!(table.count, 9);
        assert_eq!(table.capacity, 16);

        let mut table2 = Table::new();
        load_n(&mut table2, 17);
        assert_eq!(table2.count, 17);
        assert_eq!(table2.capacity, 32);

        let mut table3 = Table::new();
        load_n(&mut table3, 33);
        assert_eq!(table3.count, 33);
        assert_eq!(table3.capacity, 64);

        let mut table4 = Table::new();
        load_n(&mut table4, 65);
        assert_eq!(table4.count, 65);
        assert_eq!(table4.capacity, 128);
    }

    #[test]
    fn test_drop() {
        for i in 0..50 {
            let mut table = Table::new();
            table.set(LoxString::from_string(&format!("key {}", i)), Value::Bool(true));
        }
    }

    #[test]
    fn test_delete() {
        let mut table = Table::new();

        table.set(LoxString::new("a"), Value::Bool(true));
        table.set(LoxString::new("b"), Value::Bool(true));
        table.set(LoxString::new("c"), Value::Bool(true));

        assert_eq!(table.get(&LoxString::new("a")), Some(Value::Bool(true)));

        table.delete(&LoxString::new("a"));
        assert_eq!(table.get(&LoxString::new("a")), None);

        assert_eq!(table.get(&LoxString::new("b")), Some(Value::Bool(true)));
        
        table.delete(&LoxString::new("b"));
        assert_eq!(table.get(&LoxString::new("b")), None);

        assert_eq!(table.get(&LoxString::new("c")), Some(Value::Bool(true)));
        
        table.delete(&LoxString::new("c"));
        assert_eq!(table.get(&LoxString::new("c")), None);
    }

    #[test]
    fn test_add_table() {
        let mut table = Table::new();
        let mut table2 = Table::new();

        table.set(LoxString::new("a"), Value::Bool(true));
        table.set(LoxString::new("b"), Value::Bool(true));
        table.set(LoxString::new("c"), Value::Bool(true));

        table2.set(LoxString::new("d"), Value::Bool(true));
        table2.set(LoxString::new("e"), Value::Bool(true));
        table2.set(LoxString::new("f"), Value::Bool(true));

        table.add_table(&table2);

        assert_eq!(table.get(&LoxString::new("a")), Some(Value::Bool(true)));
        assert_eq!(table.get(&LoxString::new("b")), Some(Value::Bool(true)));
        assert_eq!(table.get(&LoxString::new("c")), Some(Value::Bool(true)));
        assert_eq!(table.get(&LoxString::new("d")), Some(Value::Bool(true)));
        assert_eq!(table.get(&LoxString::new("e")), Some(Value::Bool(true)));
        assert_eq!(table.get(&LoxString::new("f")), Some(Value::Bool(true)));
    }
}