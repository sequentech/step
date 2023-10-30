use serde::{Serializer};

pub trait HasId {
    fn id(&self) -> &str;
}

pub fn to_id<T, S>(entity: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: HasId,
    S: Serializer,
{
    serializer.serialize_str(&entity.id())
}
