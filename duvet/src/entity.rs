use crate::{
    attribute::{self, Attribute},
    schema::{EntityId, IdSetExt},
};
use core::fmt;
use sled::{transaction::TransactionError, Result, Tree};

pub type Id = EntityId;

pub struct Entities {
    /// Stores all of the created entities
    pub(crate) entities: Tree,
    /// Stores all of the entity attributes
    pub(crate) attributes: Tree,
    /// Stores all of the entities that refer to a particular attribute
    pub(crate) attribute_entities: Tree,
}

impl Entities {
    pub(crate) fn init(&self) {}

    pub fn create(&self) -> Result<EntityId> {
        let res = self.entities.transaction(|entities| {
            let id = entities.generate_id()? as _;
            let id = EntityId::new(id);
            entities.insert(id, &[])?;
            Ok(id)
        });

        match res {
            Ok(id) => Ok(id),
            Err(TransactionError::Abort(())) => unreachable!(),
            Err(TransactionError::Storage(err)) => Err(err),
        }
    }

    pub fn set_attribute<T: attribute::Value>(
        &self,
        id: EntityId,
        attr: Attribute<T>,
        value: T,
    ) -> Result<()> {
        self.attributes
            .insert(attr.prefix_with(id), value.store())?;
        self.attribute_entities.insert(attr.suffix_with(id), &[])?;

        Ok(())
    }

    pub fn get_attribute<T: attribute::Value>(
        &self,
        id: EntityId,
        attr: Attribute<T>,
    ) -> Result<Option<T>> {
        let value = self.attributes.get(attr.prefix_with(id))?;
        let value = value.map(T::load);
        Ok(value)
    }

    pub fn has_attribute<T>(&self, id: EntityId, attr: Attribute<T>) -> Result<bool> {
        self.attributes.contains_key(attr.prefix_with(id))
    }

    pub fn references<T>(&self, attr: Attribute<T>) -> impl Iterator<Item = Result<EntityId>> {
        self.attribute_entities
            .scan_prefix(attr.key())
            .map(|entity| {
                let (k, _) = entity?;
                let k = &k[20..];
                let (k,) = k.keys();
                Ok(k)
            })
    }
}

impl fmt::Debug for Entities {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Entities")
            .field("len", &self.entities.len())
            .finish()
    }
}
