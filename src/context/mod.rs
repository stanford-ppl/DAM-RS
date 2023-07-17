use dam_core::{
    identifier::Identifiable, log_graph::get_graph, ContextView, ParentView, TimeView, TimeViewable,
};

pub mod broadcast_context;
pub mod checker_context;
pub mod function_context;
pub mod generator_context;
pub mod parent;

type ParentType<'a> = dyn ParentContext<'a>;
pub trait Context: Send + Sync + TimeViewable + Identifiable {
    fn init(&mut self);
    fn run(&mut self);
    fn cleanup(&mut self);

    fn register(&self) {
        get_graph().register(self.id(), self.name());
    }
}

type ChildType = dyn Context;

#[derive(Default)]
pub struct ChildManager<'a> {
    next_id: usize,
    children: Vec<&'a mut ChildType>,
}

impl<'a> ChildManager<'a> {
    fn new_child_id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id - 1
    }

    fn add_child(&mut self, child: &'a mut ChildType) {
        self.children.push(child);
    }

    fn for_each_child_single_threaded(&mut self, map_f: impl Fn(&mut ChildType) + Sync) {
        self.children.iter_mut().for_each(|child| {
            map_f(*child);
        });
    }

    fn for_each_child_parallel(&mut self, map_f: impl Fn(&mut ChildType) + Sync) {
        std::thread::scope(|scope| {
            self.children.iter_mut().for_each(|child| {
                scope.spawn(|| map_f(*child));
            });
        });
    }

    fn view(&self) -> TimeView {
        let child_views = self.children.iter().map(|child| child.view()).collect();
        (ParentView { child_views }).into()
    }
}

pub trait ParentContext<'a>: Context {
    fn manager_mut(&mut self) -> &mut ChildManager<'a>;
    fn manager(&self) -> &ChildManager<'a>;
    fn new_child_id(&mut self) -> usize {
        self.manager_mut().new_child_id()
    }

    fn add_child(&mut self, child: &'a mut ChildType) {
        let mut handle = get_graph().register_handle(self.id());
        handle.add_child(child.id());
        self.manager_mut().add_child(child);
    }
}

impl<'a, T: ParentContext<'a> + Identifiable> Context for T {
    fn init(&mut self) {
        self.manager_mut().for_each_child_single_threaded(|child| {
            child.init();
        })

        // Check if we're the orphan, if so, dump the graph to our log.
    }

    fn run(&mut self) {
        self.register();
        self.manager_mut().for_each_child_parallel(|child| {
            child.register();
            child.run();
            child.cleanup();
        })
    }

    fn cleanup(&mut self) {
        // if we're an orphan, drop the graph.
        let graph = get_graph();
        if graph.is_orphan(self.id()) {
            graph.drop_subgraph(self.id(), self.view().tick_lower_bound());
        }
    }
}
