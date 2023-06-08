use super::ParentContext;

#[derive(Default)]
pub struct BasicParentContext<'a> {
    child_manager: super::ChildManager<'a>,
}

impl<'a> ParentContext<'a> for BasicParentContext<'a> {
    fn manager(&mut self) -> &mut super::ChildManager<'a> {
        &mut self.child_manager
    }
}
