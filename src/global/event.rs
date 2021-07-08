#[derive(Debug, Clone)]
pub enum Event {
    Start,
    Stop,
    Instance(InstanceEvent),
}

impl Event {
    pub fn instance(id: i32, kind: InstanceEventKind) -> Self {
        Self::Instance(InstanceEvent { id, kind })
    }
}

#[derive(Debug, Clone)]
pub struct InstanceEvent {
    pub id: i32,
    pub kind: InstanceEventKind,
}

#[derive(Debug, Clone)]
pub enum InstanceEventKind {
    Start,
    Stop,
    Activate,
    Deactivate,
}
