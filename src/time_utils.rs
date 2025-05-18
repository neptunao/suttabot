use anyhow::{anyhow, Result};
use chrono::NaiveTime;
use regex::Regex;

const DEFAULT_TIMEZONE_OFFSET_MINUTES: i32 = 3 * 60; // UTC+3 (Moscow)

pub fn parse_time_to_utc_minutes(time_input: &str) -> Result<i32> {
    // Regex to capture HH:MM and optional timezone like UTC+HH:MM or UTC-HH:MM
    // Regex101 link: https://regex101.com/r/Dk1zI5/1
    let re = Regex::new(r"^(?P<hours>\d{1,2}):(?P<minutes>\d{2})(?:\s+UTC(?P<tz_sign>[+-])(?P<tz_hours>\d{1,2}):(?P<tz_minutes>\d{2}))?$")?;

    if let Some(caps) = re.captures(time_input.trim()) {
        let hours: u32 = caps
            .name("hours")
            .unwrap()
            .as_str()
            .parse()
            .map_err(|_| anyhow!("Invalid hours"))?;
        let minutes: u32 = caps
            .name("minutes")
            .unwrap()
            .as_str()
            .parse()
            .map_err(|_| anyhow!("Invalid minutes"))?;

        if hours > 23 || minutes > 59 {
            return Err(anyhow!("Invalid time value: {} hours, {} minutes", hours, minutes));
        }

        let local_time_total_minutes = (hours * 60 + minutes) as i32;

        let timezone_offset_minutes = if caps.name("tz_sign").is_some() {
            let sign_str = caps.name("tz_sign").unwrap().as_str();
            let tz_hours: i32 = caps
                .name("tz_hours")
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| anyhow!("Invalid timezone hours"))?;
            let tz_minutes: i32 = caps
                .name("tz_minutes")
                .unwrap()
                .as_str()
                .parse()
                .map_err(|_| anyhow!("Invalid timezone minutes"))?;

            if tz_hours > 14 || tz_minutes > 59 { // Max offset is UTC+14:00, min is UTC-12:00
                return Err(anyhow!("Invalid timezone offset value: {}h {}m", tz_hours, tz_minutes));
            }

            let total_offset = tz_hours * 60 + tz_minutes;
            if sign_str == "-" {
                -total_offset
            } else {
                total_offset
            }
        } else {
            DEFAULT_TIMEZONE_OFFSET_MINUTES
        };

        // Convert local time to UTC minutes
        // (local_minutes_from_midnight - offset_of_local_timezone_from_utc + minutes_in_day) % minutes_in_day
        let utc_total_minutes = (local_time_total_minutes - timezone_offset_minutes + 1440) % 1440;
        Ok(utc_total_minutes)
    } else {
        Err(anyhow!(
            "Invalid time format: '{}'. Expected HH:MM or HH:MM UTC+/-HH:MM", time_input
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_utc_conversion() {
        assert_eq!(parse_time_to_utc_minutes("8:00").unwrap(), 300); // 08:00 MSK (UTC+3) -> 05:00 UTC
        assert_eq!(parse_time_to_utc_minutes("08:00").unwrap(), 300);
        assert_eq!(parse_time_to_utc_minutes("14:33").unwrap(), 693); // 14:33 MSK (UTC+3) -> 11:33 UTC
        assert_eq!(
            parse_time_to_utc_minutes("14:33 UTC+02:00").unwrap(),
            753
        ); // 14:33 UTC+2 -> 12:33 UTC
        assert_eq!(parse_time_to_utc_minutes("10:00 UTC-01:00").unwrap(), 660); // 10:00 UTC-1 -> 11:00 UTC
        assert_eq!(parse_time_to_utc_minutes("00:00 UTC+00:00").unwrap(), 0);
        assert_eq!(parse_time_to_utc_minutes("23:59 UTC+00:00").unwrap(), 1439);
        assert_eq!(parse_time_to_utc_minutes("01:00 UTC-03:00").unwrap(), 240); // 01:00 UTC-3 -> 04:00 UTC

        // Test edge cases for modulo arithmetic
        assert_eq!(parse_time_to_utc_minutes("01:00 UTC+03:00").unwrap(), 1320); // 01:00 MSK -> 22:00 UTC (previous day)
        assert_eq!(parse_time_to_utc_minutes("01:00").unwrap(), 1320); // 01:00 MSK -> 22:00 UTC (previous day)

        // Test invalid inputs
        assert!(parse_time_to_utc_minutes("24:00").is_err());
        assert!(parse_time_to_utc_minutes("10:60").is_err());
        assert!(parse_time_to_utc_minutes("10:00 UTC+15:00").is_err());
        assert!(parse_time_to_utc_minutes("10:00 UTC-13:00").is_err());
        assert!(parse_time_to_utc_minutes("invalid").is_err());
        assert!(parse_time_to_utc_minutes("10:00 AM").is_err());
         assert!(parse_time_to_utc_minutes("10:00UTC+01:00").is_err()); // Requires space
    }
}
