use std::time::{Duration, Instant};

use log::debug;

use ffmpeg_next::Rational;

const CADENCE_SAMPLE_COUNT: usize = 30;

#[derive(Default)]
pub struct FrameCadenceSampler {
    timestamps: Vec<i64>,
    wall_timestamps: Vec<i64>,
    wall_origin: Option<Instant>,
}

impl FrameCadenceSampler {
    pub fn new() -> Self {
        Self {
            timestamps: Vec::with_capacity(CADENCE_SAMPLE_COUNT),
            wall_timestamps: Vec::with_capacity(CADENCE_SAMPLE_COUNT),
            wall_origin: None,
        }
    }

    pub fn add_timestamp(&mut self, timestamp_ns: i64) {
        if self.timestamps.len() < CADENCE_SAMPLE_COUNT {
            self.timestamps.push(timestamp_ns);
        }
    }

    pub fn add_wallclock(&mut self, timestamp: Instant) {
        let origin = *self.wall_origin.get_or_insert(timestamp);
        let timestamp_ns = timestamp
            .saturating_duration_since(origin)
            .as_nanos()
            .try_into()
            .unwrap_or(i64::MAX);
        if self.wall_timestamps.len() < CADENCE_SAMPLE_COUNT {
            self.wall_timestamps.push(timestamp_ns);
        }
    }

    pub fn framerate(&self) -> Option<Rational> {
        if self.timestamps.len() < 2 {
            return None;
        }

        average_framerate(&self.timestamps).or_else(|| average_framerate(&self.wall_timestamps))
    }

    pub fn is_ready(&self) -> bool {
        self.timestamps.len() == CADENCE_SAMPLE_COUNT
    }

    pub fn sample_count(&self) -> usize {
        self.timestamps.len()
    }

    pub fn used_wallclock(&self) -> bool {
        average_framerate(&self.timestamps).is_none()
            && average_framerate(&self.wall_timestamps).is_some()
    }
}

fn average_framerate(timestamps: &[i64]) -> Option<Rational> {
    if timestamps.len() < 2 {
        return None;
    }
    let intervals: Vec<_> = timestamps
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect();
    if intervals.iter().any(|interval| *interval <= 0) {
        return None;
    }

    let total_ns: i64 = intervals.iter().sum();
    let numerator = 1_000_000_000_i64 * intervals.len() as i64;
    let divisor = gcd(numerator, total_ns);
    Some(Rational::new(
        (numerator / divisor).try_into().ok()?,
        (total_ns / divisor).try_into().ok()?,
    ))
}

fn gcd(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left.abs()
}

pub struct FpsLimit<T> {
    min_dt: Duration,
    on_deck: Option<(Duration, T)>,
    next_target_time: Option<Duration>,
}

// fps limit for VRR is pretty tricky. We can't just discard frames with close timestamps, because imagine the situation
// where we get the following stream of timestamps (in ms)
// 0, 16, 17, 10000
// we obviously want to drop the 16, not the 17, because that 17 is displayed for a very long time.
// so, basically, we need to add a frame of latency and buffer a frame to know if we should skip a frame
impl<T> FpsLimit<T> {
    pub fn new(max_fps: f64) -> Self {
        assert_ne!(max_fps, 0.);
        Self {
            min_dt: Duration::from_secs_f64(1. / max_fps),
            on_deck: None,
            next_target_time: None,
        }
    }

    pub fn on_new_frame(&mut self, f: T, ts: Duration) -> Option<T> {
        // always send the first frame, could be a long gap after.
        if self.next_target_time.is_none() {
            self.next_target_time = Some(ts + self.min_dt);
            return Some(f);
        }

        // don't have enough info to make a decision, hold on...
        if self.on_deck.is_none() {
            self.on_deck = Some((ts, f));
            return None;
        }

        let (old_ts, old_t) = self.on_deck.take().unwrap();
        let next_target_time = self.next_target_time.unwrap();
        self.on_deck = Some((ts, f));

        if ts < next_target_time {
            // drop
            debug!("--max-fps dropping frame with ts {old_ts:?}");

            None
        } else {
            debug!("--max-fps including frame with ts {old_ts:?}");

            // max to handle skips better
            self.next_target_time = Some(next_target_time.max(old_ts) + self.min_dt);
            Some(old_t)
        }
    }

    pub fn flush(&mut self) -> Option<T> {
        self.on_deck.take().map(|(_, t)| t)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::fps_limit::FpsLimit;

    #[test]
    fn basic() {
        let mut l = FpsLimit::<u32>::new(1.);
        let s = Duration::from_secs_f32;

        let out_frames: Vec<_> = [
            l.on_new_frame(0, s(0.)),
            l.on_new_frame(1, s(0.5)),
            l.on_new_frame(2, s(1.1)),
            l.on_new_frame(3, s(1.2)),
            l.on_new_frame(4, s(1.3)),
            l.on_new_frame(5, s(5.)),
            l.flush(),
        ]
        .into_iter()
        .flatten()
        .collect();

        assert_eq!(out_frames, [0, 1, 4, 5])
    }

    #[test]
    fn synthetic_120hz() {
        let mut l = FpsLimit::<u32>::new(30.);

        let mut acc = vec![];
        for i in 0..120 {
            if let Some(r) = l.on_new_frame(i, Duration::from_micros((i * 1_000_000 / 120) as u64))
            {
                acc.push(r);
            }
        }

        if let Some(r) = l.flush() {
            acc.push(r);
        }

        let ct = acc.len();
        assert!((28..32).contains(&ct), "ct={ct} acc={acc:?}");
    }

    #[test]
    fn large_skip() {
        let mut l = FpsLimit::<u32>::new(1.);
        let s = Duration::from_secs_f32;

        let out_frames: Vec<_> = [
            l.on_new_frame(0, s(0.)),
            l.on_new_frame(1, s(0.5)),
            l.on_new_frame(2, s(10.0)),
            l.on_new_frame(3, s(10.1)),
            l.on_new_frame(4, s(10.2)),
            l.on_new_frame(5, s(10.3)),
            l.flush(),
        ]
        .into_iter()
        .flatten()
        .collect();

        assert_eq!(out_frames, [0, 1, 2, 5])
    }
}

#[cfg(test)]
mod cadence_test {
    use std::time::{Duration, Instant};

    use super::FrameCadenceSampler;

    #[test]
    fn measures_synthetic_60hz_cadence() {
        let mut sampler = FrameCadenceSampler::new();
        for frame in 0..30 {
            sampler.add_timestamp(frame * 16_666_667);
        }

        let fps = sampler.framerate().unwrap();
        let measured = f64::from(fps.0) / f64::from(fps.1);
        assert!((59.9..60.1).contains(&measured), "fps={measured}");
        assert!(sampler.is_ready());
    }

    #[test]
    fn too_few_samples_have_no_measurement() {
        let mut sampler = FrameCadenceSampler::new();
        sampler.add_timestamp(0);
        assert_eq!(sampler.framerate(), None);
        assert_eq!(sampler.sample_count(), 1);
    }

    #[test]
    fn degenerate_intervals_have_no_measurement() {
        let mut sampler = FrameCadenceSampler::new();
        for _ in 0..30 {
            sampler.add_timestamp(0);
        }
        assert_eq!(sampler.framerate(), None);
    }

    #[test]
    fn uses_wallclock_when_presentation_timestamps_are_degenerate() {
        let mut sampler = FrameCadenceSampler::new();
        let start = Instant::now();
        for frame in 0..30 {
            sampler.add_timestamp(0);
            sampler.add_wallclock(start + Duration::from_millis(frame * 16));
        }

        let fps = sampler.framerate().unwrap();
        let measured = f64::from(fps.0) / f64::from(fps.1);
        assert!((62.0..63.0).contains(&measured), "fps={measured}");
        assert!(sampler.used_wallclock());
    }
}
