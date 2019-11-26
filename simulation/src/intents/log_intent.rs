use super::*;
use crate::model;
use crate::EntityId;

#[derive(Debug, Clone)]
pub struct LogIntent {
    pub entity: EntityId,
    pub payload: String,
    pub time: u64,
}

impl LogIntent {
    pub fn execute(&self, storage: &mut Storage) -> IntentResult {
        let id = (self.entity, self.time);
        let table = storage.log_table_mut::<model::LogEntry>();
        let entry = match table.get_by_id(&id).cloned() {
            Some(mut entry) => {
                entry.payload.push(self.payload.clone());
                entry
            }
            None => model::LogEntry {
                payload: vec![self.payload.clone()],
            },
        };
        table.insert(id, entry);

        Ok(())
    }
}
