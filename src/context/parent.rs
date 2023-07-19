use dam_core::{
    log_graph::{get_log, RegistryEvent},
    metric::NODE,
    time::Time,
    ContextView, ParentView, TimeView, TimeViewable,
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
        self.children.push(child);
    }
}

impl<'a> Context for BasicParentContext<'a> {
    fn init(&mut self) {
        self.children.iter_mut().for_each(|child| child.init());
    }

    fn run(&mut self) {
        self.register();
        std::thread::scope(|s| {
            self.children.iter_mut().for_each(|child| {
                let id = child.id();
                let name = child.name();
                get_log(NODE).log(RegistryEvent::WithChild(child.id(), child.name()));
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
        let finish_time = self
            .children
            .iter()
            .map(|child| child.view().tick_lower_bound())
            .max();
        get_log(NODE).log(RegistryEvent::Cleaned(finish_time.unwrap_or(Time::new(0))))
    }
}

impl<'a> TimeViewable for BasicParentContext<'a> {
    fn view(&self) -> TimeView {
        let child_views = self.children.iter().map(|child| child.view()).collect();
        (ParentView { child_views }).into()
    }
}
