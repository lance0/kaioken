pub struct Flavor {
    pub serious: bool,
}

impl Flavor {
    pub fn new(serious: bool) -> Self {
        Self { serious }
    }

    pub fn power_rank(&self, rps: f64) -> &'static str {
        if self.serious {
            return "";
        }

        match rps as u64 {
            0..=100 => "Farmer",
            101..=500 => "Krillin",
            501..=1_000 => "Piccolo",
            1_001..=5_000 => "Vegeta",
            5_001..=9_000 => "Goku",
            _ => "OVER 9000",
        }
    }

    pub fn status_initializing(&self) -> &'static str {
        if self.serious {
            "Initializing..."
        } else {
            "Powering up..."
        }
    }

    pub fn status_running(&self, concurrency: u32) -> String {
        if self.serious {
            format!("Running ({} workers)", concurrency)
        } else {
            format!("KAIOKEN x{}", concurrency)
        }
    }

    #[allow(dead_code)]
    pub fn status_error_high(&self) -> &'static str {
        if self.serious {
            "High error rate!"
        } else {
            "Senzu needed!"
        }
    }

    pub fn status_cancelled(&self) -> &'static str {
        if self.serious { "Cancelled" } else { "K.O." }
    }

    pub fn status_completed(&self) -> &'static str {
        if self.serious {
            "Completed"
        } else {
            "Victory!"
        }
    }

    pub fn title(&self) -> &'static str {
        if self.serious { "Load Test" } else { "KAIOKEN" }
    }

    pub fn power_level_title(&self) -> &'static str {
        if self.serious {
            "Throughput"
        } else {
            "POWER LEVEL"
        }
    }

    #[allow(dead_code)]
    pub fn improvement_indicator(&self) -> &'static str {
        if self.serious { "IMPROVED" } else { "POWER UP" }
    }

    #[allow(dead_code)]
    pub fn regression_indicator(&self) -> &'static str {
        if self.serious { "REGRESSED" } else { "DRAIN" }
    }
}
