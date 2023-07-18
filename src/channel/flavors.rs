#[derive(Clone, Copy, Debug)]
pub enum ChannelFlavor {
    Unknown,
    Acyclic,
    Cyclic,
}

// In order to make different flavors and flavor inference work,
// We want there to be a Sender<T> and Receiver<T> object that the
// contexts themselves work with, with underlying Flavor implementations.
// This way, we can swap the implementations without the contexts
// knowing what happened.
