use crate::utils::frame_log::FrameLog;
use crate::{Egregoria, ParCommandBuffer};
use common::History;
use legion::systems::ParallelRunnable;
use ordered_float::OrderedFloat;
use std::time::Instant;

#[derive(Default)]
pub struct SeqSchedule {
    systems: Vec<(Box<dyn ParallelRunnable>, History)>,
}

impl SeqSchedule {
    pub fn add_system(&mut self, s: Box<dyn ParallelRunnable>) -> &mut Self {
        self.systems.push((s, History::new(600)));
        self
    }

    pub fn execute(&mut self, goria: &mut Egregoria) {
        let mut sys_times = vec![];
        for (sys, h) in &mut self.systems {
            let world = &mut goria.world;
            let res = &mut goria.resources;
            let start = Instant::now();

            sys.prepare(world);
            sys.run(world, res);

            if let Some(cb) = sys.command_buffer_mut(world.id()) {
                cb.flush(world, res);
            }
            ParCommandBuffer::apply(goria);

            let elapsed = start.elapsed();

            h.add_value(elapsed.as_secs_f32());
            sys_times.push((sys.name().unwrap(), h.avg()));
        }

        sys_times.sort_unstable_by_key(|(_, t)| OrderedFloat(-*t));

        for (name, t) in sys_times {
            let s = format!("system {} took {:.2}ms", name, t * 1000.0);
            goria.read::<FrameLog>().log_frame(s);
        }
    }
}
