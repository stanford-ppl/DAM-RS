use crate::time::Time;

pub use self::view::ContextView;

pub mod function_context;
pub mod parent;
pub mod view;

type ParentType<'a> = &'a dyn ParentContext<'a>;
pub trait Context<'a>: Send + Sync {
    fn init(&mut self);
    fn run(&mut self);
    fn cleanup(&mut self);
    fn view(&self) -> Box<dyn ContextView>;
}

type ChildType<'a> = dyn Context<'a>;

#[derive(Default)]
pub struct ChildManager<'a> {
    next_id: usize,
    children: Vec<&'a mut ChildType<'a>>,
}

struct ParentView {
    child_views: Vec<Box<dyn ContextView>>,
}

impl ContextView for ParentView {
    fn signal_when(&self, when: Time) -> crossbeam::channel::Receiver<Time> {
        let (tx, rx) = crossbeam::channel::bounded::<Time>(1);
        let individual_signals: Vec<crossbeam::channel::Receiver<Time>> = self
            .child_views
            .iter()
            .map(|child| child.signal_when(when))
            .collect();
        rayon::spawn_fifo(move || {
            let local_times = individual_signals
                .iter()
                .map(|signal| signal.recv().unwrap_or(Time::infinite()));
            let min_time = local_times.min().unwrap_or(Time::infinite());
            let _ = tx.send(min_time);
        });
        rx
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

    fn add_child(&mut self, child: &'a mut ChildType<'a>) {
        self.children.push(child);
    }

    fn for_each_child(&mut self, map_f: impl Fn(&mut ChildType<'a>) + Sync) {
        rayon::in_place_scope(|s: &rayon::Scope| {
            self.children.iter_mut().for_each(|child| {
                s.spawn(|_| map_f(*child));
            });
        });
    }

    fn view(&self) -> Box<dyn ContextView> {
        let child_views = self.children.iter().map(|child| child.view()).collect();
        Box::new(ParentView { child_views })
    }
}

pub trait ParentContext<'a>: Context<'a> {
    fn manager_mut(&mut self) -> &mut ChildManager<'a>;
    fn manager(&self) -> &ChildManager<'a>;
    fn new_child_id(&mut self) -> usize {
        self.manager_mut().new_child_id()
    }

    fn add_child(&mut self, child: &'a mut ChildType<'a>) {
        self.manager_mut().add_child(child);
    }
}

impl<'a, T: ParentContext<'a>> Context<'a> for T {
    fn init(&mut self) {
        self.manager_mut().for_each_child(|child| {
            child.init();
        })
    }

    fn run(&mut self) {
        self.manager_mut().for_each_child(|child| {
            child.run();
            child.cleanup();
        })
    }

    fn cleanup(&mut self) {}

    fn view(&self) -> Box<dyn ContextView> {
        self.manager().view()
    }
}
