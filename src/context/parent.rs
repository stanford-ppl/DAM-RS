use dam_core::{TimeView, TimeViewable};

use super::ParentContext;

#[derive(Default)]
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
