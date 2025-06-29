use crate::app::Shared;

pub struct Scheduler<'a> {
    tasks: &'a mut [&'a mut dyn Tickable],
    shared: &'a mut Shared
}

impl<'a> Scheduler<'a> {
    pub fn new(tasks: &'a mut [&'a mut dyn Tickable], shared: &'a mut Shared) -> Self {
        Self {
            tasks,
            shared,
        }
    }

    pub fn run(&mut self, now: u64) {
        for task in self.tasks.iter_mut() {
            if now >= task.next_run_at() {
                task.tick(now, self.shared);
            }
        }
    }
}

pub trait Tickable {
    /// Returns the next time this task wants to be ticked (in ms)
    fn next_run_at(&self) -> u64;

    /// Called by the scheduler when `now >= next_run_at()`
    fn tick(&mut self, now: u64, shared: &mut Shared);
}