use super::*;
use crate::{
    intents::{check_move_intent, Intent},
    model::{self, EntityId, Point},
    prelude::*,
    profile,
    storage::Storage,
};
use caolo_api::OperationResult;

/// In: x, y coordinates
/// Out: OperationResult
pub fn move_bot(
    vm: &mut VM<ScriptExecutionData>,
    point: TPointer,
    output: TPointer,
) -> Result<usize, ExecutionError> {
    profile!("mode_bot");

    let point: Point = vm.get_value(point).ok_or_else(|| {
        error!("move_bot called without a point");
        ExecutionError::InvalidArgument
    })?;

    let intent = caolo_api::bots::MoveIntent {
        id: vm.get_aux().entityid().0,
        position: point,
    };
    let userid = Default::default(); // FIXME
    let storage = vm.get_aux().storage();

    let result = {
        let checkresult = check_move_intent(&intent, userid, storage);
        match checkresult {
            OperationResult::Ok => 0,
            _ => vm.set_value_at(output, checkresult),
        }
    };

    vm.get_aux_mut()
        .intents_mut()
        .push(Intent::new_move(EntityId(intent.id), intent.position));

    return Ok(result);
}

pub fn build_bot(id: EntityId, storage: &Storage) -> Option<caolo_api::bots::Bot> {
    profile!("build_bot");

    let bot = storage.entity_table::<model::Bot>().get_by_id(&id);
    if bot.is_none() {
        debug!(
            "Bot {:?} could not be built because it has no bot component",
            id
        );
        return None;
    }

    let pos = storage
        .entity_table::<model::PositionComponent>()
        .get_by_id(&id)
        .or_else(|| {
            debug!("Bot {:?} could not be built because it has no position", id);
            None
        })?;

    let carry = storage
        .entity_table::<model::CarryComponent>()
        .get_by_id(&id)
        .unwrap_or_else(|| &model::CarryComponent {
            carry: 0,
            carry_max: 0,
        });

    let owner_id = storage.entity_table::<model::OwnedEntity>().get_by_id(&id);

    Some(caolo_api::bots::Bot {
        id: id.0,
        owner_id: owner_id.map(|id| id.owner_id.0),
        position: pos.0,
        carry: carry.carry,
        carry_max: carry.carry_max,
    })
}
