use std::fmt;
use std::marker::PhantomData;

// Make a strong Id type for a given type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id<T> {
    value: u32,
    _phantom: PhantomData<T>,
}

impl<T> Id<T> {
    fn new(value: u32) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }

    pub fn value(&self) -> u32 {
        self.value
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[test]
fn test_id() {
    // Say we have some data structure and want to save i
    pub struct MyData {
        member: String,
    }
    type MyId = Id<MyData>;
    let id = MyId::new(42);

    assert_eq!(id.value(), 42);
    assert_eq!(format!("{}", id), "42");
}
