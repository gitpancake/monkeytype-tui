use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Time(u32),
    Words(u32),
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub test_duration: f64,
    pub correct_chars: usize,
    pub incorrect_chars: usize,
    pub extra_chars: usize,
    pub missed_chars: usize,
    pub mode: Mode,
    /// Per-second WPM samples for chart data + consistency.
    pub wpm_samples: Vec<f64>,
    pub raw_samples: Vec<f64>,
    pub err_samples: Vec<u32>,
    pub language: String,
}

pub struct Test {
    pub words: Vec<String>,
    /// One Vec<Option<char>> per word — typed chars (None = pending).
    pub typed: Vec<Vec<Option<char>>>,
    pub current_word: usize,
    pub current_char: usize,
    pub started: Option<Instant>,
    pub mode: Mode,
    /// Per-second snapshots: (correct_chars_so_far, total_chars_so_far, errors_so_far).
    pub samples: Vec<(usize, usize, u32)>,
    pub last_sample_sec: u32,
    pub errors_total: u32,
}

impl Test {
    pub fn new(mode: Mode, words: Vec<String>) -> Self {
        let typed = words.iter().map(|_| Vec::new()).collect();
        Self {
            words,
            typed,
            current_word: 0,
            current_char: 0,
            started: None,
            mode,
            samples: Vec::new(),
            last_sample_sec: 0,
            errors_total: 0,
        }
    }

    pub fn elapsed(&self) -> f64 {
        self.started
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn time_left(&self) -> Option<f64> {
        match self.mode {
            Mode::Time(t) => Some((t as f64 - self.elapsed()).max(0.0)),
            Mode::Words(_) => None,
        }
    }

    pub fn finished(&self) -> bool {
        match self.mode {
            Mode::Time(t) => self.elapsed() >= t as f64,
            Mode::Words(n) => self.current_word >= n as usize,
        }
    }

    pub fn type_char(&mut self, c: char) {
        if self.started.is_none() {
            self.started = Some(Instant::now());
        }
        if c == ' ' {
            // advance word
            if self.current_word + 1 < self.words.len() {
                // count missed chars on this word
                let target = &self.words[self.current_word];
                let typed = &self.typed[self.current_word];
                if typed.len() < target.len() {
                    self.errors_total += (target.len() - typed.len()) as u32;
                }
                self.current_word += 1;
                self.current_char = 0;
            }
        } else if c == '\u{8}' {
            // backspace
            if self.current_char > 0 {
                self.current_char -= 1;
                self.typed[self.current_word].pop();
            }
        } else {
            let target = self.words[self.current_word].chars().nth(self.current_char);
            let correct = target.map(|t| t == c).unwrap_or(false);
            if !correct {
                self.errors_total += 1;
            }
            self.typed[self.current_word].push(Some(c));
            self.current_char += 1;
        }
        self.maybe_sample();
    }

    fn maybe_sample(&mut self) {
        let sec = self.elapsed() as u32;
        if sec > self.last_sample_sec {
            let (correct, total) = self.char_counts();
            self.samples.push((correct, total, self.errors_total));
            self.last_sample_sec = sec;
        }
    }

    /// (correct, total_typed)
    pub fn char_counts(&self) -> (usize, usize) {
        let mut correct = 0;
        let mut total = 0;
        for (i, typed) in self.typed.iter().enumerate() {
            let target: Vec<char> = self.words[i].chars().collect();
            for (j, t) in typed.iter().enumerate() {
                total += 1;
                if let Some(tc) = t {
                    if target.get(j).copied() == Some(*tc) {
                        correct += 1;
                    }
                }
            }
        }
        (correct, total)
    }

    pub fn finalize(self) -> TestResult {
        let duration = self.elapsed().max(0.001);
        let (correct, total) = self.char_counts();
        let incorrect = total - correct;
        // missed/extra: simple approximation
        let mut missed = 0;
        let mut extra = 0;
        for (i, typed) in self.typed.iter().enumerate() {
            let target_len = self.words[i].chars().count();
            if i < self.current_word {
                if typed.len() < target_len {
                    missed += target_len - typed.len();
                } else if typed.len() > target_len {
                    extra += typed.len() - target_len;
                }
            }
        }

        // include spaces between completed words as correct chars (monkeytype counts them)
        let space_chars = self.current_word.min(self.words.len().saturating_sub(1));
        let correct_with_space = correct + space_chars;
        let total_with_space = total + space_chars;

        let minutes = duration / 60.0;
        let wpm = (correct_with_space as f64 / 5.0) / minutes;
        let raw_wpm = (total_with_space as f64 / 5.0) / minutes;
        let accuracy = if total_with_space == 0 {
            0.0
        } else {
            (correct_with_space as f64 / total_with_space as f64) * 100.0
        };

        // per-second WPM samples for chartData
        let mut wpm_samples = Vec::with_capacity(self.samples.len());
        let mut raw_samples = Vec::with_capacity(self.samples.len());
        let mut err_samples = Vec::with_capacity(self.samples.len());
        let mut prev_correct = 0usize;
        let mut prev_total = 0usize;
        let mut prev_err = 0u32;
        for (i, (c, t, e)) in self.samples.iter().enumerate() {
            let _sec = (i + 1) as f64;
            let dc = c.saturating_sub(prev_correct) as f64;
            let dt = t.saturating_sub(prev_total) as f64;
            let de = e.saturating_sub(prev_err);
            // instantaneous WPM for the 1s window: chars/5 * 60
            wpm_samples.push((dc / 5.0) * 60.0);
            raw_samples.push((dt / 5.0) * 60.0);
            err_samples.push(de);
            prev_correct = *c;
            prev_total = *t;
            prev_err = *e;
        }
        let consistency = consistency_pct(&wpm_samples);

        TestResult {
            wpm,
            raw_wpm,
            accuracy,
            consistency,
            test_duration: duration,
            correct_chars: correct,
            incorrect_chars: incorrect,
            extra_chars: extra,
            missed_chars: missed,
            mode: self.mode,
            wpm_samples,
            raw_samples,
            err_samples,
            language: "english".into(),
        }
    }
}

/// Monkeytype consistency: 100 * (1 - stddev/mean) approximation via coefficient of variation,
/// mapped through Kovalchik scaling. Simplified: cv → (1 - cv) * 100, floored at 0.
fn consistency_pct(samples: &[f64]) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;
    let sd = var.sqrt();
    let cv = sd / mean;
    ((1.0 - cv).max(0.0) * 100.0).min(100.0)
}
