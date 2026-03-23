// SPDX-License-Identifier: MIT
//! Cron task definition.

use chrono::{DateTime, Utc};
use cron::Schedule;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CronTask {
    pub id: Uuid,
    pub name: String,
    pub schedule_expr: String,
    pub prompt: String,
    schedule: Schedule,
    last_run: Option<DateTime<Utc>>,
}

impl CronTask {
    pub fn new(name: &str, expr: &str, prompt: &str) -> Result<Self, cron::error::Error> {
        let schedule = Schedule::from_str(expr)?;
        Ok(Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            schedule_expr: expr.to_string(),
            prompt: prompt.to_string(),
            schedule,
            last_run: None,
        })
    }

    pub fn next_run(&self) -> Option<DateTime<Utc>> {
        self.schedule.upcoming(Utc).next()
    }

    pub fn advance_schedule(&mut self) {
        self.last_run = Some(Utc::now());
    }
}
