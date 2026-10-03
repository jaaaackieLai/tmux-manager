use super::{PromptSlot, PromptStore, ui::PromptView};
use crate::Result;
use uuid::Uuid;
impl PromptView {
    pub fn save(&mut self, store: &PromptStore, slot: PromptSlot) -> Result<()> {
        let mut next = self.snapshot.document.clone();
        if let Some(index) = next.slots.iter().position(|s| s.id == slot.id) {
            next.slots[index] = slot.clone();
        } else {
            next.slots.push(slot.clone());
        }
        self.snapshot = store.commit(&self.snapshot.revision, &next)?;
        self.selected = Some(slot.id);
        self.editor = None;
        self.status = "已儲存".into();
        Ok(())
    }
    pub fn delete(&mut self, store: &PromptStore, id: Uuid) -> Result<()> {
        let mut next = self.snapshot.document.clone();
        next.slots.retain(|s| s.id != id);
        self.snapshot = store.commit(&self.snapshot.revision, &next)?;
        self.navigate(0);
        self.status = "已刪除".into();
        Ok(())
    }
    pub fn move_slot(&mut self, store: &PromptStore, id: Uuid, delta: i32) -> Result<()> {
        let mut next = self.snapshot.document.clone();
        next.slots.sort_by_key(|s| (s.order, s.id));
        if let Some(index) = next.slots.iter().position(|s| s.id == id) {
            let destination = (index as i64 + i64::from(delta))
                .clamp(0, next.slots.len().saturating_sub(1) as i64)
                as usize;
            next.slots.swap(index, destination);
            for (order, slot) in next.slots.iter_mut().enumerate() {
                slot.order = order as u32;
            }
            self.snapshot = store.commit(&self.snapshot.revision, &next)?;
        }
        Ok(())
    }
}
