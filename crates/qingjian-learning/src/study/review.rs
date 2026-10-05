//! 练习掌握与候选曝光分开记录；时间以日序号传入，便于固定时间回放。

use serde::{Deserialize, Serialize};

use super::Grade;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub favorite: bool,

    pub exposures: u64,

    pub wrong: u64,

    pub stage: usize,

    pub due_day: i64,

    pub introduced_day: Option<i64>,

    pub last_review_day: Option<i64>,

    pub mastered: bool,
}

impl ReviewRecord {
    pub fn grade(&mut self, grade: Grade, day: i64) {
        const INTERVALS: [i64; 5] = [1, 3, 7, 14, 30];
        self.introduced_day.get_or_insert(day);
        let repeated_today = self.last_review_day == Some(day);
        self.last_review_day = Some(day);
        match grade {
            Grade::Forgotten => {
                self.wrong = self.wrong.saturating_add(1);
                self.stage = 0;
                self.mastered = false;
                self.due_day = day + 1;
            }
            Grade::Remembered => {
                if repeated_today {
                    return;
                }
                self.mastered = false;
                self.due_day = day + INTERVALS[self.stage.min(4)];
                if !repeated_today {
                    self.stage = (self.stage + 1).min(4);
                }
            }
            Grade::Mastered => {
                self.mastered = true;
                self.due_day = day + 30;
            }
        }
    }
}
