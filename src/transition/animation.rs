pub(crate) struct AnimationSequence {
    start_val: f32,
    end_val: f32,
    start_time: f64,
    end_time: f64,
    time: f64,
}

impl AnimationSequence {
    pub(crate) fn new(duration: f32, start_val: f32, end_val: f32, start_time: f64) -> Self {
        let end_time = start_time + duration as f64;
        Self { start_val, end_val, start_time, end_time, time: 0.0 }
    }

    pub(crate) fn now(&self) -> f32 {
        let duration = self.end_time - self.start_time;
        let t = if duration <= 0.0 { 1.0 } else { ((self.time - self.start_time) / duration).clamp(0.0, 1.0) as f32 };
        let eased = if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 };
        (self.end_val - self.start_val).mul_add(eased, self.start_val)
    }

    pub(crate) fn advance_to(&mut self, timestamp: f64) -> f64 {
        self.time = timestamp.clamp(self.start_time, self.end_time);
        timestamp - self.time
    }

    pub(crate) fn finished(&self) -> bool {
        self.time >= self.end_time
    }
}
