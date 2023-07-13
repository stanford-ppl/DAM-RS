use dam_core::{identifier::Identifier, TimeView, TimeViewable};
use dam_macros::identifiable;

use super::{ChildManager, ParentContext};

#[identifiable]
pub struct BasicParentContext<'a> {
    child_manager: super::ChildManager<'a>,
}

impl<'a> ParentContext<'a> for BasicParentContext<'a> {
    fn manager_mut(&mut self) -> &mut super::ChildManager<'a> {
        &mut self.child_manager
    }

    fn manager(&self) -> &super::ChildManager<'a> {
        &self.child_manager
    }
}

impl<'a> TimeViewable for BasicParentContext<'a> {
    fn view(&self) -> TimeView {
        self.child_manager.view()
    }
}

impl<'a> BasicParentContext<'a> {
    pub fn new() -> Self {
        Self {
            child_manager: ChildManager::default(),
            identifier: Identifier::new(),
        }
    }
}

impl<'a> Default for BasicParentContext<'a> {
    fn default() -> Self {
        Self::new()
    }
}
