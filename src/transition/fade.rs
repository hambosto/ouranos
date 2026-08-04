use super::animation::AnimationSequence;

pub(crate) struct Fade {
    seq: AnimationSequence,
    progress: f32,
}

impl Fade {
    pub(crate) fn new(duration: f32) -> Self {
        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), progress: 0.0 }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        self.progress = self.seq.now();
        let inv = 1.0 - self.progress;

        for (old, &new) in canvas.iter_mut().zip(target) {
            *old = (*old as f32 * inv + new as f32 * self.progress + 0.5) as u8;
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}
