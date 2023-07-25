use std::collections::{HashMap, HashSet};

use dam_core::identifier::{Identifiable, Identifier};
use dam_core::log_graph::with_log_scope;
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
        with_log_scope(self.id(), self.name(), || {
            let parent_id = self.id();
            let parent_name = self.name();
            std::thread::scope(|s| {
                self.children.iter_mut().for_each(|child| {
                    let id = child.id();
                    let name = child.name();
                    get_log(NODE).log(RegistryEvent::WithChild(
                        parent_id,
                        parent_name.clone(),
                        child.id(),
                        child.name(),
                    ));
                    std::thread::Builder::new()
                        .name(format!("{}({})", child.id(), child.name()))
                        .spawn_scoped(s, || {
                            with_log_scope(child.id(), child.name(), || {
                                child.run();
                                child.cleanup();
                            });
                        })
                        .expect(format!("Failed to spawn child {name:?} {id:?}").as_str());
                });
            });
        });
    }

    fn cleanup(&mut self) {
        let finish_time = self
            .children
            .iter()
            .map(|child| child.view().tick_lower_bound())
            .max();
        with_log_scope(self.id(), self.name(), || {
            get_log(NODE).log(RegistryEvent::Cleaned(finish_time.unwrap_or(Time::new(0))));
        });
    }

    fn child_ids(&self) -> HashMap<Identifier, HashSet<Identifier>> {
        let mut base_map = HashMap::from([(
            self.id(),
            HashSet::from_iter(self.children.iter().map(|child| child.id())),
        )]);
        let sub_maps = self.children.iter().map(|child| child.child_ids());
        sub_maps.for_each(|sub| base_map.extend(sub));
        base_map
    }
}

impl<'a> TimeViewable for BasicParentContext<'a> {
    fn view(&self) -> TimeView {
        let child_views = self.children.iter().map(|child| child.view()).collect();
        (ParentView { child_views }).into()
    }
}
