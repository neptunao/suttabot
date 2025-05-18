use crate::db::DbService;
use crate::helpers::list_files;
use crate::sender::send_daily_message;
use anyhow::Result;
use chrono::{Timelike, Utc};
use log::{error, info, warn};
use std::collections::HashMap;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use teloxide::Bot;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Instant};

pub struct ScheduleManager {
    db: Arc<DbService>,
    // schedule maps UTC minute of day (0-1439) to a list of chat_ids
    schedule: Arc<RwLock<HashMap<i32, Vec<i64>>>>,
}

impl ScheduleManager {
    pub fn new(db: Arc<DbService>) -> Self {
        Self {
            db,
            schedule: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn refresh(&self) -> Result<()> {
        info!("Refreshing scheduler cache...");
        let active_schedules = self.db.get_all_active_schedules().await?;
        let mut new_schedule_map: HashMap<i32, Vec<i64>> = HashMap::new();

        for (chat_id, utc_minute) in active_schedules {
            new_schedule_map
                .entry(utc_minute)
                .or_default()
                .push(chat_id);
        }

        let mut schedule_guard = self.schedule.write().await;
        *schedule_guard = new_schedule_map;
        info!("Scheduler cache refreshed with {} active minutes.", schedule_guard.len());
        Ok(())
    }

    // Public getter for the schedule data Arc
    pub fn get_schedule_data_arc(&self) -> Arc<RwLock<HashMap<i32, Vec<i64>>>> {
        self.schedule.clone()
    }
}

// This task periodically calls ScheduleManager.refresh()
pub async fn run_schedule_manager_refresh_loop(schedule_manager: Arc<ScheduleManager>) {
    let mut refresh_interval = interval(StdDuration::from_secs(3600)); // Refresh every hour
    loop {
        refresh_interval.tick().await;
        if let Err(e) = schedule_manager.refresh().await {
            error!("Failed to refresh schedule manager: {}", e);
        }
    }
}

// This is the main scheduler loop that checks every minute.
pub async fn scheduler_loop(
    bot: Bot,
    schedule_data: Arc<RwLock<HashMap<i32, Vec<i64>>>>, // UTC minute -> Vec<chat_id>
    data_dir: PathBuf,
    sutta_files: Arc<Vec<DirEntry>>,
) {
    info!("Scheduler loop starting. Aligning to next minute.");

    // Align to the start of the next minute
    let now_instant = Instant::now();
    let now_utc = Utc::now();
    let seconds_into_current_minute = now_utc.second() as u64;
    let nanos_into_current_second = now_utc.nanosecond() as u64;
    let alignment_delay_ns = (60 - seconds_into_current_minute) * 1_000_000_000 - nanos_into_current_second;

    if alignment_delay_ns > 0 && alignment_delay_ns < 60_000_000_000 {
        sleep(StdDuration::from_nanos(alignment_delay_ns)).await;
    }
    info!("Scheduler aligned. Starting per-minute checks.");

    let mut minute_ticker = interval(StdDuration::from_secs(60));
    minute_ticker.tick().await; // Consume the immediate tick

    loop {
        minute_ticker.tick().await;
        let tick_start_instant = Instant::now();
        let current_utc = Utc::now();
        let current_minute_utc = (current_utc.hour() * 60 + current_utc.minute()) as i32;

        // info!("Scheduler tick: Minute {}", current_minute_utc); // For debugging

        let schedule_guard = schedule_data.read().await;
        if let Some(chat_ids_to_notify) = schedule_guard.get(&current_minute_utc) {
            if !chat_ids_to_notify.is_empty() {
                info!(
                    "Found {} users to notify at minute {}",
                    chat_ids_to_notify.len(),
                    current_minute_utc
                );
            }
            for &chat_id in chat_ids_to_notify {
                let bot_clone = bot.clone();
                let files_clone = sutta_files.clone(); // Arc clone
                tokio::spawn(async move {
                    // Note: send_daily_message expects &[DirEntry].
                    // If sutta_files is Arc<Vec<DirEntry>>, we need to pass &files_clone
                    match send_daily_message(&bot_clone, chat_id, &files_clone).await {
                        Ok(_) => info!("Successfully sent scheduled sutta to chat_id: {}", chat_id),
                        Err(e) => warn!(
                            "Failed to send scheduled sutta to chat_id: {}. Error: {:?}",
                            chat_id,
                            e
                        ),
                    }
                });
            }
        }

        // Optional: Log processing time for the tick if it's useful for monitoring
        // let processing_duration = tick_start_instant.elapsed();
        // if processing_duration > StdDuration::from_secs(1) { // Log if processing takes more than a second
        //     warn!("Scheduler tick processing took longer than 1 second: {:?}", processing_duration);
        // }
    }
}
