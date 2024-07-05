use std::{str::FromStr, usize};

use regex::{Error, Regex};

use crate::{
    error::{ATDomeError, ATDomeResult},
    status::Status,
};

const MAIN: &str = r"MAIN +[A-Z]+ +(\d+)";
const DROP: &str = r"DROP +[A-Z]+ +(\d+)";
const AUTO_SHUTDOWN: &str = r"\[(ON|OFF)\] +(\d+)";
const AZ_POS_MATCH: &str = r"(POSN|HOME) +(\d*\.?\d+)";
const MOVE_CODE: &str = r"(?:RL|RR|--) +(\d+)";
const AZ_HOMED: &str = r"Dome (not )?homed";
const ESTOP_ACTIVE: &str = r"Emergency Stop Active: +(\d)";
const SCB_LINK_OK: &str = r"Top Comm Link OK: +(\d)";
const HOME_AZIMUTH: &str = r"Home Azimuth: +(\d*\.?\d+)";
const HIGH_SPEED: &str = r"High Speed.+: +(\d*\.?\d+)";
const COAST: &str = r"Coast.+: +(\d*\.?\d+)";
const TOLERANCE: &str = r"Tolerance.+: +(\d*\.?\d+)";
const ENCODER_COUNTS_PER_360: &str = r"Encoder Counts per 360: +(\d+)";
const ENCODER_COUNTS: &str = r"Encoder Counts: +(\d+)";
const LAST_AZIMUTH_GOTO: &str = r"Last Azimuth GoTo: +(\d*\.?\d+)";
const AZIMUTH_MOVE_TIMEOUT: &str = r"Azimuth Move Timeout.+: +(\d*\.?\d+)";
const RAIN_SENSOR_ENABLED: &str = r"Rain-Snow enabled: +(\d)";
const CLOUD_SENSOR_ENABLED: &str = r"Cloud Sensor enabled: +(\d)";
const WATCHDOG_TIMER: &str = r"Watchdog Reset Time: +(\d*\.?\d+)";
const DROPOUT_TIMER: &str = r"Dropout Timer: +(\d*\.?\d+)";
const REVERSAL_DELAY: &str = r"Reverse Delay: +(\d*\.?\d+)";
const MAIN_DOOR_ENCODER_CLOSED: &str = r"Main Door Encoder Closed: +(\d+)";
const MAIN_DOOR_ENCODER_OPENED: &str = r"Main Door Encoder Opened: +(\d+)";
const DROPOUT_DOOR_ENCODER_CLOSED: &str = r"Dropout Encoder Closed: +(\d+)";
const DROPOUT_DOOR_ENCODER_OPENED: &str = r"Dropout Encoder Opened: +(\d+)";
const DOOR_MOVE_TIMEOUT: &str = r"Door Move Timeout.+: +(\d*\.?\d+)";

#[derive(Debug)]
pub struct StatusParser {
    pub main: Regex,
    pub drop: Regex,
    pub auto_shutdown: Regex,
    pub az_pos_match: Regex,
    pub move_code: Regex,
    pub az_homed: Regex,
    pub estop_active: Regex,
    pub scb_link_ok: Regex,
    pub home_azimuth: Regex,
    pub high_speed: Regex,
    pub coast: Regex,
    pub tolerance: Regex,
    pub encoder_counts_per_360: Regex,
    pub encoder_counts: Regex,
    pub last_azimuth_goto: Regex,
    pub azimuth_move_timeout: Regex,
    pub rain_sensor_enabled: Regex,
    pub cloud_sensor_enabled: Regex,
    pub watchdog_timer: Regex,
    pub dropout_timer: Regex,
    pub reversal_delay: Regex,
    pub main_door_encoder_closed: Regex,
    pub main_door_encoder_opened: Regex,
    pub dropout_door_encoder_closed: Regex,
    pub dropout_door_encoder_opened: Regex,
    pub door_move_timeout: Regex,
}

impl StatusParser {
    pub fn new() -> Result<StatusParser, Error> {
        let main = Regex::new(&MAIN)?;
        let drop = Regex::new(&DROP)?;
        let auto_shutdown = Regex::new(&AUTO_SHUTDOWN)?;
        let az_pos_match = Regex::new(&AZ_POS_MATCH)?;
        let move_code = Regex::new(&MOVE_CODE)?;
        let az_homed = Regex::new(&AZ_HOMED)?;
        let estop_active = Regex::new(&ESTOP_ACTIVE)?;
        let scb_link_ok = Regex::new(&SCB_LINK_OK)?;
        let home_azimuth = Regex::new(&HOME_AZIMUTH)?;
        let high_speed = Regex::new(&HIGH_SPEED)?;
        let coast = Regex::new(&COAST)?;
        let tolerance = Regex::new(&TOLERANCE)?;
        let encoder_counts_per_360 = Regex::new(&ENCODER_COUNTS_PER_360)?;
        let encoder_counts = Regex::new(&ENCODER_COUNTS)?;
        let last_azimuth_goto = Regex::new(&LAST_AZIMUTH_GOTO)?;
        let azimuth_move_timeout = Regex::new(&AZIMUTH_MOVE_TIMEOUT)?;
        let rain_sensor_enabled = Regex::new(&RAIN_SENSOR_ENABLED)?;
        let cloud_sensor_enabled = Regex::new(&CLOUD_SENSOR_ENABLED)?;
        let watchdog_timer = Regex::new(&WATCHDOG_TIMER)?;
        let dropout_timer = Regex::new(&DROPOUT_TIMER)?;
        let reversal_delay = Regex::new(&REVERSAL_DELAY)?;
        let main_door_encoder_closed = Regex::new(&MAIN_DOOR_ENCODER_CLOSED)?;
        let main_door_encoder_opened = Regex::new(&MAIN_DOOR_ENCODER_OPENED)?;
        let dropout_door_encoder_closed = Regex::new(&DROPOUT_DOOR_ENCODER_CLOSED)?;
        let dropout_door_encoder_opened = Regex::new(&DROPOUT_DOOR_ENCODER_OPENED)?;
        let door_move_timeout = Regex::new(&DOOR_MOVE_TIMEOUT)?;
        Ok(StatusParser {
            main,
            drop,
            auto_shutdown,
            az_pos_match,
            move_code,
            az_homed,
            estop_active,
            scb_link_ok,
            home_azimuth,
            high_speed,
            coast,
            tolerance,
            encoder_counts_per_360,
            encoder_counts,
            last_azimuth_goto,
            azimuth_move_timeout,
            rain_sensor_enabled,
            cloud_sensor_enabled,
            watchdog_timer,
            dropout_timer,
            reversal_delay,
            main_door_encoder_closed,
            main_door_encoder_opened,
            dropout_door_encoder_closed,
            dropout_door_encoder_opened,
            door_move_timeout,
        })
    }

    pub fn make_status(self, lines: &[&str]) -> ATDomeResult<Status> {
        let length = lines.len();
        if length != 27 && length != 28 {
            return Err(ATDomeError::new(&format!(
                "Got {length}; expected 26 or 28."
            )));
        }
        let main_door_pct: f32 = StatusParser::unwrap_capture(&lines[0], &self.main, 1)?;
        let dropout_door_pct: f32 = StatusParser::unwrap_capture(&lines[1], &self.drop, 1)?;
        let auto_shutdown_enabled: String =
            StatusParser::unwrap_capture(&lines[2], &self.auto_shutdown, 1)?;
        let auto_shutdown_enabled = auto_shutdown_enabled == "ON";
        let sensor_code: usize = StatusParser::unwrap_capture(&lines[2], &self.auto_shutdown, 2)?;
        let az_home_switch: String =
            StatusParser::unwrap_capture(&lines[3], &self.az_pos_match, 1)?;
        let az_home_switch = az_home_switch == "HOME";
        let az_pos: f32 = StatusParser::unwrap_capture(&lines[3], &self.az_pos_match, 2)?;
        let move_code: u8 = StatusParser::unwrap_capture(&lines[4], &self.move_code, 1)?;
        let homed = !StatusParser::has_group(&lines[5], &self.az_homed, 1)?;
        let estop_active: bool =
            StatusParser::unwrap_capture::<usize>(&lines[6], &self.estop_active, 1)? > 0;
        let scb_link_ok: bool =
            StatusParser::unwrap_capture::<usize>(&lines[7], &self.scb_link_ok, 1)? > 0;
        let home_azimuth: f32 = StatusParser::unwrap_capture(&lines[8], &self.home_azimuth, 1)?;
        let high_speed: f32 = StatusParser::unwrap_capture(&lines[9], &self.high_speed, 1)?;
        let coast: f32 = StatusParser::unwrap_capture(&lines[10], &self.coast, 1)?;
        let tolerance: f32 = StatusParser::unwrap_capture(&lines[11], &self.tolerance, 1)?;
        let encoder_counts_per_360: u64 =
            StatusParser::unwrap_capture(&lines[12], &self.encoder_counts_per_360, 1)?;
        let encoder_counts: u64 =
            StatusParser::unwrap_capture(&lines[13], &self.encoder_counts, 1)?;
        let last_azimuth_goto: f32 =
            StatusParser::unwrap_capture(&lines[14], &self.last_azimuth_goto, 1)?;
        let azimuth_move_timeout: f32 =
            StatusParser::unwrap_capture(&lines[15], &self.azimuth_move_timeout, 1)?;
        let rain_sensor_enabled: bool =
            StatusParser::unwrap_capture::<usize>(&lines[16], &self.rain_sensor_enabled, 1)? > 0;
        let cloud_sensor_enabled: bool =
            StatusParser::unwrap_capture::<usize>(&lines[17], &self.cloud_sensor_enabled, 1)? > 0;
        let watchdog_timer: f32 =
            StatusParser::unwrap_capture(&lines[18], &self.watchdog_timer, 1)?;
        let dropout_timer: f32 = StatusParser::unwrap_capture(&lines[19], &self.dropout_timer, 1)?;
        let reversal_delay: f32 =
            StatusParser::unwrap_capture(&lines[20], &self.reversal_delay, 1)?;
        let main_door_encoder_closed: u64 =
            StatusParser::unwrap_capture(&lines[21], &self.main_door_encoder_closed, 1)?;
        let main_door_encoder_opened: u64 =
            StatusParser::unwrap_capture(&lines[22], &self.main_door_encoder_opened, 1)?;
        let dropout_door_encoder_closed: u64 =
            StatusParser::unwrap_capture(&lines[23], &self.dropout_door_encoder_closed, 1)?;
        let dropout_door_encoder_opened: u64 =
            StatusParser::unwrap_capture(&lines[24], &self.dropout_door_encoder_opened, 1)?;
        let door_move_timeout: f32 =
            StatusParser::unwrap_capture(&lines[25], &self.door_move_timeout, 1)?;

        Ok(Status {
            main_door_pct,
            dropout_door_pct,
            auto_shutdown_enabled,
            sensor_code,
            az_home_switch,
            az_pos,
            move_code,
            homed,
            estop_active,
            scb_link_ok,
            home_azimuth,
            high_speed,
            coast,
            tolerance,
            encoder_counts_per_360,
            encoder_counts,
            last_azimuth_goto,
            azimuth_move_timeout,
            rain_sensor_enabled,
            cloud_sensor_enabled,
            watchdog_timer,
            dropout_timer,
            reversal_delay,
            main_door_encoder_closed,
            main_door_encoder_opened,
            dropout_door_encoder_closed,
            dropout_door_encoder_opened,
            door_move_timeout,
        })
    }

    fn unwrap_capture<T: FromStr>(
        line: &str,
        regex: &Regex,
        extract_group: usize,
    ) -> ATDomeResult<T> {
        if let Some(capture) = regex.captures(&line) {
            if let Some(group) = capture.get(extract_group) {
                if let Ok(value) = group.as_str().parse::<T>() {
                    Ok::<T, ATDomeError>(value)
                } else {
                    Err(ATDomeError::new(&format!(
                        "Cannot convert string to return type: {}",
                        group.as_str()
                    )))
                }
            } else {
                return Err(ATDomeError::new(&format!(
                    "Could not find expected group 1 in captured group: {capture:?}"
                )));
            }
        } else {
            return Err(ATDomeError::new(&format!("Failed to match {line}")));
        }
    }

    fn has_group(line: &str, regex: &Regex, extract_group: usize) -> ATDomeResult<bool> {
        if let Some(capture) = regex.captures(&line) {
            return Ok(capture.get(extract_group).is_some());
        } else {
            return Err(ATDomeError::new(&format!("Failed to match {line}")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_parser_new() {
        let status_parser = StatusParser::new();

        assert!(status_parser.is_ok())
    }

    #[test]
    fn test_make_status_wrong_input() {
        let lines: [&str; 4] = ["this", "is", "a", "test"];

        let status_parser = StatusParser::new().unwrap();

        let status = status_parser.make_status(&lines);

        assert!(!status.is_ok());
    }

    #[test]
    fn test_make_status() {
        let lines: [&str; 27] = [
            "MAIN SHUT 000",
            "DROP SHUT 000",
            "[OFF] 00",
            "POSN 262.91",
            "-- 000",
            "Dome homed",
            "Emergency Stop Active: 0",
            "Top Comm Link OK: 1",
            "Home Azimuth:  0.00",
            "High Speed (degrees): 5.00",
            "Coast (degrees): 0.50",
            "Tolerance (degrees): 1.00",
            "Encoder Counts per 360: 4018143232",
            "Encoder Counts: 10970978722",
            "Last Azimuth GoTo:  10.00",
            "Azimuth Move Timeout (secs): 120",
            "Rain-Snow enabled: 0",
            "Cloud Sensor enabled: 1",
            "Watchdog Reset Time: 600",
            "Dropout Timer: 5",
            "Reverse Delay: 5",
            "Main Door Encoder Closed: 118551649796",
            "Main Door Encoder Opened: 8360300777",
            "Dropout Encoder Closed: 5669713343",
            "Dropout Encoder Opened: 5710964429",
            "Door Move Timeout (secs): 360",
            "Dome has been homed: False",
        ];

        let status_parser = StatusParser::new().unwrap();

        let status = status_parser.make_status(&lines).unwrap();

        println!("{status:?}");
        assert!(!status.auto_shutdown_enabled);
        assert!(!status.az_home_switch);
        assert_eq!(status.az_pos, 262.91);
        assert_eq!(status.move_code, 0);
        assert_eq!(status.homed, true);
        assert_eq!(status.estop_active, false);
        assert_eq!(status.scb_link_ok, true);
        assert_eq!(status.home_azimuth, 0.0);
        assert_eq!(status.high_speed, 5.0);
        assert_eq!(status.coast, 0.5);
        assert_eq!(status.tolerance, 1.0);
        assert_eq!(status.encoder_counts_per_360, 4018143232);
        assert_eq!(status.encoder_counts, 10970978722);
        assert_eq!(status.last_azimuth_goto, 10.0);
        assert_eq!(status.azimuth_move_timeout, 120.0);
        assert_eq!(status.rain_sensor_enabled, false);
        assert_eq!(status.cloud_sensor_enabled, true);
        assert_eq!(status.watchdog_timer, 600.0);
        assert_eq!(status.dropout_timer, 5.0);
        assert_eq!(status.reversal_delay, 5.0);
        assert_eq!(status.main_door_encoder_closed, 118551649796);
        assert_eq!(status.main_door_encoder_opened, 8360300777);
        assert_eq!(status.dropout_door_encoder_closed, 5669713343);
        assert_eq!(status.dropout_door_encoder_opened, 5710964429);
        assert_eq!(status.door_move_timeout, 360.0);
    }
}
