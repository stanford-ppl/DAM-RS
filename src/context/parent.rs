use dam_core::{
    identifier::Identifiable, log_graph::get_graph, ContextView, ParentView, TimeView, TimeViewable,
};
use dam_macros::identifiable;

use super::Context;

type ChildType = dyn Context;

#[identifiable]
#[derive(Default)]
pub struct BasicParentContext<'a> {
    children: Vec<&'a mut ChildType>,
}

impl<'a> BasicParentContext<'a> {
    pub fn add_child(&mut self, child: &'a mut ChildType) {
        let mut handle = get_graph().register_handle(self.id());
        handle.add_child(child.id());
        self.children.push(child);
    }
}

impl<'a> Context for BasicParentContext<'a> {
    fn init(&mut self) {
        self.children.iter_mut().for_each(|child| child.init());
    }

    fn run(&mut self) {
        std::thread::scope(|s| {
            self.children.iter_mut().for_each(|child| {
                let id = child.id();
                let name = child.name();
                std::thread::Builder::new()
                    .name(format!("{}({})", child.id(), child.name()))
                    .spawn_scoped(s, || {
                        child.register();
                        child.run();
                        child.cleanup();
                    })
                    .expect(format!("Failed to spawn child {name:?} {id:?}").as_str());
            });
        });
    }

    fn cleanup(&mut self) {
        get_graph().drop_subgraph(self.id(), self.view().tick_lower_bound());
    }
}

impl<'a> TimeViewable for BasicParentContext<'a> {
    fn view(&self) -> TimeView {
        let child_views = self.children.iter().map(|child| child.view()).collect();
        (ParentView { child_views }).into()
    }
}
