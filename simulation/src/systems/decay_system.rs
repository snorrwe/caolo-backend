use super::System;
use crate::model::{self, EntityId};
use crate::storage::views::{DeleteEntityView, UnsafeView};
use crate::tables::JoinIterator;

pub struct DecaySystem;

impl<'a> System<'a> for DecaySystem {
    type Mut = (
        UnsafeView<EntityId, model::HpComponent>,
        UnsafeView<EntityId, model::DecayComponent>,
        DeleteEntityView,
    );
    type Const = ();

    fn update(&mut self, (mut hps, mut decays, mut delete): Self::Mut, _: Self::Const) {
        debug!("update decay system called");

        let iter =
            unsafe { JoinIterator::new(decays.as_mut().iter_mut(), hps.as_mut().iter_mut()) };
        iter.for_each(|(id, (d, hp))| {
            if d.t > 0 {
                d.t -= 1;
            }
            if d.t == 0 {
                hp.hp -= hp.hp.min(d.hp_amount);
            }
            if hp.hp == 0 {
                debug!("Entity {:?} has died, deleting", id);
                unsafe {
                    delete.delete_entity(id);
                }
            }
        });

        debug!("update decay system done");
    }
}