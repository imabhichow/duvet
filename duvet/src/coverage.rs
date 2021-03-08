use crate::{
    attribute::Attribute,
    db::Db,
    schema::{EntityId, FileId},
};
use anyhow::Result;
use core::ops::Range;

#[cfg(feature = "llvm-coverage")]
pub mod llvm;

pub fn notify<S, T, H: Handler>(
    db: &Db,
    subject: Attribute<S>,
    target: Attribute<T>,
    handler: &mut H,
) -> Result<()> {
    let entities = db.entities();

    let mut found = vec![];

    for subject in entities.references(subject) {
        let subject = subject?;

        let mut all_ok = true;
        let mut found_any = false;

        for reference in db.regions().references(subject) {
            let reference = reference?;

            for potential_target in reference.entities().iter().copied() {
                if potential_target == subject {
                    continue;
                }
                if entities.has_attribute(potential_target, target)? {
                    found.push(potential_target);
                    found_any = true;
                }
            }

            if found.is_empty() {
                all_ok = false;
                handler.on_region_failure(reference.file, reference.range(), subject)?;
            } else {
                handler.on_region_success(reference.file, reference.range(), subject, &found)?;
                found.clear();
            }
        }

        if all_ok && found_any {
            handler.on_entity_success(subject)?;
        } else {
            handler.on_entity_failure(subject)?;
        }
    }

    Ok(())
}

#[allow(unused_variables)]
pub trait Handler {
    fn on_region_success(
        &mut self,
        file: FileId,
        bytes: Range<u32>,
        entity: EntityId,
        references: &[EntityId],
    ) -> Result<()> {
        Ok(())
    }

    fn on_region_failure(
        &mut self,
        file: FileId,
        bytes: Range<u32>,
        entity: EntityId,
    ) -> Result<()> {
        Ok(())
    }

    fn on_entity_success(&mut self, entity: EntityId) -> Result<()> {
        Ok(())
    }

    fn on_entity_failure(&mut self, entity: EntityId) -> Result<()> {
        Ok(())
    }
}
