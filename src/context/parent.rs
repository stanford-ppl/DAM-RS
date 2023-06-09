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
