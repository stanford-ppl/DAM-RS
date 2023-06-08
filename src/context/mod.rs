use rayon::prelude::*;

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

impl<'a> ChildManager<'a> {
    fn new_child_id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id - 1
    }

    fn add_child(&mut self, child: &'a mut ChildType<'a>) {
        self.children.push(child);
    }

    fn for_each_child(&mut self, map_f: impl Fn(&mut ChildType<'a>) + Sync) {
        rayon::scope(|s: &rayon::Scope| {
            self.children.iter_mut().for_each(|child| {
                s.spawn(|_| map_f(*child));
            });
        });
    }
}

pub trait ParentContext<'a>: Context<'a> {
    fn manager(&mut self) -> &mut ChildManager<'a>;
    fn new_child_id(&mut self) -> usize {
        self.manager().new_child_id()
    }

    fn add_child(&mut self, child: &'a mut ChildType<'a>) {
        self.manager().add_child(child);
    }
}

impl<'a, T: ParentContext<'a>> Context<'a> for T {
    fn init(&mut self) {
        self.manager().for_each_child(|child| {
            child.init();
        })
    }

    fn run(&mut self) {
        self.manager().for_each_child(|child| {
            child.run();
            child.cleanup();
        })
    }

    fn cleanup(&mut self) {}

    fn view(&self) -> Box<dyn ContextView> {
        todo!()
    }
}
