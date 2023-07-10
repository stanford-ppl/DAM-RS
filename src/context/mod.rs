use crate::time::Time;

pub use self::view::ContextView;
use self::view::TimeView;

pub mod broadcast_context;
pub mod checker_context;
pub mod function_context;
pub mod generator_context;
pub mod parent;
pub mod view;

type ParentType<'a> = dyn ParentContext<'a>;
pub trait Context: Send + Sync {
    fn init(&mut self);
    fn run(&mut self);
    fn cleanup(&mut self);
    fn view(&self) -> TimeView;
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

type ChildType = dyn Context;

#[derive(Default)]
pub struct ChildManager<'a> {
    next_id: usize,
    children: Vec<&'a mut ChildType>,
}

pub struct ParentView {
    pub child_views: Vec<TimeView>,
}

impl ContextView for ParentView {
    fn wait_until(&self, when: Time) -> Time {
        let individual_signals: Vec<_> = self
            .child_views
            .iter()
            .map(|child| child.wait_until(when))
            .collect();
        individual_signals.into_iter().min().unwrap_or(when)
    }

    fn tick_lower_bound(&self) -> Time {
        let min_time = self
            .child_views
            .iter()
            .map(|child| child.tick_lower_bound())
            .min();
        match min_time {
            Some(time) => time,
            None => Time::infinite(),
        }
    }
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
        self.manager_mut().add_child(child);
    }
}

impl<'a, T: ParentContext<'a>> Context for T {
    fn init(&mut self) {
        self.manager_mut().for_each_child_single_threaded(|child| {
            child.init();
        })
    }

    fn run(&mut self) {
        // self.manager_mut().for_each_child_single_threaded(|child| {
        //     println!("Starting run: {}", child.name());
        // });
        self.manager_mut().for_each_child_parallel(|child| {
            child.run();
            child.cleanup();
        })
    }

    fn cleanup(&mut self) {}

    fn view(&self) -> TimeView {
        self.manager().view()
    }
}
