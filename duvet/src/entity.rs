use crate::{
    attribute::{self, Attribute},
    schema::EntityId,
};
use sled::{transaction::TransactionError, Result, Tree};

pub struct Entities {
    /// Stores all of the created entities
    pub(crate) entities: Tree,
    /// Stores all of the entity attributes
    pub(crate) attributes: Tree,
    /// Stores all of the entities that refer to a particular attribute
    pub(crate) attribute_entities: Tree,
}

impl Entities {
    pub fn insert(&self) -> Result<EntityId> {
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
        attr: &Attribute<T>,
        value: T,
    ) -> Result<()> {
        self.attributes
            .insert(attr.prefix_with(id), value.store())?;
        self.attributes.insert(attr.suffix_with(id), &[])?;

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
}
